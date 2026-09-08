use std::ffi::c_void;
use std::sync::OnceLock;

pub const NATIVE_FILTER_API_MAGIC: u32 = 0x4846_5331;
pub const NATIVE_FILTER_API_VERSION: u16 = 2;
pub const NATIVE_FILTER_STATUS_OK: i32 = 0;
pub const NATIVE_FILTER_STATUS_INVALID: i32 = -1;
pub const NATIVE_FILTER_STATUS_CONFLICT: i32 = -2;
pub const NATIVE_FILTER_STATUS_BUSY: i32 = -3;

pub const NATIVE_FILTER_EVENT_REQUEST_HEADERS: u32 = 1;
pub const NATIVE_FILTER_EVENT_RESPONSE_HEADERS: u32 = 2;
pub const NATIVE_FILTER_EVENT_FINISH: u32 = 3;
pub const NATIVE_FILTER_PEER_REQUEST: u32 = 1 << NATIVE_FILTER_EVENT_REQUEST_HEADERS;
pub const NATIVE_FILTER_PEER_RESPONSE: u32 = 1 << NATIVE_FILTER_EVENT_RESPONSE_HEADERS;
pub const NATIVE_FILTER_PEER_FINISH: u32 = 1 << NATIVE_FILTER_EVENT_FINISH;
pub const NATIVE_FILTER_FLAG_LUA_COMPAT: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeFilterBytes {
    pub data: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeFilterHeader {
    pub name: *const u8,
    pub name_len: usize,
    pub status: i32,
    pub data: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeFilterEvent {
    pub magic: u32,
    pub version: u16,
    pub reserved: u16,
    pub size: u32,
    pub kind: u32,
    pub stream: *mut c_void,
    pub headers: *const NativeFilterHeader,
    pub header_count: usize,
    pub method: NativeFilterBytes,
    pub path: NativeFilterBytes,
    pub host: NativeFilterBytes,
    pub reason: NativeFilterBytes,
    pub frontend: NativeFilterBytes,
    pub backend: NativeFilterBytes,
    pub status: i32,
    pub set_header: Option<NativeFilterSetHeader>,
    pub set_header_arg: *mut c_void,
    pub server: NativeFilterBytes,
    /// Client peer address, valid only until the callback returns.
    pub peer: NativeFilterBytes,
    /// Final HAProxy termination condition/state, valid only until the callback returns.
    pub termination_state: NativeFilterBytes,
}

pub type NativeFilterCallback =
    unsafe extern "C" fn(event: *const NativeFilterEvent, state: *mut *mut c_void) -> i32;
pub type NativeFilterSetHeader = unsafe extern "C" fn(
    arg: *mut c_void,
    name: *const u8,
    name_len: usize,
    value: *const u8,
    value_len: usize,
) -> i32;
pub type NativeFilterDestroy = unsafe extern "C" fn(state: *mut c_void);
pub type NativeFilterLuaCompat =
    unsafe extern "C" fn(args: *const NativeFilterBytes, arg_count: usize) -> i32;
pub type NativeFilterLuaAttribute = unsafe extern "C" fn(
    state: *mut *mut c_void,
    name: *const NativeFilterBytes,
    value: *const NativeFilterBytes,
) -> i32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeFilterDescriptor {
    pub magic: u32,
    pub version: u16,
    pub reserved: u16,
    pub size: u32,
    pub name: *const u8,
    pub name_len: usize,
    pub on_request: Option<NativeFilterCallback>,
    pub on_response: Option<NativeFilterCallback>,
    pub on_finish: Option<NativeFilterCallback>,
    pub destroy: Option<NativeFilterDestroy>,
    /// Event kinds for which HAProxy should populate `NativeFilterEvent::peer`.
    pub peer_events: u32,
    pub flags: u32,
    pub lua_compat: Option<NativeFilterLuaCompat>,
    pub lua_attribute: Option<NativeFilterLuaAttribute>,
}

// The descriptor is immutable process-wide ABI data; its raw pointers point to
// static name/callback storage supplied by the provider.
unsafe impl Sync for NativeFilterDescriptor {}

impl NativeFilterDescriptor {
    #[inline]
    pub fn is_compatible(&self) -> bool {
        self.magic == NATIVE_FILTER_API_MAGIC
            && self.version == NATIVE_FILTER_API_VERSION
            && self.reserved == 0
            && self.size >= std::mem::size_of::<Self>() as u32
            && !self.name.is_null()
            && self.name_len != 0
            && self.on_request.is_some()
            && self.on_response.is_some()
            && self.on_finish.is_some()
            && self.destroy.is_some()
            && self.flags & !NATIVE_FILTER_FLAG_LUA_COMPAT == 0
            && (self.flags & NATIVE_FILTER_FLAG_LUA_COMPAT == 0 || self.lua_compat.is_some())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NativeFilterRegistrationError {
    Unavailable,
    Invalid,
    Conflict,
    Busy,
}

#[cfg(unix)]
type NativeFilterRegister = unsafe extern "C" fn(*const NativeFilterDescriptor) -> i32;
#[cfg(unix)]
type NativeFilterClose = unsafe extern "C" fn(*const NativeFilterDescriptor) -> i32;
#[cfg(unix)]
type NativeFilterLuaCompatAvailable = unsafe extern "C" fn() -> i32;

#[cfg(unix)]
unsafe fn native_filter_register_from_address(
    address: *mut c_void,
) -> Option<NativeFilterRegister> {
    (!address.is_null()).then(|| std::mem::transmute(address))
}

#[cfg(unix)]
fn native_filter_register() -> Option<NativeFilterRegister> {
    static REGISTER: OnceLock<Option<NativeFilterRegister>> = OnceLock::new();
    *REGISTER.get_or_init(|| unsafe {
        let address = libc::dlsym(libc::RTLD_DEFAULT, c"ha_register_native_filter".as_ptr());
        native_filter_register_from_address(address)
    })
}

#[cfg(unix)]
fn native_filter_close() -> Option<NativeFilterClose> {
    static CLOSE: OnceLock<Option<NativeFilterClose>> = OnceLock::new();
    *CLOSE.get_or_init(|| unsafe {
        let address = libc::dlsym(libc::RTLD_DEFAULT, c"ha_close_native_filter".as_ptr());
        (!address.is_null()).then(|| std::mem::transmute(address))
    })
}

#[cfg(unix)]
fn native_filter_lua_compat_available_symbol() -> Option<NativeFilterLuaCompatAvailable> {
    static AVAILABLE: OnceLock<Option<NativeFilterLuaCompatAvailable>> = OnceLock::new();
    *AVAILABLE.get_or_init(|| unsafe {
        let address = libc::dlsym(
            libc::RTLD_DEFAULT,
            c"ha_native_filter_lua_compat_available".as_ptr(),
        );
        (!address.is_null()).then(|| std::mem::transmute(address))
    })
}

/// Returns whether HAProxy can dispatch compatible Lua filter configurations
/// to the registered native provider.
pub fn native_filter_lua_compat_available() -> bool {
    #[cfg(unix)]
    if let Some(available) = native_filter_lua_compat_available_symbol() {
        return unsafe { available() != 0 };
    }

    false
}

#[cfg(unix)]
type NativeFilterSetLuaAttribute =
    unsafe extern "C" fn(*mut c_void, *const NativeFilterBytes, *const NativeFilterBytes) -> i32;

#[cfg(unix)]
fn native_filter_set_lua_attribute_symbol() -> Option<NativeFilterSetLuaAttribute> {
    static SET: OnceLock<Option<NativeFilterSetLuaAttribute>> = OnceLock::new();
    *SET.get_or_init(|| unsafe {
        let address = libc::dlsym(
            libc::RTLD_DEFAULT,
            c"ha_native_filter_set_lua_attribute".as_ptr(),
        );
        (!address.is_null()).then(|| std::mem::transmute(address))
    })
}

/// Applies a Lua span attribute to the native filter instance for one stream.
///
/// Returns `false` when the native bridge is unavailable or no native filter
/// instance owns the stream, allowing the caller to use its Lua fallback.
#[inline]
pub fn set_native_filter_lua_attribute(stream: usize, name: &str, value: &str) -> bool {
    #[cfg(unix)]
    if let Some(set) = native_filter_set_lua_attribute_symbol() {
        let name = NativeFilterBytes {
            data: name.as_ptr(),
            len: name.len(),
        };
        let value = NativeFilterBytes {
            data: value.as_ptr(),
            len: value.len(),
        };
        return unsafe { set(stream as *mut c_void, &name, &value) } == NATIVE_FILTER_STATUS_OK;
    }

    false
}

#[inline]
fn registration_error(status: i32) -> NativeFilterRegistrationError {
    match status {
        NATIVE_FILTER_STATUS_CONFLICT => NativeFilterRegistrationError::Conflict,
        NATIVE_FILTER_STATUS_BUSY => NativeFilterRegistrationError::Busy,
        _ => NativeFilterRegistrationError::Invalid,
    }
}

/// Registers a process-resident native filter provider with a compatible HAProxy.
///
/// The descriptor and every callback must remain loaded until HAProxy has
/// released all filter instances created from it.
pub fn register_native_filter(
    descriptor: &'static NativeFilterDescriptor,
) -> Result<(), NativeFilterRegistrationError> {
    if !descriptor.is_compatible() {
        return Err(NativeFilterRegistrationError::Invalid);
    }

    #[cfg(unix)]
    if let Some(register) = native_filter_register() {
        return match unsafe { register(descriptor) } {
            NATIVE_FILTER_STATUS_OK => Ok(()),
            status => Err(registration_error(status)),
        };
    }

    Err(NativeFilterRegistrationError::Unavailable)
}

/// Closes a registered provider before its code or descriptor is unloaded.
///
/// HAProxy rejects the close while any filter instance can still call the
/// provider. A successful close releases HAProxy's descriptor reference; it
/// does not unload the provider itself.
pub fn close_native_filter(
    descriptor: &'static NativeFilterDescriptor,
) -> Result<(), NativeFilterRegistrationError> {
    if !descriptor.is_compatible() {
        return Err(NativeFilterRegistrationError::Invalid);
    }

    #[cfg(unix)]
    if let Some(close) = native_filter_close() {
        return match unsafe { close(descriptor) } {
            NATIVE_FILTER_STATUS_OK => Ok(()),
            status => Err(registration_error(status)),
        };
    }

    Err(NativeFilterRegistrationError::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn callback(
        _event: *const NativeFilterEvent,
        _state: *mut *mut c_void,
    ) -> i32 {
        0
    }

    unsafe extern "C" fn destroy(_state: *mut c_void) {}

    unsafe extern "C" fn lua_compat(_args: *const NativeFilterBytes, _arg_count: usize) -> i32 {
        0
    }

    fn descriptor() -> NativeFilterDescriptor {
        NativeFilterDescriptor {
            magic: NATIVE_FILTER_API_MAGIC,
            version: NATIVE_FILTER_API_VERSION,
            reserved: 0,
            size: std::mem::size_of::<NativeFilterDescriptor>() as u32,
            name: b"test".as_ptr(),
            name_len: 4,
            on_request: Some(callback),
            on_response: Some(callback),
            on_finish: Some(callback),
            destroy: Some(destroy),
            peer_events: 0,
            flags: 0,
            lua_compat: None,
            lua_attribute: None,
        }
    }

    #[test]
    fn accepts_only_complete_current_descriptors() {
        let mut descriptor = descriptor();
        assert!(descriptor.is_compatible());

        descriptor.magic = 0;
        assert!(!descriptor.is_compatible());
        descriptor.magic = NATIVE_FILTER_API_MAGIC;
        descriptor.size = 1;
        assert!(!descriptor.is_compatible());
        descriptor.size = std::mem::size_of::<NativeFilterDescriptor>() as u32;
        descriptor.reserved = 1;
        assert!(!descriptor.is_compatible());
        descriptor.reserved = 0;
        descriptor.name_len = 0;
        assert!(!descriptor.is_compatible());
        descriptor.name_len = 4;
        descriptor.on_finish = None;
        assert!(!descriptor.is_compatible());
        descriptor.on_finish = Some(callback);
        descriptor.flags = NATIVE_FILTER_FLAG_LUA_COMPAT;
        assert!(!descriptor.is_compatible());
        descriptor.lua_compat = Some(lua_compat);
        assert!(descriptor.is_compatible());
    }

    #[test]
    fn event_contract_exposes_scoped_final_outcome_views() {
        let peer = b"127.0.0.1";
        let termination = b"--";
        let event = NativeFilterEvent {
            magic: NATIVE_FILTER_API_MAGIC,
            version: NATIVE_FILTER_API_VERSION,
            size: std::mem::size_of::<NativeFilterEvent>() as u32,
            peer: NativeFilterBytes {
                data: peer.as_ptr(),
                len: peer.len(),
            },
            termination_state: NativeFilterBytes {
                data: termination.as_ptr(),
                len: termination.len(),
            },
            ..NativeFilterEvent::default()
        };

        assert_eq!(event.version, NATIVE_FILTER_API_VERSION);
        assert_eq!(event.peer.len, peer.len());
        assert_eq!(event.termination_state.len, termination.len());
    }

    #[test]
    fn peer_event_masks_are_independent() {
        assert_eq!(
            NATIVE_FILTER_PEER_REQUEST | NATIVE_FILTER_PEER_RESPONSE | NATIVE_FILTER_PEER_FINISH,
            0b1110
        );
        assert_eq!(NATIVE_FILTER_PEER_REQUEST & NATIVE_FILTER_PEER_RESPONSE, 0);
    }

    #[test]
    fn maps_provider_conflicts_without_collapsing_them_into_invalid() {
        assert_eq!(
            registration_error(NATIVE_FILTER_STATUS_CONFLICT),
            NativeFilterRegistrationError::Conflict
        );
        assert_eq!(
            registration_error(NATIVE_FILTER_STATUS_INVALID),
            NativeFilterRegistrationError::Invalid
        );
        assert_eq!(
            registration_error(NATIVE_FILTER_STATUS_BUSY),
            NativeFilterRegistrationError::Busy
        );
    }

    #[cfg(unix)]
    #[test]
    fn missing_registration_symbol_is_unavailable() {
        assert!(unsafe { native_filter_register_from_address(std::ptr::null_mut()) }.is_none());
    }
}
