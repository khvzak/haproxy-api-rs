use mlua::{FromLua, Lua, Result, Table, TableExt, ToLua, Value};

use crate::{Converters, Fetches, Http, LogLevel};

/// The txn class contain all the functions relative to the http or tcp transaction.
#[derive(Clone)]
pub struct Txn<'lua> {
    class: Table<'lua>,
    pub c: Converters<'lua>,
    pub f: Fetches<'lua>,
}

impl<'lua> Txn<'lua> {
    /// Returns an HTTP class object.
    pub fn http(&self) -> Result<Http<'lua>> {
        self.class.get("http")
    }

    /// Sends a log on the default syslog server if it is configured and on the stderr if it is allowed.
    pub fn log<S>(&self, level: LogLevel, msg: &S) -> Result<()>
    where
        S: AsRef<str> + ?Sized,
    {
        let msg = msg.as_ref();
        self.class.call_method("log", (level, msg))
    }

    /// Sends a log line with the default loglevel for the proxy associated with the transaction.
    pub fn deflog<S>(&self, msg: &S) -> Result<()>
    where
        S: AsRef<str> + ?Sized,
    {
        self.class.call_method("log", msg.as_ref())
    }

    /// Returns data stored in the current transaction (with the `set_priv()`) function.
    pub fn get_priv<R: FromLua<'lua>>(&self) -> Result<R> {
        self.class.call_method("get_priv", ())
    }

    /// Stores any data in the current HAProxy transaction.
    /// This action replaces the old stored data.
    pub fn set_priv<A: ToLua<'lua>>(&self, val: A) -> Result<()> {
        self.class.call_method("set_priv", val)
    }

    /// Returns data stored in the variable `name`.
    pub fn get_var<R: FromLua<'lua>>(&self, name: &str) -> Result<R> {
        self.class.call_method("get_var", name)
    }

    /// Store variable `name` in an HAProxy converting the type.
    pub fn set_var<A: ToLua<'lua>>(&self, name: &str, val: A) -> Result<()> {
        self.class.call_method("set_var", (name, val))
    }

    /// Unsets the variable `name`.
    pub fn unset_var(&self, name: &str) -> Result<()> {
        self.class.call_method("unset_var", name)
    }

    /// Changes the log level of the current request.
    /// The `level` must be an integer between 0 and 7.
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
            class,
        })
    }
}
