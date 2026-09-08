pub const NATIVE_MODULE_API_MAGIC: u32 = 0x484d_5331;
pub const NATIVE_MODULE_API_VERSION: u16 = 1;
pub const NATIVE_MODULE_STATUS_OK: i32 = 0;
pub const NATIVE_MODULE_STATUS_ERROR: i32 = -1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeModuleDescriptor {
    pub magic: u32,
    pub version: u16,
    pub reserved: u16,
    pub size: u32,
    pub name: *const u8,
    pub name_len: usize,
    pub init: Option<NativeModuleInit>,
    pub close: Option<NativeModuleClose>,
}

pub type NativeModuleInit = unsafe extern "C" fn() -> i32;
pub type NativeModuleClose = unsafe extern "C" fn();
pub type NativeModuleGetDescriptor = unsafe extern "C" fn() -> *const NativeModuleDescriptor;

// The descriptor is immutable process-wide ABI data supplied by a module.
unsafe impl Sync for NativeModuleDescriptor {}

impl NativeModuleDescriptor {
    #[inline]
    pub fn is_compatible(&self) -> bool {
        self.magic == NATIVE_MODULE_API_MAGIC
            && self.version == NATIVE_MODULE_API_VERSION
            && self.reserved == 0
            && self.size >= std::mem::size_of::<Self>() as u32
            && !self.name.is_null()
            && self.name_len != 0
            && self.init.is_some()
            && self.close.is_some()
    }
}

#[inline]
pub fn native_module_descriptor_symbol() -> &'static [u8] {
    b"ha_native_module_get_descriptor\0"
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn init() -> i32 {
        NATIVE_MODULE_STATUS_OK
    }

    unsafe extern "C" fn close() {}

    fn descriptor() -> NativeModuleDescriptor {
        NativeModuleDescriptor {
            magic: NATIVE_MODULE_API_MAGIC,
            version: NATIVE_MODULE_API_VERSION,
            reserved: 0,
            size: std::mem::size_of::<NativeModuleDescriptor>() as u32,
            name: b"test".as_ptr(),
            name_len: 4,
            init: Some(init),
            close: Some(close),
        }
    }

    #[test]
    fn accepts_only_complete_current_descriptors() {
        let mut descriptor = descriptor();
        assert!(descriptor.is_compatible());

        descriptor.magic = 0;
        assert!(!descriptor.is_compatible());
        descriptor.magic = NATIVE_MODULE_API_MAGIC;
        descriptor.size = 1;
        assert!(!descriptor.is_compatible());
        descriptor.size = std::mem::size_of::<NativeModuleDescriptor>() as u32;
        descriptor.reserved = 1;
        assert!(!descriptor.is_compatible());
        descriptor.reserved = 0;
        descriptor.name_len = 0;
        assert!(!descriptor.is_compatible());
        descriptor.name_len = 4;
        descriptor.init = None;
        assert!(!descriptor.is_compatible());
    }

    #[test]
    fn exports_the_stable_loader_symbol_name() {
        assert_eq!(
            native_module_descriptor_symbol(),
            b"ha_native_module_get_descriptor\0"
        );
    }
}
