use mlua::{FromLua, Lua, Result, Table, TableExt, ToLua, Value};

use crate::{Channel, Converters, Fetches, Http, LogLevel};

pub struct Txn<'lua> {
    class: Table<'lua>,
    pub c: Converters<'lua>,
    pub f: Fetches<'lua>,
    pub req: Channel<'lua>,
    pub res: Channel<'lua>,
}

impl<'lua> Txn<'lua> {
    pub fn log(&self, level: LogLevel, msg: &str) -> Result<()> {
        self.class.call_method("log", (level, msg))
    }

    pub fn http(&self) -> Result<Http> {
        self.class.get("http")
    }

    pub fn deflog(&self, msg: &str) -> Result<()> {
        self.class.call_method("log", msg)
    }

    pub fn get_priv<R: FromLua<'lua>>(&self) -> Result<R> {
        self.class.call_method("get_priv", ())
    }

    pub fn set_priv<A: ToLua<'lua>>(&self, val: A) -> Result<()> {
        self.class.call_method("set_priv", val)
    }

    pub fn get_var<R: FromLua<'lua>>(&self, name: &str) -> Result<R> {
        self.class.call_method("get_var", name)
    }

    pub fn set_var<A: ToLua<'lua>>(&self, name: &str, val: A) -> Result<()> {
        self.class.call_method("set_var", (name, val))
    }

    pub fn unset_var(&self, name: &str) -> Result<()> {
        self.class.call_method("unset_var", name)
    }

    pub fn set_loglevel(&self, level: LogLevel) -> Result<()> {
        self.class.call_method("set_loglevel", level)
    }

    // TODO: set_tos
    // TODO: set_mark
    // TODO: set_priority_class
    // TODO: set_priority_offset
}

impl<'lua> FromLua<'lua> for Txn<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(Txn {
            c: class.get("c")?,
            f: class.get("f")?,
            req: class.get("req")?,
            res: class.get("res")?,
            class: class,
        })
    }
}
