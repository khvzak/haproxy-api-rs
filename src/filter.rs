use std::any::type_name;
use std::ops::{Deref, DerefMut};

use mlua::{AnyUserData, IntoLua, Lua, ObjectLike, Result, Table, UserData, Value, Variadic};

use crate::{Channel, Core, HttpMessage, LogLevel, Txn};

/// Represents methods available to call in [`UserFilter`].
pub struct FilterMethod;

impl FilterMethod {
    pub const START_ANALYZE: u8 = 0b00000001;
    pub const END_ANALYZE: u8 = 0b00000010;
    pub const HTTP_HEADERS: u8 = 0b00000100;
    pub const HTTP_PAYLOAD: u8 = 0b00001000;
    pub const HTTP_END: u8 = 0b00010000;
    pub const HTTP_REPLY: u8 = 0b00100000;

    pub const ALL: u8 = u8::MAX;
}

/// Selects which transaction helper objects HAProxy materializes for a filter callback.
///
/// The default is [`ALL`](Self::ALL), preserving the complete Lua transaction
/// surface. Filters that use only the callback arguments and a small subset of
/// helpers can opt into a smaller per-request allocation footprint.
pub struct TxnFields;

impl TxnFields {
    pub const FETCHES: u32 = 1 << 0;
    pub const SAFE_FETCHES: u32 = 1 << 1;
    pub const CONVERTERS: u32 = 1 << 2;
    pub const SAFE_CONVERTERS: u32 = 1 << 3;
    pub const REQUEST_CHANNEL: u32 = 1 << 4;
    pub const RESPONSE_CHANNEL: u32 = 1 << 5;
    pub const HTTP: u32 = 1 << 6;
    pub const HTTP_REQUEST: u32 = 1 << 7;
    pub const HTTP_RESPONSE: u32 = 1 << 8;
    /// Exposes a stable native transaction identity for the lifetime of a stream.
    pub const TRANSACTION_ID: u32 = 1 << 9;
    /// Enables HAProxy's stream-owned opaque transaction slot.
    pub const TRANSACTION_SLOT: u32 = 1 << 10;
    /// Keeps opt-in identity and ownership state out of the legacy default.
    pub const ALL: u32 = (1 << 9) - 1;
}

/// A code that filter callback functions may return.
pub enum FilterResult {
    /// A filtering step is finished for filter.
    Continue,
    /// A filtering step must be paused, waiting for more data or for an external event depending on filter.
    Wait,
    /// Trigger a error
    Error,
}

impl FilterResult {
    fn code(&self) -> i8 {
        match self {
            FilterResult::Continue => 1,
            FilterResult::Wait => 0,
            FilterResult::Error => -1,
        }
    }
}

/// A flag corresponding to the filter flag FLT_CFG_FL_HTX.
/// When it is set for a filter, it means the filter is able to filter HTTP streams.
const FLT_CFG_FL_HTX: u8 = 1;

/// A trait that defines all required callback functions to implement filters.
pub trait UserFilter: Sized {
    /// Sets methods available for this filter.
    /// By default ALL
    const METHODS: u8 = FilterMethod::ALL;

    /// Continue execution if a filter callback returns an error.
    const CONTINUE_IF_ERROR: bool = true;

    /// Transaction helper objects to materialize for each callback.
    const TXN_FIELDS: u32 = TxnFields::ALL;

    /// Creates a new instance of filter.
    fn new(lua: &Lua, args: Table) -> Result<Self>;

    /// Called when the analysis starts on the channel `chn`.
    fn start_analyze(&mut self, lua: &Lua, txn: Txn, chn: Channel) -> Result<FilterResult> {
        let _ = (lua, txn, chn);
        Ok(FilterResult::Continue)
    }

    /// Called when the analysis ends on the channel `chn`.
    fn end_analyze(&mut self, lua: &Lua, txn: Txn, chn: Channel) -> Result<FilterResult> {
        let _ = (lua, txn, chn);
        Ok(FilterResult::Continue)
    }

    /// Called just before the HTTP payload analysis and after any processing on the HTTP message `msg`.
    fn http_headers(&mut self, lua: &Lua, txn: Txn, msg: HttpMessage) -> Result<FilterResult> {
        let _ = (lua, txn, msg);
        Ok(FilterResult::Continue)
    }

    /// Called during the HTTP payload analysis on the HTTP message `msg`.
    fn http_payload(&mut self, lua: &Lua, txn: Txn, msg: HttpMessage) -> Result<Option<usize>> {
        let _ = (lua, txn, msg);
        Ok(None)
    }

    /// Called after the HTTP payload analysis on the HTTP message `msg`.
    fn http_end(&mut self, lua: &Lua, txn: Txn, msg: HttpMessage) -> Result<FilterResult> {
        let _ = (lua, txn, msg);
        Ok(FilterResult::Continue)
    }

    /// Called when HAProxy sends an internally generated HTTP response.
    fn http_reply(&mut self, lua: &Lua, txn: Txn, status: i16) -> Result<FilterResult> {
        let _ = (lua, txn, status);
        Ok(FilterResult::Continue)
    }

    //
    // HAProxy provided methods
    //

    /// Enable the data filtering on the channel `chn` for the current filter.
    /// It may be called at any time from any callback functions proceeding the data analysis.
    fn register_data_filter(lua: &Lua, txn: Txn, chn: Channel) -> Result<()> {
        let global_filter = lua.globals().raw_get::<Table>("filter")?;
        global_filter.call_function::<()>("register_data_filter", (txn.r#priv, chn))?;
        Ok(())
    }

    /// Disable the data filtering on the channel `chn` for the current filter.
    /// It may be called at any time from any callback functions.
    fn unregister_data_filter(lua: &Lua, txn: Txn, chn: Channel) -> Result<()> {
        let filter = lua.globals().raw_get::<Table>("filter")?;
        filter.call_function::<()>("unregister_data_filter", (txn.r#priv, chn))?;
        Ok(())
    }

    /// Set the pause timeout to the specified time, defined in milliseconds.
    fn wake_time(lua: &Lua, milliseconds: u64) -> Result<()> {
        let filter = lua.globals().raw_get::<Table>("filter")?;
        filter.call_function::<()>("wake_time", milliseconds)?;
        Ok(())
    }
}

pub(crate) struct UserFilterWrapper<T>(T);

impl<T> UserFilterWrapper<T>
where
    T: UserFilter + 'static,
{
    pub(crate) fn make_class(lua: &Lua) -> Result<Table> {
        let class = lua.create_table()?;
        class.raw_set("__index", &class)?;

        // Attributes
        class.raw_set("id", type_name::<T>())?;
        class.raw_set("flags", FLT_CFG_FL_HTX)?;
        class.raw_set("txn_fields", T::TXN_FIELDS)?;

        //
        // Methods
        //
        let class_key = lua.create_registry_value(&class)?;
        class.raw_set(
            "new",
            lua.create_function(move |lua, class: Table| {
                let args = class.raw_get("args")?;
                let filter = match T::new(lua, args) {
                    Ok(filter) => filter,
                    Err(err) => {
                        let core = Core::new(lua)?;
                        let msg = format!("Filter '{}': {err}", type_name::<T>());
                        core.log(LogLevel::Err, msg)?;
                        return Ok(Value::Nil);
                    }
                };
                let this = lua.create_sequence_from([Self(filter)])?;
                let class = lua.registry_value::<Table>(&class_key)?;
                this.set_metatable(Some(class))?;
                Ok(Value::Table(this))
            })?,
        )?;

        if T::METHODS & FilterMethod::START_ANALYZE != 0 {
            class.raw_set(
                "start_analyze",
                lua.create_function(|lua, (t, mut txn, chn): (Table, Txn, Channel)| {
                    let ud = t.raw_get::<AnyUserData>(1)?;
                    let mut this = ud.borrow_mut::<Self>()?;
                    txn.r#priv = Value::Table(t);
                    Self::process_result(lua, this.start_analyze(lua, txn, chn))
                })?,
            )?;
        }

        if T::METHODS & FilterMethod::END_ANALYZE != 0 {
            class.raw_set(
                "end_analyze",
                lua.create_function(|lua, (t, mut txn, chn): (Table, Txn, Channel)| {
                    let ud = t.raw_get::<AnyUserData>(1)?;
                    let mut this = ud.borrow_mut::<Self>()?;
                    txn.r#priv = Value::Table(t);
                    Self::process_result(lua, this.end_analyze(lua, txn, chn))
                })?,
            )?;
        }

        if T::METHODS & FilterMethod::HTTP_HEADERS != 0 {
            class.raw_set(
                "http_headers",
                lua.create_function(|lua, (t, mut txn, msg): (Table, Txn, HttpMessage)| {
                    let ud = t.raw_get::<AnyUserData>(1)?;
                    let mut this = ud.borrow_mut::<Self>()?;
                    txn.r#priv = Value::Table(t);
                    Self::process_result(lua, this.http_headers(lua, txn, msg))
                })?,
            )?;
        }

        if T::METHODS & FilterMethod::HTTP_PAYLOAD != 0 {
            class.raw_set(
                "http_payload",
                lua.create_function(|lua, (t, mut txn, msg): (Table, Txn, HttpMessage)| {
                    let ud = t.raw_get::<AnyUserData>(1)?;
                    let mut this = ud.borrow_mut::<Self>()?;
                    txn.r#priv = Value::Table(t);
                    let mut res = Variadic::new();
                    match this.http_payload(lua, txn, msg) {
                        Ok(Some(len)) => {
                            res.push(len.into_lua(lua)?);
                        }
                        Ok(None) => {}
                        Err(err) if T::CONTINUE_IF_ERROR => {
                            if let Ok(core) = Core::new(lua) {
                                let _ = core.log(
                                    LogLevel::Err,
                                    format!("Filter '{}': {}", type_name::<T>(), err),
                                );
                            }
                        }
                        Err(err) => return Err(err),
                    };
                    Ok(res)
                })?,
            )?;
        }

        if T::METHODS & FilterMethod::HTTP_END != 0 {
            class.raw_set(
                "http_end",
                lua.create_function(|lua, (t, mut txn, msg): (Table, Txn, HttpMessage)| {
                    let ud = t.raw_get::<AnyUserData>(1)?;
                    let mut this = ud.borrow_mut::<Self>()?;
                    txn.r#priv = Value::Table(t);
                    Self::process_result(lua, this.http_end(lua, txn, msg))
                })?,
            )?;
        }

        if T::METHODS & FilterMethod::HTTP_REPLY != 0 {
            class.raw_set(
                "http_reply",
                lua.create_function(|lua, (t, mut txn, status): (Table, Txn, i16)| {
                    let ud = t.raw_get::<AnyUserData>(1)?;
                    let mut this = ud.borrow_mut::<Self>()?;
                    txn.r#priv = Value::Table(t);
                    Self::process_result(lua, this.http_reply(lua, txn, status))
                })?,
            )?;
        }

        Ok(class)
    }

    #[inline]
    fn process_result(lua: &Lua, res: Result<FilterResult>) -> Result<i8> {
        match res {
            Ok(res) => Ok(res.code()),
            Err(err) if T::CONTINUE_IF_ERROR => {
                if let Ok(core) = Core::new(lua) {
                    let _ = core.log(
                        LogLevel::Err,
                        format!("Filter '{}': {}", type_name::<T>(), err),
                    );
                }
                Ok(FilterResult::Continue.code())
            }
            Err(err) => Err(err),
        }
    }
}

impl<T> UserData for UserFilterWrapper<T> where T: UserFilter + 'static {}

impl<T> Deref for UserFilterWrapper<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for UserFilterWrapper<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MinimalFilter;

    impl UserFilter for MinimalFilter {
        const METHODS: u8 = 0;
        const TXN_FIELDS: u32 = TxnFields::FETCHES | TxnFields::TRANSACTION_ID;

        fn new(_: &Lua, _: Table) -> Result<Self> {
            Ok(Self)
        }
    }

    #[test]
    fn publishes_the_declared_transaction_capabilities() {
        let lua = Lua::new();
        let class = UserFilterWrapper::<MinimalFilter>::make_class(&lua).unwrap();

        assert_eq!(
            class.get::<u32>("txn_fields").unwrap(),
            TxnFields::FETCHES | TxnFields::TRANSACTION_ID
        );
    }

    #[test]
    fn legacy_all_keeps_opt_in_transaction_state_out() {
        assert_eq!(TxnFields::ALL & TxnFields::TRANSACTION_ID, 0);
        assert_eq!(TxnFields::ALL & TxnFields::TRANSACTION_SLOT, 0);
    }

    struct ReplyFilter;

    impl UserFilter for ReplyFilter {
        const METHODS: u8 = FilterMethod::HTTP_REPLY;

        fn new(_: &Lua, _: Table) -> Result<Self> {
            Ok(Self)
        }
    }

    #[test]
    fn publishes_only_declared_http_reply_callback() {
        let lua = Lua::new();
        let class = UserFilterWrapper::<ReplyFilter>::make_class(&lua).unwrap();

        assert!(matches!(
            class.raw_get::<Value>("http_reply").unwrap(),
            Value::Function(_)
        ));
        assert!(matches!(
            UserFilterWrapper::<MinimalFilter>::make_class(&lua)
                .unwrap()
                .raw_get::<Value>("http_reply")
                .unwrap(),
            Value::Nil
        ));
    }
}
