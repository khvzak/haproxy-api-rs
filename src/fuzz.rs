use std::cell::RefCell;
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr;

use mlua::{Lua, Table};

use crate::pairs::collect_pairs;
use crate::{
    NativeFilterDescriptor, NativeFilterEvent, NativeMetricsBatchV3, NativeMetricsDescriptor,
    NativeMetricsDescriptorV2, NativeMetricsDescriptorV3, NativeMetricsEvent, NativeMetricsEventV2,
    NativeModuleDescriptor, NATIVE_FILTER_API_MAGIC, NATIVE_FILTER_API_VERSION,
    NATIVE_METRICS_API_MAGIC, NATIVE_METRICS_API_VERSION, NATIVE_METRICS_API_VERSION_V2,
    NATIVE_METRICS_API_VERSION_V3, NATIVE_MODULE_API_MAGIC, NATIVE_MODULE_API_VERSION,
};

const NAME: &[u8] = b"fuzz";

thread_local! {
    static LUA: RefCell<Option<Lua>> = const { RefCell::new(None) };
}

unsafe extern "C" fn filter_callback(_: *const NativeFilterEvent, _: *mut *mut c_void) -> i32 {
    0
}

unsafe extern "C" fn filter_destroy(_: *mut c_void) {}

unsafe extern "C" fn metrics_callback(_: *const NativeMetricsEvent) -> i32 {
    0
}

unsafe extern "C" fn metrics_callback_v2(_: *const NativeMetricsEventV2) -> i32 {
    0
}

unsafe extern "C" fn metrics_callback_v3(_: *const NativeMetricsBatchV3) -> i32 {
    0
}

unsafe extern "C" fn module_init() -> i32 {
    0
}

unsafe extern "C" fn module_close() {}

fn word(data: &[u8], offset: usize) -> u32 {
    let mut bytes = [0; 4];
    for (destination, source) in bytes.iter_mut().zip(data.get(offset..).unwrap_or(&[])) {
        *destination = *source;
    }
    u32::from_le_bytes(bytes)
}

fn size(data: &[u8], offset: usize, valid: u32) -> u32 {
    if data.get(offset).copied().unwrap_or_default() & 1 == 0 {
        valid
    } else {
        word(data, offset + 1)
    }
}

fn u16_value(data: &[u8], offset: usize, valid: u16) -> u16 {
    if data.get(offset).copied().unwrap_or_default() & 1 == 0 {
        valid
    } else {
        data.get(offset + 1).copied().unwrap_or_default() as u16
    }
}

fn u32_value(data: &[u8], offset: usize, valid: u32) -> u32 {
    if data.get(offset).copied().unwrap_or_default() & 1 == 0 {
        valid
    } else {
        word(data, offset + 1)
    }
}

fn callback<T>(data: &[u8], offset: usize, value: T) -> Option<T> {
    (data.get(offset).copied().unwrap_or_default() & 1 != 0).then_some(value)
}

pub fn exercise_descriptors(data: &[u8]) {
    let valid_name = data.first().copied().unwrap_or_default() & 1 != 0;
    let name = if valid_name {
        NAME.as_ptr()
    } else {
        ptr::null()
    };
    let name_len = if valid_name {
        4
    } else {
        word(data, 5) as usize
    };

    let filter = NativeFilterDescriptor {
        magic: u32_value(data, 9, NATIVE_FILTER_API_MAGIC),
        version: u16_value(data, 13, NATIVE_FILTER_API_VERSION),
        reserved: u16_value(data, 15, 0),
        size: size(data, 17, size_of::<NativeFilterDescriptor>() as u32),
        name,
        name_len,
        on_request: callback(data, 19, filter_callback),
        on_response: callback(data, 20, filter_callback),
        on_finish: callback(data, 21, filter_callback),
        destroy: callback(data, 22, filter_destroy),
        peer_events: 0,
        flags: 0,
        lua_compat: None,
    };
    let _ = filter.is_compatible();

    let module = NativeModuleDescriptor {
        magic: u32_value(data, 23, NATIVE_MODULE_API_MAGIC),
        version: u16_value(data, 27, NATIVE_MODULE_API_VERSION),
        reserved: u16_value(data, 29, 0),
        size: size(data, 31, size_of::<NativeModuleDescriptor>() as u32),
        name,
        name_len,
        init: callback(data, 33, module_init),
        close: callback(data, 34, module_close),
    };
    let _ = module.is_compatible();

    let metrics = NativeMetricsDescriptor {
        magic: u32_value(data, 35, NATIVE_METRICS_API_MAGIC),
        version: u16_value(data, 39, NATIVE_METRICS_API_VERSION),
        reserved: u16_value(data, 41, 0),
        size: size(data, 43, size_of::<NativeMetricsDescriptor>() as u32),
        name,
        name_len,
        on_finish: callback(data, 45, metrics_callback),
    };
    let _ = metrics.is_compatible();

    let metrics_v2 = NativeMetricsDescriptorV2 {
        magic: u32_value(data, 45, NATIVE_METRICS_API_MAGIC),
        version: u16_value(data, 49, NATIVE_METRICS_API_VERSION_V2),
        reserved: u16_value(data, 51, 0),
        size: size(data, 53, size_of::<NativeMetricsDescriptorV2>() as u32),
        name,
        name_len,
        on_observation: callback(data, 55, metrics_callback_v2),
    };
    let _ = metrics_v2.is_compatible();

    let metrics_v3 = NativeMetricsDescriptorV3 {
        magic: u32_value(data, 57, NATIVE_METRICS_API_MAGIC),
        version: u16_value(data, 61, NATIVE_METRICS_API_VERSION_V3),
        reserved: u16_value(data, 63, 0),
        size: size(data, 65, size_of::<NativeMetricsDescriptorV3>() as u32),
        name,
        name_len,
        on_batch: callback(data, 67, metrics_callback_v3),
    };
    let _ = metrics_v3.is_compatible();
}

pub fn exercise_pairs(data: &[u8]) -> mlua::Result<()> {
    LUA.with(|state| {
        let mut state = state.borrow_mut();
        let lua = state.get_or_insert_with(Lua::new);
        let input = lua.create_string(&data[..data.len().min(64)])?;
        let table: Table = lua
            .load(
                r#"
                local input = ...
                local index = 0
                return setmetatable({}, {
                    __pairs = function()
                        return function()
                            index = index + 1
                            if index > #input then return nil end
                            local byte = input:byte(index)
                            local key = tostring(index)
                            if byte % 4 == 0 then return key, input:sub(index, index) end
                            if byte % 4 == 1 then return key, byte end
                            if byte % 4 == 2 then return byte, input:sub(index, index) end
                            return key, nil
                        end
                    end,
                })
                "#,
            )
            .call(&input)?;
        drop(collect_pairs::<String>(&table, lua));
        drop(table);
        drop(input);
        lua.gc_collect()
    })
}
