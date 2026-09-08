use std::cell::OnceCell;
use std::ffi::c_void;
use std::ops::Deref;
use std::sync::OnceLock;

use mlua::{FromLua, Function, IntoLua, LightUserData, Lua, ObjectLike, Result, Table, Value};

use crate::{Converters, Fetches, Http, HttpMessage, LogLevel};

const NATIVE_TXN_SLOT_API_MAGIC: u32 = 0x4854_5331;
const NATIVE_TXN_SLOT_API_VERSION: u16 = 1;
const NATIVE_TXN_SLOT_OWNER_DEFAULT: u64 = 1;
const NATIVE_TXN_SLOT_OK: i32 = 0;
const NATIVE_TXN_SLOT_OCCUPIED: i32 = -2;

#[cfg(unix)]
type NativeGetTxnSlot = unsafe extern "C" fn(*const c_void, u64, *mut *mut c_void) -> i32;
#[cfg(unix)]
type NativeSetTxnSlot = unsafe extern "C" fn(
    *mut c_void,
    u64,
    *mut c_void,
    Option<unsafe extern "C" fn(*mut c_void)>,
) -> i32;
#[cfg(unix)]
type NativeTakeTxnSlot = unsafe extern "C" fn(*mut c_void, u64, *mut *mut c_void) -> i32;

#[cfg(unix)]
#[repr(C)]
struct NativeTxnSlotApi {
    magic: u32,
    version: u16,
    reserved: u16,
    size: u32,
    get: Option<NativeGetTxnSlot>,
    set: Option<NativeSetTxnSlot>,
    take: Option<NativeTakeTxnSlot>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NativeTxnSlotError {
    Unavailable,
    Invalid,
    Occupied,
}

#[cfg(unix)]
fn native_txn_slot_api() -> Option<&'static NativeTxnSlotApi> {
    static API: OnceLock<Option<&'static NativeTxnSlotApi>> = OnceLock::new();
    *API.get_or_init(|| unsafe {
        let address = libc::dlsym(libc::RTLD_DEFAULT, c"ha_stream_txn_slot_api_v1".as_ptr());
        let api = (address as *const NativeTxnSlotApi).as_ref()?;
        (api.magic == NATIVE_TXN_SLOT_API_MAGIC
            && api.version == NATIVE_TXN_SLOT_API_VERSION
            && api.size >= std::mem::size_of::<NativeTxnSlotApi>() as u32
            && api.get.is_some()
            && api.set.is_some()
            && api.take.is_some())
        .then_some(api)
    })
}

#[cfg(not(unix))]
fn native_txn_slot_api() -> Option<()> {
    None
}

fn native_txn_slot_error(status: i32) -> NativeTxnSlotError {
    match status {
        NATIVE_TXN_SLOT_OCCUPIED => NativeTxnSlotError::Occupied,
        _ => NativeTxnSlotError::Invalid,
    }
}

fn native_txn_slot_error_message(error: NativeTxnSlotError) -> &'static str {
    match error {
        NativeTxnSlotError::Unavailable => "native transaction slots are unavailable",
        NativeTxnSlotError::Invalid => "native transaction slot rejected value",
        NativeTxnSlotError::Occupied => "native transaction slot belongs to another owner",
    }
}

#[derive(Clone)]
pub struct Txn {
    class: Table,
    pub c: Converters,
    pub f: Fetches,
    pub(crate) r#priv: Value,
    stream_id: OnceCell<usize>,
}

impl Txn {
    /// Returns a native identity stable for this transaction's stream lifetime.
    ///
    /// This is available with HAProxy builds that advertise
    /// [`TxnFields::TRANSACTION_ID`](crate::TxnFields::TRANSACTION_ID).
    #[inline]
    pub fn transaction_id(&self) -> Result<usize> {
        if let Some(&stream_id) = self.stream_id.get() {
            return Ok(stream_id);
        }
        let stream_id = self.class.raw_get::<LightUserData>("__stream_id")?.0 as usize;
        let _ = self.stream_id.set(stream_id);
        Ok(stream_id)
    }

    /// Returns the HAProxy-owned opaque transaction slot, when populated.
    ///
    /// New HAProxy builds expose the value directly; older builds use the
    /// compatible `get_txn_slot` fallback.
    #[inline]
    pub fn get_txn_slot(&self) -> Result<Option<LightUserData>> {
        #[cfg(unix)]
        if native_txn_slot_api().is_some() {
            return self.get_txn_slot_native();
        }
        if let Some(value) = self.class.raw_get("__txn_slot")? {
            return Ok(Some(value));
        }
        if self
            .class
            .get::<Option<Function>>("get_txn_slot")?
            .is_none()
        {
            return Err(mlua::Error::runtime("transaction slots are unavailable"));
        }
        self.class.call_method("get_txn_slot", ())
    }

    /// Returns the native slot mirror after the caller has verified support.
    #[inline]
    pub fn get_txn_slot_native(&self) -> Result<Option<LightUserData>> {
        match self.get_txn_slot_native_checked() {
            Ok(value) => Ok(value),
            Err(NativeTxnSlotError::Unavailable) => self.class.raw_get("__txn_slot"),
            Err(error) => Err(mlua::Error::runtime(native_txn_slot_error_message(error))),
        }
    }

    #[inline]
    pub fn get_txn_slot_native_checked(
        &self,
    ) -> std::result::Result<Option<LightUserData>, NativeTxnSlotError> {
        #[cfg(unix)]
        if let Some(api) = native_txn_slot_api() {
            let get = api.get.expect("validated native transaction slot API");
            let stream =
                self.transaction_id()
                    .map_err(|_| NativeTxnSlotError::Invalid)? as *const c_void;
            let mut value = std::ptr::null_mut();
            let status = unsafe { get(stream, NATIVE_TXN_SLOT_OWNER_DEFAULT, &mut value) };
            return if status == NATIVE_TXN_SLOT_OK {
                Ok((!value.is_null()).then_some(LightUserData(value)))
            } else {
                Err(native_txn_slot_error(status))
            };
        }
        Err(NativeTxnSlotError::Unavailable)
    }

    /// Stores an opaque value through HAProxy's native stream slot API.
    #[inline]
    pub fn set_txn_slot_native(&self, value: LightUserData, destroy: LightUserData) -> Result<()> {
        self.set_txn_slot_native_checked(value, destroy)
            .map_err(native_txn_slot_error_message)
            .map_err(mlua::Error::runtime)
    }

    #[inline]
    pub fn set_txn_slot_native_checked(
        &self,
        value: LightUserData,
        destroy: LightUserData,
    ) -> std::result::Result<(), NativeTxnSlotError> {
        if !value.0.is_null() && destroy.0.is_null() {
            return Err(NativeTxnSlotError::Invalid);
        }
        #[cfg(unix)]
        if let Some(api) = native_txn_slot_api() {
            let set = api.set.expect("validated native transaction slot API");
            let stream = self
                .transaction_id()
                .map_err(|_| NativeTxnSlotError::Invalid)? as *mut c_void;
            let destroy = if value.0.is_null() {
                None
            } else {
                Some(unsafe {
                    std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*mut c_void)>(destroy.0)
                })
            };
            return match unsafe { set(stream, NATIVE_TXN_SLOT_OWNER_DEFAULT, value.0, destroy) } {
                NATIVE_TXN_SLOT_OK => Ok(()),
                status => Err(native_txn_slot_error(status)),
            };
        }
        Err(NativeTxnSlotError::Unavailable)
    }

    /// Takes the native stream slot without invoking its destructor.
    #[inline]
    pub fn take_txn_slot_native(&self) -> Result<Option<LightUserData>> {
        self.take_txn_slot_native_checked()
            .map_err(native_txn_slot_error_message)
            .map_err(mlua::Error::runtime)
    }

    #[inline]
    pub fn take_txn_slot_native_checked(
        &self,
    ) -> std::result::Result<Option<LightUserData>, NativeTxnSlotError> {
        #[cfg(unix)]
        if let Some(api) = native_txn_slot_api() {
            let take = api.take.expect("validated native transaction slot API");
            let stream = self
                .transaction_id()
                .map_err(|_| NativeTxnSlotError::Invalid)? as *mut c_void;
            let mut value = std::ptr::null_mut();
            let status = unsafe { take(stream, NATIVE_TXN_SLOT_OWNER_DEFAULT, &mut value) };
            return if status == NATIVE_TXN_SLOT_OK {
                Ok((!value.is_null()).then_some(LightUserData(value)))
            } else {
                Err(native_txn_slot_error(status))
            };
        }
        Err(NativeTxnSlotError::Unavailable)
    }

    /// Stores an opaque value and its C-compatible destructor in HAProxy's
    /// stream-owned transaction slot.
    #[inline]
    pub fn set_txn_slot(&self, value: LightUserData, destroy: LightUserData) -> Result<()> {
        self.class.call_method("set_txn_slot", (value, destroy))
    }

    /// Takes the opaque transaction slot without running its destructor.
    #[inline]
    pub fn take_txn_slot(&self) -> Result<Option<LightUserData>> {
        self.class.call_method("take_txn_slot", ())
    }

    /// Returns an HTTP class object.
    #[inline]
    pub fn http(&self) -> Result<Http> {
        self.class.get("http")
    }

    /// Returns the request HTTPMessage object.
    pub fn http_req(&self) -> Result<HttpMessage> {
        self.class.get("http_req")
    }

    /// Returns the response HTTPMessage object.
    pub fn http_res(&self) -> Result<HttpMessage> {
        self.class.get("http_res")
    }

    /// Sends a log on the default syslog server if it is configured and on the stderr if it is allowed.
    #[inline]
    pub fn log(&self, level: LogLevel, msg: impl AsRef<str>) -> Result<()> {
        let msg = msg.as_ref();
        self.class.call_method("log", (level, msg))
    }

    /// Sends a log line with the default loglevel for the proxy associated with the transaction.
    #[inline]
    pub fn deflog(&self, msg: impl AsRef<str>) -> Result<()> {
        self.class.call_method("deflog", msg.as_ref())
    }

    /// Returns data stored in the current transaction (with the `set_priv()`) function.
    #[inline]
    pub fn get_priv<R: FromLua>(&self) -> Result<R> {
        self.class.call_method("get_priv", ())
    }

    /// Stores any data in the current HAProxy transaction.
    /// This action replaces the old stored data.
    #[inline]
    pub fn set_priv(&self, val: impl IntoLua) -> Result<()> {
        self.class.call_method("set_priv", val)
    }

    /// Returns data stored in the variable `name`.
    #[inline]
    pub fn get_var<R: FromLua>(&self, name: &str) -> Result<R> {
        self.class.call_method("get_var", name)
    }

    /// Store variable `name` in an HAProxy converting the type.
    #[inline]
    pub fn set_var(&self, name: &str, val: impl IntoLua) -> Result<()> {
        self.class.call_method("set_var", (name, val))
    }

    /// Store variable `name` in an HAProxy if the variable already exists.
    #[inline]
    pub fn set_var_if_exists(&self, name: &str, val: impl IntoLua) -> Result<()> {
        self.class.call_method("set_var", (name, val, true))
    }

    /// Unsets the variable `name`.
    #[inline]
    pub fn unset_var(&self, name: &str) -> Result<()> {
        self.class.call_method("unset_var", name)
    }

    /// Changes the log level of the current request.
    /// The `level` must be an integer between 0 and 7.
    #[inline]
    pub fn set_loglevel(&self, level: LogLevel) -> Result<()> {
        self.class.call_method("set_loglevel", level)
    }
}

impl FromLua for Txn {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(Txn {
            c: class.get("c")?,
            f: class.get("f")?,
            class,
            r#priv: Value::Nil,
            stream_id: OnceCell::new(),
        })
    }
}

impl Deref for Txn {
    type Target = Table;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.class
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn txn_class(lua: &Lua) -> Table {
        let class = lua.create_table().unwrap();
        class.set("f", lua.create_table().unwrap()).unwrap();
        class.set("c", lua.create_table().unwrap()).unwrap();
        class
    }

    #[test]
    fn reads_the_native_slot_mirror_without_calling_lua() {
        let lua = Lua::new();
        let class = txn_class(&lua);
        let mut marker = 0_u8;
        let pointer = (&mut marker as *mut u8).cast();
        class.set("__txn_slot", LightUserData(pointer)).unwrap();
        class
            .set(
                "get_txn_slot",
                lua.create_function(|_, ()| -> Result<()> { Ok(()) })
                    .unwrap(),
            )
            .unwrap();

        let txn = Txn::from_lua(Value::Table(class), &lua).unwrap();
        assert_eq!(txn.get_txn_slot().unwrap().unwrap().0, pointer);
        assert_eq!(txn.get_txn_slot_native().unwrap().unwrap().0, pointer);
    }

    #[test]
    fn native_slot_mirror_preserves_empty_slots() {
        let lua = Lua::new();
        let class = txn_class(&lua);
        class.set("__txn_slot", Value::Nil).unwrap();
        class
            .set(
                "get_txn_slot",
                lua.create_function(|_, ()| -> Result<Option<LightUserData>> { Ok(None) })
                    .unwrap(),
            )
            .unwrap();

        let txn = Txn::from_lua(Value::Table(class), &lua).unwrap();
        assert!(txn.get_txn_slot().unwrap().is_none());
    }

    #[test]
    fn native_slot_rejects_value_without_destructor() {
        let lua = Lua::new();
        let txn = Txn::from_lua(Value::Table(txn_class(&lua)), &lua).unwrap();
        let value = LightUserData((&0_u8 as *const u8).cast_mut().cast());

        assert!(txn
            .set_txn_slot_native(value, LightUserData(std::ptr::null_mut()))
            .is_err());
    }

    #[test]
    fn native_slot_status_preserves_owner_conflicts() {
        assert_eq!(native_txn_slot_error(-2), NativeTxnSlotError::Occupied);
        assert_eq!(native_txn_slot_error(-1), NativeTxnSlotError::Invalid);
        assert_eq!(
            native_txn_slot_error_message(NativeTxnSlotError::Occupied),
            "native transaction slot belongs to another owner"
        );
    }

    #[test]
    fn transaction_id_is_cached_after_the_first_successful_lookup() {
        let lua = Lua::new();
        let class = txn_class(&lua);
        let mut first_marker = 0_u8;
        let mut second_marker = 0_u8;
        let first = (&mut first_marker as *mut u8).cast();
        let second = (&mut second_marker as *mut u8).cast();
        class.set("__stream_id", LightUserData(first)).unwrap();

        let txn = Txn::from_lua(Value::Table(class.clone()), &lua).unwrap();
        assert_eq!(txn.transaction_id().unwrap(), first as usize);

        class.set("__stream_id", LightUserData(second)).unwrap();
        assert_eq!(txn.transaction_id().unwrap(), first as usize);
    }

    #[test]
    fn older_haproxy_uses_the_compatible_slot_fallback() {
        let lua = Lua::new();
        let class = txn_class(&lua);
        let mut marker = 0_u8;
        let pointer = (&mut marker as *mut u8).cast();
        class
            .set(
                "get_txn_slot",
                lua.create_function(move |_, ()| Ok(LightUserData(pointer)))
                    .unwrap(),
            )
            .unwrap();

        let txn = Txn::from_lua(Value::Table(class), &lua).unwrap();
        assert_eq!(txn.get_txn_slot().unwrap().unwrap().0, pointer);
    }
}
