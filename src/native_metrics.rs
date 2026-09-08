use std::ffi::c_void;
use std::sync::OnceLock;

use crate::native_filter::{
    NativeFilterBytes, NativeFilterRegistrationError, NATIVE_FILTER_API_MAGIC,
    NATIVE_FILTER_STATUS_BUSY, NATIVE_FILTER_STATUS_CONFLICT, NATIVE_FILTER_STATUS_OK,
};

pub const NATIVE_METRICS_API_MAGIC: u32 = NATIVE_FILTER_API_MAGIC;
pub const NATIVE_METRICS_API_VERSION: u16 = 1;
pub const NATIVE_METRICS_EVENT_FINISH: u32 = 1;
pub const NATIVE_METRICS_API_VERSION_V2: u16 = 2;
pub const NATIVE_METRICS_EVENT_OBSERVATION_V2: u32 = 1;
pub const NATIVE_METRICS_API_VERSION_V3: u16 = 3;
pub const NATIVE_METRICS_EVENT_BATCH_V3: u32 = 1;
pub const NATIVE_METRICS_EVENT_SNAPSHOT_V3: u32 = 2;
pub const NATIVE_METRIC_COUNTER_V2: u32 = 1;
pub const NATIVE_METRIC_GAUGE_V2: u32 = 2;
pub const NATIVE_METRIC_HISTOGRAM_V2: u32 = 3;
pub const NATIVE_METRIC_TEMPORALITY_DELTA_V2: u32 = 1;
pub const NATIVE_METRIC_TEMPORALITY_CUMULATIVE_V2: u32 = 2;
pub const NATIVE_METRICS_MAX_LABELS_V2: usize = 4;
pub const NATIVE_METRICS_MAX_EVENTS_V3: usize = 13;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeMetricsEvent {
    pub magic: u32,
    pub version: u16,
    pub reserved: u16,
    pub size: u32,
    pub kind: u32,
    pub stream: *mut c_void,
    pub name: NativeFilterBytes,
    pub value: u64,
}

pub type NativeMetricsCallback = unsafe extern "C" fn(event: *const NativeMetricsEvent) -> i32;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeMetricsLabelV2 {
    pub key: NativeFilterBytes,
    pub value: NativeFilterBytes,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeMetricsEventV2 {
    pub magic: u32,
    pub version: u16,
    pub reserved: u16,
    pub size: u32,
    pub kind: u32,
    pub stream: *mut c_void,
    pub name: NativeFilterBytes,
    pub unit: NativeFilterBytes,
    pub source: NativeFilterBytes,
    pub metric_type: u32,
    pub temporality: u32,
    pub value_u64: u64,
    pub value_f64: f64,
    pub labels: *const NativeMetricsLabelV2,
    pub label_count: u16,
    pub reserved2: u16,
}

pub type NativeMetricsCallbackV2 = unsafe extern "C" fn(event: *const NativeMetricsEventV2) -> i32;

/// A synchronous batch of borrowed v2 observations.
///
/// The callback must consume `events` before returning; neither the batch nor
/// any event or label payload is retained by the provider.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeMetricsBatchV3 {
    pub magic: u32,
    pub version: u16,
    pub reserved: u16,
    pub size: u32,
    pub kind: u32,
    pub stream: *mut c_void,
    pub events: *const NativeMetricsEventV2,
    pub event_count: u16,
    pub reserved2: u16,
}

pub type NativeMetricsCallbackV3 = unsafe extern "C" fn(batch: *const NativeMetricsBatchV3) -> i32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeMetricsDescriptor {
    pub magic: u32,
    pub version: u16,
    pub reserved: u16,
    pub size: u32,
    pub name: *const u8,
    pub name_len: usize,
    pub on_finish: Option<NativeMetricsCallback>,
}

unsafe impl Sync for NativeMetricsDescriptor {}

impl NativeMetricsDescriptor {
    #[inline]
    pub fn is_compatible(&self) -> bool {
        self.magic == NATIVE_METRICS_API_MAGIC
            && self.version == NATIVE_METRICS_API_VERSION
            && self.reserved == 0
            && self.size >= std::mem::size_of::<Self>() as u32
            && !self.name.is_null()
            && self.name_len != 0
            && self.on_finish.is_some()
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeMetricsDescriptorV2 {
    pub magic: u32,
    pub version: u16,
    pub reserved: u16,
    pub size: u32,
    pub name: *const u8,
    pub name_len: usize,
    pub on_observation: Option<NativeMetricsCallbackV2>,
}

unsafe impl Sync for NativeMetricsDescriptorV2 {}

impl NativeMetricsDescriptorV2 {
    #[inline]
    pub fn is_compatible(&self) -> bool {
        self.magic == NATIVE_METRICS_API_MAGIC
            && self.version == NATIVE_METRICS_API_VERSION_V2
            && self.reserved == 0
            && self.size >= std::mem::size_of::<Self>() as u32
            && !self.name.is_null()
            && self.name_len != 0
            && self.on_observation.is_some()
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeMetricsDescriptorV3 {
    pub magic: u32,
    pub version: u16,
    pub reserved: u16,
    pub size: u32,
    pub name: *const u8,
    pub name_len: usize,
    pub on_batch: Option<NativeMetricsCallbackV3>,
}

unsafe impl Sync for NativeMetricsDescriptorV3 {}

impl NativeMetricsDescriptorV3 {
    #[inline]
    pub fn is_compatible(&self) -> bool {
        self.magic == NATIVE_METRICS_API_MAGIC
            && self.version == NATIVE_METRICS_API_VERSION_V3
            && self.reserved == 0
            && self.size >= std::mem::size_of::<Self>() as u32
            && !self.name.is_null()
            && self.name_len != 0
            && self.on_batch.is_some()
    }
}

#[cfg(unix)]
type NativeMetricsRegister = unsafe extern "C" fn(*const NativeMetricsDescriptor) -> i32;
#[cfg(unix)]
type NativeMetricsClose = unsafe extern "C" fn(*const NativeMetricsDescriptor) -> i32;
#[cfg(unix)]
type NativeMetricsRegisterV2 = unsafe extern "C" fn(*const NativeMetricsDescriptorV2) -> i32;
#[cfg(unix)]
type NativeMetricsCloseV2 = unsafe extern "C" fn(*const NativeMetricsDescriptorV2) -> i32;
#[cfg(unix)]
type NativeMetricsRegisterV3 = unsafe extern "C" fn(*const NativeMetricsDescriptorV3) -> i32;
#[cfg(unix)]
type NativeMetricsCloseV3 = unsafe extern "C" fn(*const NativeMetricsDescriptorV3) -> i32;

#[cfg(unix)]
unsafe fn native_metrics_register_from_address(
    address: *mut c_void,
) -> Option<NativeMetricsRegister> {
    (!address.is_null()).then(|| std::mem::transmute(address))
}

#[cfg(unix)]
fn native_metrics_register() -> Option<NativeMetricsRegister> {
    static REGISTER: OnceLock<Option<NativeMetricsRegister>> = OnceLock::new();
    *REGISTER.get_or_init(|| unsafe {
        let address = libc::dlsym(libc::RTLD_DEFAULT, c"ha_register_native_metrics".as_ptr());
        native_metrics_register_from_address(address)
    })
}

#[cfg(unix)]
fn native_metrics_close() -> Option<NativeMetricsClose> {
    static CLOSE: OnceLock<Option<NativeMetricsClose>> = OnceLock::new();
    *CLOSE.get_or_init(|| unsafe {
        let address = libc::dlsym(libc::RTLD_DEFAULT, c"ha_close_native_metrics".as_ptr());
        (!address.is_null()).then(|| std::mem::transmute(address))
    })
}

#[cfg(unix)]
fn native_metrics_register_v2() -> Option<NativeMetricsRegisterV2> {
    static REGISTER: OnceLock<Option<NativeMetricsRegisterV2>> = OnceLock::new();
    *REGISTER.get_or_init(|| unsafe {
        let address = libc::dlsym(
            libc::RTLD_DEFAULT,
            c"ha_register_native_metrics_v2".as_ptr(),
        );
        (!address.is_null()).then(|| std::mem::transmute(address))
    })
}

#[cfg(unix)]
fn native_metrics_close_v2() -> Option<NativeMetricsCloseV2> {
    static CLOSE: OnceLock<Option<NativeMetricsCloseV2>> = OnceLock::new();
    *CLOSE.get_or_init(|| unsafe {
        let address = libc::dlsym(libc::RTLD_DEFAULT, c"ha_close_native_metrics_v2".as_ptr());
        (!address.is_null()).then(|| std::mem::transmute(address))
    })
}

#[cfg(unix)]
fn native_metrics_register_v3() -> Option<NativeMetricsRegisterV3> {
    static REGISTER: OnceLock<Option<NativeMetricsRegisterV3>> = OnceLock::new();
    *REGISTER.get_or_init(|| unsafe {
        let address = libc::dlsym(
            libc::RTLD_DEFAULT,
            c"ha_register_native_metrics_v3".as_ptr(),
        );
        (!address.is_null()).then(|| std::mem::transmute(address))
    })
}

#[cfg(unix)]
fn native_metrics_close_v3() -> Option<NativeMetricsCloseV3> {
    static CLOSE: OnceLock<Option<NativeMetricsCloseV3>> = OnceLock::new();
    *CLOSE.get_or_init(|| unsafe {
        let address = libc::dlsym(libc::RTLD_DEFAULT, c"ha_close_native_metrics_v3".as_ptr());
        (!address.is_null()).then(|| std::mem::transmute(address))
    })
}

#[inline]
fn registration_error(status: i32) -> NativeFilterRegistrationError {
    match status {
        NATIVE_FILTER_STATUS_CONFLICT => NativeFilterRegistrationError::Conflict,
        NATIVE_FILTER_STATUS_BUSY => NativeFilterRegistrationError::Busy,
        _ => NativeFilterRegistrationError::Invalid,
    }
}

pub fn register_native_metrics(
    descriptor: &'static NativeMetricsDescriptor,
) -> Result<(), NativeFilterRegistrationError> {
    if !descriptor.is_compatible() {
        return Err(NativeFilterRegistrationError::Invalid);
    }

    #[cfg(unix)]
    if let Some(register) = native_metrics_register() {
        return match unsafe { register(descriptor) } {
            NATIVE_FILTER_STATUS_OK => Ok(()),
            status => Err(registration_error(status)),
        };
    }

    Err(NativeFilterRegistrationError::Unavailable)
}

pub fn close_native_metrics(
    descriptor: &'static NativeMetricsDescriptor,
) -> Result<(), NativeFilterRegistrationError> {
    if !descriptor.is_compatible() {
        return Err(NativeFilterRegistrationError::Invalid);
    }

    #[cfg(unix)]
    if let Some(close) = native_metrics_close() {
        return match unsafe { close(descriptor) } {
            NATIVE_FILTER_STATUS_OK => Ok(()),
            status => Err(registration_error(status)),
        };
    }

    Err(NativeFilterRegistrationError::Unavailable)
}

pub fn register_native_metrics_v2(
    descriptor: &'static NativeMetricsDescriptorV2,
) -> Result<(), NativeFilterRegistrationError> {
    if !descriptor.is_compatible() {
        return Err(NativeFilterRegistrationError::Invalid);
    }

    #[cfg(unix)]
    if let Some(register) = native_metrics_register_v2() {
        return match unsafe { register(descriptor) } {
            NATIVE_FILTER_STATUS_OK => Ok(()),
            status => Err(registration_error(status)),
        };
    }

    Err(NativeFilterRegistrationError::Unavailable)
}

pub fn close_native_metrics_v2(
    descriptor: &'static NativeMetricsDescriptorV2,
) -> Result<(), NativeFilterRegistrationError> {
    if !descriptor.is_compatible() {
        return Err(NativeFilterRegistrationError::Invalid);
    }

    #[cfg(unix)]
    if let Some(close) = native_metrics_close_v2() {
        return match unsafe { close(descriptor) } {
            NATIVE_FILTER_STATUS_OK => Ok(()),
            status => Err(registration_error(status)),
        };
    }

    Err(NativeFilterRegistrationError::Unavailable)
}

pub fn register_native_metrics_v3(
    descriptor: &'static NativeMetricsDescriptorV3,
) -> Result<(), NativeFilterRegistrationError> {
    if !descriptor.is_compatible() {
        return Err(NativeFilterRegistrationError::Invalid);
    }

    #[cfg(unix)]
    if let Some(register) = native_metrics_register_v3() {
        return match unsafe { register(descriptor) } {
            NATIVE_FILTER_STATUS_OK => Ok(()),
            status => Err(registration_error(status)),
        };
    }

    Err(NativeFilterRegistrationError::Unavailable)
}

pub fn close_native_metrics_v3(
    descriptor: &'static NativeMetricsDescriptorV3,
) -> Result<(), NativeFilterRegistrationError> {
    if !descriptor.is_compatible() {
        return Err(NativeFilterRegistrationError::Invalid);
    }

    #[cfg(unix)]
    if let Some(close) = native_metrics_close_v3() {
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

    unsafe extern "C" fn callback(_event: *const NativeMetricsEvent) -> i32 {
        NATIVE_FILTER_STATUS_OK
    }

    fn descriptor() -> NativeMetricsDescriptor {
        NativeMetricsDescriptor {
            magic: NATIVE_METRICS_API_MAGIC,
            version: NATIVE_METRICS_API_VERSION,
            reserved: 0,
            size: std::mem::size_of::<NativeMetricsDescriptor>() as u32,
            name: b"test".as_ptr(),
            name_len: 4,
            on_finish: Some(callback),
        }
    }

    #[test]
    fn accepts_only_complete_metric_descriptors() {
        let mut descriptor = descriptor();
        assert!(descriptor.is_compatible());

        descriptor.version = 0;
        assert!(!descriptor.is_compatible());
        descriptor.version = NATIVE_METRICS_API_VERSION;
        descriptor.size = 1;
        assert!(!descriptor.is_compatible());
        descriptor.size = std::mem::size_of::<NativeMetricsDescriptor>() as u32;
        descriptor.on_finish = None;
        assert!(!descriptor.is_compatible());
    }

    #[test]
    fn metric_event_is_versioned_and_length_delimited() {
        let name = b"haproxy.requests";
        let event = NativeMetricsEvent {
            magic: NATIVE_METRICS_API_MAGIC,
            version: NATIVE_METRICS_API_VERSION,
            size: std::mem::size_of::<NativeMetricsEvent>() as u32,
            kind: NATIVE_METRICS_EVENT_FINISH,
            name: NativeFilterBytes {
                data: name.as_ptr(),
                len: name.len(),
            },
            value: 1,
            ..NativeMetricsEvent::default()
        };

        assert_eq!(event.size as usize, std::mem::size_of_val(&event));
        assert_eq!(event.name.len, name.len());
        assert_eq!(event.value, 1);
    }

    #[test]
    fn v2_descriptor_is_separate_and_versioned() {
        unsafe extern "C" fn callback(_event: *const NativeMetricsEventV2) -> i32 {
            NATIVE_FILTER_STATUS_OK
        }
        let mut descriptor = NativeMetricsDescriptorV2 {
            magic: NATIVE_METRICS_API_MAGIC,
            version: NATIVE_METRICS_API_VERSION_V2,
            reserved: 0,
            size: std::mem::size_of::<NativeMetricsDescriptorV2>() as u32,
            name: b"v2".as_ptr(),
            name_len: 2,
            on_observation: Some(callback),
        };
        assert!(descriptor.is_compatible());
        descriptor.version = NATIVE_METRICS_API_VERSION;
        assert!(!descriptor.is_compatible());
        descriptor.version = NATIVE_METRICS_API_VERSION_V2;
        descriptor.reserved = 1;
        assert!(!descriptor.is_compatible());
        descriptor.reserved = 0;
        descriptor.name = std::ptr::null();
        assert!(!descriptor.is_compatible());
        descriptor.name = b"v2".as_ptr();
        descriptor.name_len = 0;
        assert!(!descriptor.is_compatible());
        descriptor.name_len = 2;
        descriptor.on_observation = None;
        assert!(!descriptor.is_compatible());
    }

    #[test]
    fn v2_observation_carries_bounded_labels_and_typed_values() {
        let event = NativeMetricsEventV2 {
            magic: NATIVE_METRICS_API_MAGIC,
            version: NATIVE_METRICS_API_VERSION_V2,
            size: std::mem::size_of::<NativeMetricsEventV2>() as u32,
            kind: NATIVE_METRICS_EVENT_OBSERVATION_V2,
            metric_type: NATIVE_METRIC_HISTOGRAM_V2,
            temporality: NATIVE_METRIC_TEMPORALITY_DELTA_V2,
            value_f64: 12.5,
            label_count: NATIVE_METRICS_MAX_LABELS_V2 as u16,
            ..NativeMetricsEventV2::default()
        };
        assert_eq!(event.size as usize, std::mem::size_of_val(&event));
        assert_eq!(event.label_count as usize, NATIVE_METRICS_MAX_LABELS_V2);
        assert_eq!(event.value_f64, 12.5);
    }

    #[test]
    fn v3_batch_descriptor_and_header_are_bounded() {
        assert_eq!(NATIVE_METRICS_MAX_EVENTS_V3, 13);
        unsafe extern "C" fn callback(_batch: *const NativeMetricsBatchV3) -> i32 {
            NATIVE_FILTER_STATUS_OK
        }
        let mut descriptor = NativeMetricsDescriptorV3 {
            magic: NATIVE_METRICS_API_MAGIC,
            version: NATIVE_METRICS_API_VERSION_V3,
            reserved: 0,
            size: std::mem::size_of::<NativeMetricsDescriptorV3>() as u32,
            name: b"v3".as_ptr(),
            name_len: 2,
            on_batch: Some(callback),
        };
        assert!(descriptor.is_compatible());
        descriptor.version = NATIVE_METRICS_API_VERSION_V2;
        assert!(!descriptor.is_compatible());

        let batch = NativeMetricsBatchV3 {
            magic: NATIVE_METRICS_API_MAGIC,
            version: NATIVE_METRICS_API_VERSION_V3,
            size: std::mem::size_of::<NativeMetricsBatchV3>() as u32,
            kind: NATIVE_METRICS_EVENT_BATCH_V3,
            event_count: NATIVE_METRICS_MAX_EVENTS_V3 as u16,
            ..NativeMetricsBatchV3::default()
        };
        assert_eq!(batch.size as usize, std::mem::size_of_val(&batch));
        assert_eq!(batch.event_count as usize, NATIVE_METRICS_MAX_EVENTS_V3);

        let snapshot = NativeMetricsBatchV3 {
            kind: NATIVE_METRICS_EVENT_SNAPSHOT_V3,
            ..batch
        };
        assert_eq!(snapshot.kind, NATIVE_METRICS_EVENT_SNAPSHOT_V3);
    }
}
