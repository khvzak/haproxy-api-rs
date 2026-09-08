//! # HAProxy 2.x Lua API
//!
//! Intended to be used together with [mlua] in a module mode.
//!
//! Please see the [Lua API] documentation for details.
//!
//! [Lua API]: http://www.arpalert.org/src/haproxy-lua-api/2.2/index.html
//! [mlua]: https://crates.io/crates/mlua

#[cfg(feature = "async")]
mod r#async;
#[cfg(feature = "async")]
mod async_pool;
#[cfg(feature = "async")]
mod async_pool_lua;
#[cfg(feature = "lua")]
mod channel;
#[cfg(feature = "lua")]
mod converters;
#[cfg(feature = "lua")]
mod core;
#[cfg(feature = "lua")]
mod event_sub;
#[cfg(feature = "lua")]
mod fetches;
#[cfg(feature = "lua")]
mod filter;
#[cfg(feature = "fuzzing")]
pub mod fuzz;
#[cfg(feature = "lua")]
mod http;
#[cfg(feature = "lua")]
mod http_message;
#[cfg(feature = "lua")]
mod listener;
#[cfg(any(feature = "lua", feature = "native"))]
mod native_filter;
#[cfg(any(feature = "lua", feature = "native"))]
mod native_metrics;
#[cfg(any(feature = "lua", feature = "native"))]
mod native_module;
#[cfg(feature = "lua")]
mod pairs;
#[cfg(feature = "lua")]
mod proxy;
#[cfg(feature = "runtime")]
mod runtime;
#[cfg(feature = "lua")]
mod server;
#[cfg(feature = "lua")]
mod stick_table;
#[cfg(feature = "lua")]
mod txn;

#[cfg(feature = "lua")]
pub use crate::channel::Channel;
#[cfg(feature = "lua")]
pub use crate::converters::Converters;
#[cfg(feature = "lua")]
pub use crate::core::{Action, Core, LogLevel, ServiceMode, Time};
#[cfg(feature = "lua")]
pub use crate::event_sub::EventSub;
#[cfg(feature = "lua")]
pub use crate::fetches::{Fetches, RequestHeaders};
#[cfg(feature = "lua")]
pub use crate::filter::{FilterMethod, FilterResult, TxnFields, UserFilter};
#[cfg(feature = "lua")]
pub use crate::http::{Headers, Http};
#[cfg(feature = "lua")]
pub use crate::http_message::HttpMessage;
#[cfg(any(feature = "lua", feature = "native"))]
pub use crate::native_filter::{
    close_native_filter, native_filter_lua_compat_available, register_native_filter,
    set_native_filter_lua_attribute, NativeFilterBytes, NativeFilterCallback,
    NativeFilterDescriptor, NativeFilterDestroy, NativeFilterEvent, NativeFilterHeader,
    NativeFilterLuaAttribute, NativeFilterLuaCompat, NativeFilterRegistrationError,
    NativeFilterSetHeader, NATIVE_FILTER_API_MAGIC, NATIVE_FILTER_API_VERSION,
    NATIVE_FILTER_EVENT_FINISH, NATIVE_FILTER_EVENT_REQUEST_HEADERS,
    NATIVE_FILTER_EVENT_RESPONSE_HEADERS, NATIVE_FILTER_FLAG_LUA_COMPAT, NATIVE_FILTER_PEER_FINISH,
    NATIVE_FILTER_PEER_REQUEST, NATIVE_FILTER_PEER_RESPONSE, NATIVE_FILTER_STATUS_BUSY,
    NATIVE_FILTER_STATUS_CONFLICT, NATIVE_FILTER_STATUS_INVALID, NATIVE_FILTER_STATUS_OK,
};
#[cfg(any(feature = "lua", feature = "native"))]
pub use crate::native_metrics::{
    close_native_metrics, close_native_metrics_v2, close_native_metrics_v3,
    register_native_metrics, register_native_metrics_v2, register_native_metrics_v3,
    NativeMetricsBatchV3, NativeMetricsCallback, NativeMetricsCallbackV2, NativeMetricsCallbackV3,
    NativeMetricsDescriptor, NativeMetricsDescriptorV2, NativeMetricsDescriptorV3,
    NativeMetricsEvent, NativeMetricsEventV2, NativeMetricsLabelV2, NATIVE_METRICS_API_MAGIC,
    NATIVE_METRICS_API_VERSION, NATIVE_METRICS_API_VERSION_V2, NATIVE_METRICS_API_VERSION_V3,
    NATIVE_METRICS_EVENT_BATCH_V3, NATIVE_METRICS_EVENT_FINISH,
    NATIVE_METRICS_EVENT_OBSERVATION_V2, NATIVE_METRICS_EVENT_SNAPSHOT_V3,
    NATIVE_METRICS_MAX_EVENTS_V3, NATIVE_METRICS_MAX_LABELS_V2, NATIVE_METRIC_COUNTER_V2,
    NATIVE_METRIC_GAUGE_V2, NATIVE_METRIC_HISTOGRAM_V2, NATIVE_METRIC_TEMPORALITY_CUMULATIVE_V2,
    NATIVE_METRIC_TEMPORALITY_DELTA_V2,
};
#[cfg(any(feature = "lua", feature = "native"))]
pub use crate::native_module::{
    native_module_descriptor_symbol, NativeModuleClose, NativeModuleDescriptor,
    NativeModuleGetDescriptor, NativeModuleInit, NATIVE_MODULE_API_MAGIC,
    NATIVE_MODULE_API_VERSION, NATIVE_MODULE_STATUS_ERROR, NATIVE_MODULE_STATUS_OK,
};
#[cfg(feature = "lua")]
pub use crate::proxy::Proxy;
#[cfg(feature = "lua")]
pub use crate::server::Server;
#[cfg(feature = "lua")]
pub use crate::stick_table::StickTable;
#[cfg(feature = "lua")]
pub use crate::txn::{NativeTxnSlotError, Txn};

#[cfg(feature = "async")]
pub use crate::r#async::{create_async_function, runtime};
#[cfg(all(feature = "runtime", not(feature = "async")))]
pub use crate::runtime::runtime;
