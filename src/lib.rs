//! # HAProxy 2.x Lua API
//!
//! Intended to be used together with [mlua] in a module mode.
//!
//! Please see the [Lua API] documentation for details.
//!
//! [Lua API]: http://www.arpalert.org/src/haproxy-lua-api/2.2/index.html
//! [mlua]: https://crates.io/crates/mlua

mod channel;
mod converters;
mod core;
mod fetches;
mod filter;
mod http;
mod http_message;
mod listener;
mod proxy;
mod server;
mod stick_table;
mod txn;

pub use crate::channel::Channel;
pub use crate::converters::Converters;
pub use crate::core::{Action, Core, LogLevel, ServiceMode, Time};
pub use crate::fetches::Fetches;
pub use crate::filter::{FilterMethod, FilterResult, UserFilter};
pub use crate::http::{Headers, Http};
pub use crate::http_message::HttpMessage;
pub use crate::proxy::Proxy;
pub use crate::server::Server;
pub use crate::stick_table::StickTable;
pub use crate::txn::Txn;

#[cfg(feature = "async")]
pub use crate::core::create_async_function;
