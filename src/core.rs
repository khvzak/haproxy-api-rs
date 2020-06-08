use std::collections::HashMap;
use std::future::Future;

use mlua::{
    ExternalError, FromLuaMulti, Function, Lua, Result, Table, TableExt, ToLua, ToLuaMulti, Value,
};

use crate::Proxy;

#[derive(Clone)]
pub struct Core<'lua> {
    lua: &'lua Lua,
    class: Table<'lua>,
}

#[derive(Debug, Clone)]
pub struct Time {
    pub sec: u64,
    pub usec: u64,
}

#[derive(Debug)]
pub enum ServiceMode {
    Tcp,
    Http,
}

#[derive(Debug)]
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
    pub fn new(lua: &'lua Lua) -> Result<Self> {
        let class: Table = lua.globals().get("core")?;
        Ok(Core { lua, class })
    }

    pub fn proxies(&self) -> Result<HashMap<String, Proxy>> {
        self.class.get("proxies")
    }

    pub fn backends(&self) -> Result<HashMap<String, Proxy>> {
        self.class.get("backends")
    }

    pub fn frontends(&self) -> Result<HashMap<String, Proxy>> {
        self.class.get("frontends")
    }

    pub fn log<S: AsRef<str>>(&self, level: LogLevel, msg: S) -> Result<()> {
        let msg = msg.as_ref();
        self.class.call_function("log", (level, msg))
    }

    pub fn get_info(&self) -> Result<Vec<String>> {
        self.class.call_function("get_info", ())
    }

    pub fn now(&self) -> Result<Time> {
        let time: Table = self.class.call_function("now", ())?;
        Ok(Time {
            sec: time.get("sec")?,
            usec: time.get("usec")?,
        })
    }

    pub fn http_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.class.call_function("http_date", date)?;
        date.ok_or_else(|| "invalid date".to_lua_err())
    }

    pub fn imf_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.class.call_function("imf_date", date)?;
        date.ok_or_else(|| "invalid date".to_lua_err())
    }

    pub fn rfc850_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.class.call_function("rfc850_date", date)?;
        date.ok_or_else(|| "invalid date".to_lua_err())
    }

    pub fn asctime_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.class.call_function("asctime_date", date)?;
        date.ok_or_else(|| "invalid date".to_lua_err())
    }

    pub fn register_action<A, F>(
        &self,
        name: &str,
        actions: &[&str],
        func: F,
        nb_args: usize,
    ) -> Result<()>
    where
        A: FromLuaMulti<'lua>,
        F: Fn(&'lua Lua, A) -> Result<()> + Send + 'static,
    {
        let func = self.lua.create_function(func)?;
        self.class
            .call_function("register_action", (name, actions.to_vec(), func, nb_args))
    }

    pub fn register_async_action<A, F, FR>(
        &self,
        name: &str,
        actions: &[&str],
        func: F,
        nb_args: usize,
    ) -> Result<()>
    where
        A: FromLuaMulti<'lua>,
        F: Fn(&'lua Lua, A) -> FR + Send + 'static,
        FR: Future<Output = Result<()>> + 'static,
    {
        let _yield_fixup = YieldFixUp::new(self.lua)?;
        let func = self.lua.create_async_function(func)?;
        self.class
            .call_function("register_action", (name, actions.to_vec(), func, nb_args))
    }

    pub fn register_lua_action<S>(&self, name: &str, actions: &[&str], code: &S, nb_args: usize,) -> Result<()>
    where
        S: AsRef<[u8]> + ?Sized,
    {
        let func = self.lua.load(code).into_function()?;
        self.class.call_function("register_action", (name, actions.to_vec(), func, nb_args))
    }

    pub fn register_converters<A, R, F>(&self, name: &str, func: F) -> Result<()>
    where
        A: FromLuaMulti<'lua>,
        R: ToLua<'lua>,
        F: Fn(&'lua Lua, A) -> Result<R> + Send + 'static,
    {
        let func = self.lua.create_function(func)?;
        self.class
            .call_function("register_converters", (name, func))
    }

    pub fn register_async_converters<A, R, F, FR>(
        &self,
        name: &str,
        func: F,
    ) -> Result<()>
    where
        A: FromLuaMulti<'lua>,
        R: ToLua<'lua>,
        F: Fn(&'lua Lua, A) -> FR + Send + 'static,
        FR: Future<Output = Result<R>> + 'static,
    {
        let _yield_fixup = YieldFixUp::new(self.lua)?;
        let func = self.lua.create_async_function(func)?;
        self.class
            .call_function("register_converters", (name, func))
    }

    pub fn register_lua_converters<S>(&self, name: &str, code: &S) -> Result<()>
    where
        S: AsRef<[u8]> + ?Sized,
    {
        let func = self.lua.load(code).into_function()?;
        self.class.call_function("register_converters", (name, func))
    }

    pub fn register_fetches<A, R, F>(&self, name: &str, func: F) -> Result<()>
    where
        A: FromLuaMulti<'lua>,
        R: ToLua<'lua>,
        F: Fn(&'lua Lua, A) -> Result<R> + Send + 'static,
    {
        let func = self.lua.create_function(func)?;
        self.class.call_function("register_fetches", (name, func))
    }

    pub fn register_async_fetches<A, R, F, FR>(&self, name: &str, func: F) -> Result<()>
    where
        A: FromLuaMulti<'lua>,
        R: ToLua<'lua>,
        F: Fn(&'lua Lua, A) -> FR + Send + 'static,
        FR: Future<Output = Result<R>> + 'static,
    {
        let _yield_fixup = YieldFixUp::new(self.lua)?;
        let func = self.lua.create_async_function(func)?;
        self.class.call_function("register_fetches", (name, func))
    }

    pub fn register_lua_fetches<S>(&self, name: &str, code: &S) -> Result<()>
    where
        S: AsRef<[u8]> + ?Sized,
    {
        let func = self.lua.load(code).into_function()?;
        self.class.call_function("register_fetches", (name, func))
    }

    pub fn register_lua_service<S>(&self, name: &str, mode: ServiceMode, code: &S) -> Result<()>
    where
        S: AsRef<[u8]> + ?Sized,
    {
        let func = self.lua.load(code).into_function()?;
        let mode = match mode {
            ServiceMode::Tcp => "tcp",
            ServiceMode::Http => "http",
        };
        self.class.call_function("register_service", (name, mode, func))
    }

    pub fn register_init<F>(&self, func: F) -> Result<()>
    where
        F: Fn(&'lua Lua) -> Result<()> + Send + 'static,
    {
        let func = self.lua.create_function(move |lua, ()| func(lua))?;
        self.class.call_function("register_init", func)
    }

    pub fn register_task<F>(&self, func: F) -> Result<()>
    where
        F: Fn(&'lua Lua) -> Result<()> + Send + 'static,
    {
        let func = self.lua.create_function(move |lua, ()| func(lua))?;
        self.class.call_function("register_task", func)
    }

    pub fn register_async_task<F, FR>(&self, func: F) -> Result<()>
    where
        F: Fn(&'lua Lua) -> FR + Send + 'static,
        FR: Future<Output = Result<()>> + 'static,
    {
        let _yield_fixup = YieldFixUp::new(self.lua)?;
        let func = self.lua.create_async_function(move |lua, ()| func(lua))?;
        self.class.call_function("register_task", func)
    }

    pub fn register_lua_task<S>(&self, code: &S) -> Result<()>
    where
        S: AsRef<[u8]> + ?Sized,
    {
        let func = self.lua.load(code).into_function()?;
        self.class.call_function("register_task", func)
    }

    // TODO: register_cli

    pub fn set_nice(&self, nice: i32) -> Result<()> {
        self.class.call_function("set_nice", nice)
    }

    pub fn add_acl(&self, filename: &str, key: &str) -> Result<()> {
        self.class.call_function("add_acl", (filename, key))
    }

    pub fn del_acl(&self, filename: &str, key: &str) -> Result<()> {
        self.class.call_function("del_acl", (filename, key))
    }

    pub fn del_map(&self, filename: &str, key: &str) -> Result<()> {
        self.class.call_function("del_map", (filename, key))
    }

    pub fn set_map(&self, filename: &str, key: &str, value: &str) -> Result<()> {
        self.class.call_function("set_map", (filename, key, value))
    }

    // SKIP: parse_addr/match_addr/tokenize
}

impl<'lua> ToLua<'lua> for LogLevel {
    fn to_lua(self, lua: &'lua Lua) -> Result<Value<'lua>> {
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
        .to_lua(lua)
    }
}

pub fn create_async_function<'lua, A, R, F, FR>(
    lua: &'lua Lua,
    func: F,
) -> Result<Function<'lua>>
where
    A: FromLuaMulti<'lua>,
    R: ToLuaMulti<'lua>,
    F: 'static + Send + Fn(&'lua Lua, A) -> FR,
    FR: 'lua + Future<Output = Result<R>>,
{
    let _yield_fixup = YieldFixUp::new(lua)?;
    lua.create_async_function(func)
}

struct YieldFixUp<'lua>(&'lua Lua, Function<'lua>);

impl<'lua> YieldFixUp<'lua> {
    fn new(lua: &'lua Lua) -> Result<Self> {
        let coroutine: Table = lua.globals().get("coroutine")?;
        let orig_yield: Function = coroutine.get("yield")?;
        let core: Table = lua.globals().get("core")?;
        coroutine.set("yield", core.get::<_, Function>("yield")?)?;
        Ok(YieldFixUp(lua, orig_yield))
    }
}

impl<'lua> Drop for YieldFixUp<'lua> {
    fn drop(&mut self) {
        if let Err(e) = (|| {
            let coroutine: Table = self.0.globals().get("coroutine")?;
            coroutine.set("yield", self.1.clone())
        })() {
            println!("Error in YieldFixUp destructor: {}", e);
        }
    }
}
