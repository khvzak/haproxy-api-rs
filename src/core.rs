use mlua::{
    ExternalError, FromLua, FromLuaMulti, Lua, MultiValue, Result, Table, TableExt, ToLua, Value,
};

// use crate::txn::Txn;

pub struct Core<'lua>(&'lua Lua, Table<'lua>);

#[derive(Debug)]
pub struct Time {
    pub sec: u64,
    pub usec: u64,
}

pub enum ServiceMode {
    Tcp,
    Http,
}

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
        let core: Table = lua.globals().get("core")?;
        Ok(Core(lua, core))
    }

    pub fn log<S: AsRef<str>>(&self, level: LogLevel, msg: S) -> Result<()> {
        let msg = msg.as_ref();
        self.1.call_function("log", (level, msg))
    }

    pub fn get_info(&self) -> Result<Vec<String>> {
        self.1.call_function("get_info", ())
    }

    pub fn now(&self) -> Result<Time> {
        let time: Table = self.1.call_function("now", ())?;
        Ok(Time {
            sec: time.get("sec")?,
            usec: time.get("usec")?,
        })
    }

    pub fn http_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.1.call_function("http_date", date)?;
        date.ok_or("invalid date".to_lua_err())
    }

    pub fn imf_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.1.call_function("imf_date", date)?;
        date.ok_or("invalid date".to_lua_err())
    }

    pub fn rfc850_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.1.call_function("rfc850_date", date)?;
        date.ok_or("invalid date".to_lua_err())
    }

    pub fn asctime_date(&self, date: &str) -> Result<u64> {
        let date: Option<u64> = self.1.call_function("asctime_date", date)?;
        date.ok_or("invalid date".to_lua_err())
    }

    pub fn msleep(&self, milliseconds: u64) -> Result<()> {
        self.1.call_function("msleep", milliseconds)
    }

    // TODO: proxies
    // TODO: backends
    // TODO: frontends

    pub fn register_action<'callback, A, F>(
        &self,
        name: &str,
        actions: &[&str],
        func: F,
        nb_args: Option<usize>,
    ) -> Result<()>
    where
        A: FromLuaMulti<'callback>,
        F: Fn(&'callback Lua, Core, Table, A) -> Result<()> + 'static,
    {
        let func = self.0.create_function(move |lua, args: MultiValue| {
            let mut args = args.into_vec();
            args.reverse();
            let txn: Table = Table::from_lua(args.pop().expect("txn expected"), lua)?;
            args.reverse();
            let args = MultiValue::from_vec(args);

            func(lua, Core::new(lua)?, txn, A::from_lua_multi(args, lua)?)
        })?;

        self.1
            .call_function("register_action", (name, actions.to_vec(), func, nb_args))
    }

    pub fn register_converters<'callback, A, R, F>(&self, name: &str, func: F) -> Result<()>
    where
        A: FromLuaMulti<'callback>,
        R: ToLua<'callback>,
        F: Fn(&'callback Lua, Core, A) -> Result<R> + 'static,
    {
        let func = self
            .0
            .create_function(move |lua, args| func(lua, Core::new(lua)?, args))?;
        self.1.call_function("register_converters", (name, func))
    }

    pub fn register_fetches<'callback, A, R, F>(&self, name: &str, func: F) -> Result<()>
    where
        A: FromLuaMulti<'callback>,
        R: ToLua<'callback>,
        F: Fn(&'callback Lua, Core, A) -> Result<R> + 'static,
    {
        let func = self
            .0
            .create_function(move |lua, args| func(lua, Core::new(lua)?, args))?;
        self.1.call_function("register_fetches", (name, func))
    }

    pub fn register_service<'callback, A, R, F>(
        &self,
        name: &str,
        mode: ServiceMode,
        func: F,
    ) -> Result<()>
    where
        A: FromLuaMulti<'callback>,
        R: ToLua<'callback>,
        F: Fn(&'callback Lua, Core, A) -> Result<R> + 'static,
    {
        let func = self
            .0
            .create_function(move |lua, args| func(lua, Core::new(lua)?, args))?;
        let mode = match mode {
            ServiceMode::Tcp => "tcp",
            ServiceMode::Http => "http",
        };
        self.1.call_function("register_service", (name, mode, func))
    }

    pub fn register_init<'callback, F>(&self, func: F) -> Result<()>
    where
        F: Fn(&'callback Lua, Core) -> Result<()> + 'static,
    {
        let func = self
            .0
            .create_function(move |lua, ()| func(lua, Core::new(lua)?))?;
        self.1.call_function("register_init", func)
    }

    pub fn register_task<'callback, F>(&self, func: F) -> Result<()>
    where
        F: Fn(&'callback Lua, Core) -> Result<()> + 'static,
    {
        let func = self
            .0
            .create_function(move |lua, ()| func(lua, Core::new(lua)?))?;
        self.1.call_function("register_task", func)
    }

    // pub fn register_cli<'callback, F>(&self, func: F) -> Result<()>
    // where
    //     F: Fn(&'callback Lua) -> Result<()> + 'static,
    // {
    //     let func = self.0.create_function(move |lua, ()| func(lua))?;
    //     self.1.call_function("register_task", func)
    // }

    pub fn set_nice(&self, nice: i32) -> Result<()> {
        self.1.call_function("set_nice", nice)
    }

    pub fn set_map(&self, filename: &str, key: &str, value: &str) -> Result<()> {
        self.1.call_function("set_map", (filename, key, value))
    }

    pub fn sleep(&self, seconds: usize) -> Result<()> {
        self.1.call_function("sleep", seconds)
    }

    // TODO
    pub fn tcp(&self) -> Result<Table> {
        self.1.call_function("tcp", ())
    }

    // Drop: concat()

    // TODO: parse_addr
    // TODO: match_addr
    // TODO: tokenize

    // pub fn get_backend(&self, name: &str) -> LuaResult<Option<LuaTable>> {
    //     let backends: LuaTable = self.1.get("backends")?;
    //     return backends.get(name);
    // }
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
