use std::future::Future;

use mlua::{
    ExternalError, FromLua, FromLuaMulti, Function, Lua, Result, Table, TableExt, ToLua, Value,
};

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
    // TODO: add_acl
    // TODO: del_acl
    // TODO: del_map

    pub fn new(lua: &'lua Lua) -> Result<Self> {
        let class: Table = lua.globals().get("core")?;
        Ok(Core { lua, class })
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
        date.ok_or("invalid date".to_lua_err())
    }

    pub fn imf_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.class.call_function("imf_date", date)?;
        date.ok_or("invalid date".to_lua_err())
    }

    pub fn rfc850_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.class.call_function("rfc850_date", date)?;
        date.ok_or("invalid date".to_lua_err())
    }

    pub fn asctime_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.class.call_function("asctime_date", date)?;
        date.ok_or("invalid date".to_lua_err())
    }

    pub fn msleep(&self, milliseconds: u64) -> Result<()> {
        self.class.call_function("msleep", milliseconds)
    }

    // TODO: proxies
    // TODO: backends
    // TODO: frontends

    pub fn register_action<'callback, A, F>(
        &self,
        name: &str,
        actions: &[&str],
        func: F,
        nb_args: usize,
    ) -> Result<()>
    where
        A: FromLuaMulti<'callback>,
        F: Fn(&'callback Lua, A) -> Result<()> + 'static,
    {
        let func = self.lua.create_function(func)?;
        self.class
            .call_function("register_action", (name, actions.to_vec(), func, nb_args))
    }

    pub fn register_async_action<'callback, A, F, FR>(
        &self,
        name: &str,
        actions: &[&str],
        func: F,
        nb_args: usize,
    ) -> Result<()>
    where
        A: FromLuaMulti<'callback>,
        F: Fn(&'callback Lua, A) -> FR + 'static,
        FR: Future<Output = Result<()>> + 'static,
    {
        let _yield_fixup = YieldFixUp::new(self.lua)?;
        let func = self.lua.create_async_function(func)?;
        self.class
            .call_function("register_action", (name, actions.to_vec(), func, nb_args))
    }

    pub fn register_converters<'callback, A, R, F>(&self, name: &str, func: F) -> Result<()>
    where
        A: FromLuaMulti<'callback>,
        R: ToLua<'callback>,
        F: Fn(&'callback Lua, A) -> Result<R> + 'static,
    {
        let func = self.lua.create_function(func)?;
        self.class
            .call_function("register_converters", (name, func))
    }

    pub fn register_fetches<'callback, A, R, F>(&self, name: &str, func: F) -> Result<()>
    where
        A: FromLuaMulti<'callback>,
        R: ToLua<'callback>,
        F: Fn(&'callback Lua, A) -> Result<R> + 'static,
    {
        let func = self.lua.create_function(func)?;
        self.class.call_function("register_fetches", (name, func))
    }

    pub fn register_async_fetches<'callback, A, R, F, FR>(&self, name: &str, func: F) -> Result<()>
    where
        A: FromLuaMulti<'callback>,
        R: ToLua<'callback>,
        F: Fn(&'callback Lua, A) -> FR + 'static,
        FR: Future<Output = Result<R>> + 'static,
    {
        let _yield_fixup = YieldFixUp::new(self.lua)?;
        let func = self.lua.create_async_function(func)?;
        self.class.call_function("register_fetches", (name, func))
    }

    pub fn register_service<'callback, A, F>(
        &self,
        name: &str,
        mode: ServiceMode,
        func: F,
    ) -> Result<()>
    where
        A: FromLua<'callback>,
        F: Fn(&'callback Lua, A) -> Result<()> + 'static,
    {
        let func = self.lua.create_function(func)?;
        let mode = match mode {
            ServiceMode::Tcp => "tcp",
            ServiceMode::Http => "http",
        };
        self.class
            .call_function("register_service", (name, mode, func))
    }

    pub fn register_async_service<'callback, A, F, FR>(
        &self,
        name: &str,
        mode: ServiceMode,
        func: F,
    ) -> Result<()>
    where
        A: FromLua<'callback>,
        F: Fn(&'callback Lua, A) -> FR + 'static,
        FR: Future<Output = Result<()>> + 'static,
    {
        let _yield_fixup = YieldFixUp::new(self.lua)?;
        let func = self.lua.create_async_function(func)?;
        let mode = match mode {
            ServiceMode::Tcp => "tcp",
            ServiceMode::Http => "http",
        };
        self.class
            .call_function("register_service", (name, mode, func))
    }

    pub fn register_init<'callback, F>(&self, func: F) -> Result<()>
    where
        F: Fn(&'callback Lua) -> Result<()> + 'static,
    {
        let func = self.lua.create_function(move |lua, ()| func(lua))?;
        self.class.call_function("register_init", func)
    }

    pub fn register_task<'callback, F>(&self, func: F) -> Result<()>
    where
        F: Fn(&'callback Lua) -> Result<()> + 'static,
    {
        let func = self.lua.create_function(move |lua, ()| func(lua))?;
        self.class.call_function("register_task", func)
    }

    pub fn register_async_task<'callback, F, FR>(&self, func: F) -> Result<()>
    where
        F: Fn(&'callback Lua) -> FR + 'static,
        FR: Future<Output = Result<()>> + 'static,
    {
        let _yield_fixup = YieldFixUp::new(self.lua)?;
        let func = self.lua.create_async_function(move |lua, ()| func(lua))?;
        self.class.call_function("register_task", func)
    }

    // TODO: register_cli

    pub fn set_nice(&self, nice: i32) -> Result<()> {
        self.class.call_function("set_nice", nice)
    }

    pub fn set_map(&self, filename: &str, key: &str, value: &str) -> Result<()> {
        self.class.call_function("set_map", (filename, key, value))
    }

    pub fn sleep(&self, seconds: usize) -> Result<()> {
        self.class.call_function("sleep", seconds)
    }

    // TODO: tcp
    // TODO: parse_addr
    // TODO: match_addr
    // TODO: tokenize
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
