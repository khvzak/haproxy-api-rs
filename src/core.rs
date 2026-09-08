use std::collections::HashMap;
#[cfg(feature = "async")]
use std::future::Future;
use std::ops::Deref;

use mlua::{
    chunk::{AsChunk, Chunk},
    AnyUserData, FromLuaMulti, IntoLua, Lua, ObjectLike, Result, Table, Value,
};

use crate::filter::UserFilterWrapper;
use crate::pairs::collect_pairs;
use crate::{EventSub, Proxy, UserFilter};

/// It derefs to a Lua table, so you can use it as a Lua table directly.
#[derive(Clone)]
pub struct Core<'lua> {
    lua: &'lua Lua,
    class: Table,
}

#[derive(Debug, Copy, Clone)]
pub struct Time {
    pub sec: u64,
    pub usec: u64,
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
    TcpReq,
    TcpRes,
    HttpReq,
    HttpRes,
    HttpAfterRes,
}

impl Action {
    fn as_str(&self) -> &'static str {
        match self {
            Action::TcpReq => "tcp-req",
            Action::TcpRes => "tcp-res",
            Action::HttpReq => "http-req",
            Action::HttpRes => "http-res",
            Action::HttpAfterRes => "http-after-res",
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ServiceMode {
    Tcp,
    Http,
}

#[derive(Debug, Copy, Clone)]
pub enum LogLevel {
    Emerg,
    Alert,
    Crit,
    Err,
    Warning,
    Notice,
    Info,
    Debug,
}

impl<'lua> Core<'lua> {
    /// Creates new Core object using Lua global `core`
    #[inline]
    pub fn new(lua: &'lua Lua) -> Result<Self> {
        let class: Table = lua.globals().get("core")?;
        Ok(Core { lua, class })
    }

    /// Returns a map of declared proxies (frontends and backends), indexed by proxy name.
    #[inline]
    pub fn proxies(&self) -> Result<HashMap<String, Proxy>> {
        collect_pairs(&self.class.get("proxies")?, self.lua)
    }

    /// Returns a map of declared proxies with backend capability, indexed by the backend name.
    #[inline]
    pub fn backends(&self) -> Result<HashMap<String, Proxy>> {
        collect_pairs(&self.class.get("backends")?, self.lua)
    }

    /// Returns a map of declared proxies with frontend capability, indexed by the frontend name.
    #[inline]
    pub fn frontends(&self) -> Result<HashMap<String, Proxy>> {
        collect_pairs(&self.class.get("frontends")?, self.lua)
    }

    /// Returns the executing thread number starting at 0.
    /// If thread is 0, Lua scope is shared by all threads, otherwise the scope is dedicated to a single thread.
    /// This is HAProxy >=2.4 feature.
    #[inline]
    pub fn thread(&self) -> Result<u16> {
        self.class.get("thread")
    }

    /// Sends a log on the default syslog server if it is configured and on the stderr if it is allowed.
    #[inline]
    pub fn log(&self, level: LogLevel, msg: impl AsRef<str>) -> Result<()> {
        let msg = msg.as_ref();
        self.class.call_function("log", (level, msg))
    }

    /// Adds the ACL `key` in the ACLs list referenced by `filename`.
    #[inline]
    pub fn add_acl(&self, filename: &str, key: &str) -> Result<()> {
        self.class.call_function("add_acl", (filename, key))
    }

    /// Deletes the ACL entry by `key` in the ACLs list referenced by `filename`.
    #[inline]
    pub fn del_acl(&self, filename: &str, key: &str) -> Result<()> {
        self.class.call_function("del_acl", (filename, key))
    }

    /// Deletes the map entry indexed with the specified `key` in the list of maps
    /// referenced by his `filename`.
    #[inline]
    pub fn del_map(&self, filename: &str, key: &str) -> Result<()> {
        self.class.call_function("del_map", (filename, key))
    }

    /// Sets the `value` associated to the `key` in the map referenced by `filename`.
    #[inline]
    pub fn set_map(&self, filename: &str, key: &str, value: &str) -> Result<()> {
        self.class.call_function("set_map", (filename, key, value))
    }

    /// Returns HAProxy core information (uptime, pid, memory pool usage, tasks number, ...).
    #[inline]
    pub fn get_info(&self) -> Result<Table> {
        self.class.call_function("get_info", ())
    }

    /// Returns the current time.
    /// The time returned is fixed by the HAProxy core and assures than the hour will be monotonic
    /// and that the system call `gettimeofday` will not be called too.
    #[inline]
    pub fn now(&self) -> Result<Time> {
        let time: Table = self.class.call_function("now", ())?;
        Ok(Time {
            sec: time.get("sec")?,
            usec: time.get("usec")?,
        })
    }

    /// Registers a function executed as an action.
    /// The expected actions are `tcp-req`, `tcp-res`, `http-req`, `http-res` or `http-after-res`.
    /// All the registered actions can be used in HAProxy with the prefix `lua.`.
    pub fn register_action<F, A>(
        &self,
        name: &str,
        actions: &[Action],
        nb_args: usize,
        func: F,
    ) -> Result<()>
    where
        F: Fn(&Lua, A) -> Result<()> + Send + 'static,
        A: FromLuaMulti,
    {
        let func = self.lua.create_function(func)?;
        let actions = actions.iter().map(|act| act.as_str()).collect::<Vec<_>>();
        self.class
            .call_function("register_action", (name, actions, func, nb_args))
    }

    /// Registers an asynchronous function executed as an action.
    ///
    /// See [`Core::register_action`] for more details.
    #[cfg(feature = "async")]
    pub fn register_async_action<F, A, FR>(
        &self,
        name: &str,
        actions: &[Action],
        nb_args: usize,
        func: F,
    ) -> Result<()>
    where
        F: Fn(A) -> FR + 'static,
        A: FromLuaMulti + 'static,
        FR: Future<Output = Result<()>> + Send + 'static,
    {
        let func = crate::r#async::create_async_function(self.lua, func)?;
        let actions = actions.iter().map(|act| act.as_str()).collect::<Vec<_>>();
        self.class
            .call_function("register_action", (name, actions, func, nb_args))
    }

    /// Same as [`register_action`] but using Lua function.
    ///
    /// [`register_action`]: #method.register_action
    pub fn register_lua_action(
        &self,
        name: &str,
        actions: &[&str],
        nb_args: usize,
        code: impl AsChunk,
    ) -> Result<()> {
        let func = self.lua.load(code).into_function()?;
        self.class
            .call_function("register_action", (name, actions.to_vec(), func, nb_args))
    }

    /// Registers a function executed as a converter.
    /// All the registered converters can be used in HAProxy with the prefix `lua.`.
    pub fn register_converters<F, A, R>(&self, name: &str, func: F) -> Result<()>
    where
        F: Fn(&Lua, A) -> Result<R> + Send + 'static,
        A: FromLuaMulti,
        R: IntoLua,
    {
        let func = self.lua.create_function(func)?;
        self.class
            .call_function("register_converters", (name, func))
    }

    /// Same as [`register_converters`] but using Lua function.
    ///
    /// [`register_converters`]: #method.register_converters
    pub fn register_lua_converters(&self, name: &str, code: impl AsChunk) -> Result<()> {
        let func = self.lua.load(code).into_function()?;
        self.class
            .call_function("register_converters", (name, func))
    }

    /// Registers a function executed as sample fetch.
    /// All the registered sample fetch can be used in HAProxy with the prefix `lua.`.
    pub fn register_fetches<F, A, R>(&self, name: &str, func: F) -> Result<()>
    where
        F: Fn(&Lua, A) -> Result<R> + Send + 'static,
        A: FromLuaMulti,
        R: IntoLua,
    {
        let func = self.lua.create_function(func)?;
        self.class.call_function("register_fetches", (name, func))
    }

    /// Same as [`register_fetches`] but using Lua function.
    ///
    /// [`register_fetches`]: #method.register_fetches
    pub fn register_lua_fetches(&self, name: &str, code: impl AsChunk) -> Result<()> {
        let func = self.lua.load(code).into_function()?;
        self.class.call_function("register_fetches", (name, func))
    }

    /// Registers a custom filter that implements [`UserFilter`] trait.
    pub fn register_filter<T: UserFilter + 'static>(&self, name: &str) -> Result<()> {
        let lua = self.lua;
        let func = lua.create_function(|_, (class, args): (Table, Table)| {
            class.raw_set("args", args)?;
            Ok(class)
        });
        let filter_class = UserFilterWrapper::<T>::make_class(lua)?;
        self.class
            .call_function("register_filter", (name, filter_class, func))
    }

    /// Registers a Lua function executed as a service.
    /// All the registered service can be used in HAProxy with the prefix `lua.`.
    pub fn register_lua_service(
        &self,
        name: &str,
        mode: ServiceMode,
        code: impl AsChunk,
    ) -> Result<()> {
        let func = self.lua.load(code).into_function()?;
        let mode = match mode {
            ServiceMode::Tcp => "tcp",
            ServiceMode::Http => "http",
        };
        self.class
            .call_function("register_service", (name, mode, func))
    }

    /// Registers a function executed after the configuration parsing.
    /// This is useful to check any parameters.
    pub fn register_init<F>(&self, func: F) -> Result<()>
    where
        F: Fn(&Lua) -> Result<()> + Send + 'static,
    {
        let func = self.lua.create_function(move |lua, ()| func(lua))?;
        self.class.call_function("register_init", func)
    }

    /// Registers and start an independent task.
    /// The task is started when the HAProxy main scheduler starts.
    pub fn register_task<F>(&self, func: F) -> Result<()>
    where
        F: Fn(&Lua) -> Result<()> + Send + 'static,
    {
        let func = self.lua.create_function(move |lua, ()| func(lua))?;
        self.class.call_function("register_task", func)
    }

    /// Registers and start an independent asynchronous task.
    #[cfg(feature = "async")]
    pub fn register_async_task<F, FR>(&self, func: F) -> Result<()>
    where
        F: Fn() -> FR + 'static,
        FR: Future<Output = Result<()>> + Send + 'static,
    {
        let func = crate::r#async::create_async_function(self.lua, move |()| func())?;
        self.class.call_function("register_task", func)
    }

    /// Same as [`register_task`] but using Lua function.
    ///
    /// [`register_task`]: #method.register_task
    pub fn register_lua_task(&self, code: impl AsChunk) -> Result<()> {
        let func = self.lua.load(code).into_function()?;
        self.class.call_function("register_task", func)
    }

    /// Registers a Lua function executed as a cli command.
    pub fn register_lua_cli(&self, path: &[&str], usage: &str, code: impl AsChunk) -> Result<()> {
        let func = self.lua.load(code).into_function()?;
        self.class
            .call_function("register_cli", (path, usage, func))
    }

    /// Changes the nice of the current task or current session.
    #[inline]
    pub fn set_nice(&self, nice: i32) -> Result<()> {
        self.class.call_function("set_nice", nice)
    }

    /// Parses ipv4 or ipv6 addresses and its facultative associated network.
    #[inline]
    pub fn parse_addr(&self, addr: &str) -> Result<Option<AnyUserData>> {
        self.class.call_function("parse_addr", addr)
    }

    /// Matches two networks.
    /// For example "127.0.0.1/32" matches "127.0.0.0/8". The order of network is not important.
    #[inline]
    pub fn match_addr(&self, addr1: AnyUserData, addr2: AnyUserData) -> Result<bool> {
        self.class.call_function("match_addr", (addr1, addr2))
    }

    /// Register a Lua function that will be called on specific system events.
    pub fn event_sub(&self, event_types: &[&str], code: impl AsChunk) -> Result<EventSub> {
        (self.class).call_function("event_sub", (event_types, Chunk::wrap(code)))
    }
}

impl Deref for Core<'_> {
    type Target = Table;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.class
    }
}

impl IntoLua for LogLevel {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        (match self {
            LogLevel::Emerg => 0,
            LogLevel::Alert => 1,
            LogLevel::Crit => 2,
            LogLevel::Err => 3,
            LogLevel::Warning => 4,
            LogLevel::Notice => 5,
            LogLevel::Info => 6,
            LogLevel::Debug => 7,
        })
        .into_lua(lua)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_core_and_proxy_lists_use_pairs_metamethods() {
        let lua = Lua::new();
        let core_class: Table = lua
            .load(
                r#"
                local function list(key, value)
                    local done = false
                    return setmetatable({}, {
                        __pairs = function()
                            return function()
                                if done then return nil end
                                done = true
                                return key, value
                            end
                        end,
                    })
                end
                local proxy = {
                    servers = list("server-a", {}),
                    listeners = list("listener-a", {}),
                }
                return {
                    proxies = list("proxy-a", proxy),
                    backends = list("backend-a", proxy),
                    frontends = list("frontend-a", proxy),
                }
                "#,
            )
            .eval()
            .unwrap();
        lua.globals().set("core", core_class).unwrap();

        let core = Core::new(&lua).unwrap();
        let proxies = core.proxies().unwrap();
        let backends = core.backends().unwrap();
        let frontends = core.frontends().unwrap();
        let proxy = proxies.get("proxy-a").unwrap();

        assert!(backends.contains_key("backend-a"));
        assert!(frontends.contains_key("frontend-a"));
        assert!(proxy.get_servers().unwrap().contains_key("server-a"));
        assert!(proxy.get_listeners().unwrap().contains_key("listener-a"));
    }

    #[test]
    fn parse_addr_invalid_input_is_none() {
        let lua = Lua::new();
        let core = lua.create_table().unwrap();
        core.set(
            "parse_addr",
            lua.create_function(|_, _: String| Ok::<Option<AnyUserData>, mlua::Error>(None))
                .unwrap(),
        )
        .unwrap();

        assert!(Core {
            lua: &lua,
            class: core
        }
        .parse_addr("not-an-address")
        .unwrap()
        .is_none());
    }
}
