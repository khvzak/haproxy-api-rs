use mlua::{FromLua, IntoLua, Lua, Result, Table, TableExt, Value};

use crate::{Converters, Fetches, Http, LogLevel};

/// The txn class contain all the functions relative to the http or tcp transaction.
#[derive(Clone)]
pub struct Txn<'lua> {
    class: Table<'lua>,
    pub c: Converters<'lua>,
    pub f: Fetches<'lua>,
    pub(crate) r#priv: Value<'lua>,
}

impl<'lua> Txn<'lua> {
    /// Returns an HTTP class object.
    #[inline]
    pub fn http(&self) -> Result<Http<'lua>> {
        self.class.get("http")
    }

    /// Sends a log on the default syslog server if it is configured and on the stderr if it is allowed.
    #[inline]
    pub fn log(&self, level: LogLevel, msg: impl AsRef<str>) -> Result<()> {
        let msg = msg.as_ref();
        self.class.call_method("log", (level, msg))
    }

    /// Sends a log line with the default loglevel for the proxy associated with the transaction.
    #[inline]
    pub fn deflog<S>(&self, msg: &S) -> Result<()>
    where
        S: AsRef<str> + ?Sized,
    {
        self.class.call_method("deflog", msg.as_ref())
    }

    /// Returns data stored in the current transaction (with the `set_priv()`) function.
    #[inline]
    pub fn get_priv<R: FromLua<'lua>>(&self) -> Result<R> {
        self.class.call_method("get_priv", ())
    }

    /// Stores any data in the current HAProxy transaction.
    /// This action replaces the old stored data.
    #[inline]
    pub fn set_priv<A: IntoLua<'lua>>(&self, val: A) -> Result<()> {
        self.class.call_method("set_priv", val)
    }

    /// Returns data stored in the variable `name`.
    #[inline]
    pub fn get_var<R: FromLua<'lua>>(&self, name: &str) -> Result<R> {
        self.class.call_method("get_var", name)
    }

    /// Store variable `name` in an HAProxy converting the type.
    #[inline]
    pub fn set_var<A: IntoLua<'lua>>(&self, name: &str, val: A) -> Result<()> {
        self.class.call_method("set_var", (name, val))
    }

    /// Unsets the variable `name`.
    #[inline]
    pub fn unset_var(&self, name: &str) -> Result<()> {
        self.class.call_method("unset_var", name)
    }

    /// Changes the log level of the current request.
    /// The `level` must be an integer between 0 and 7.
    #[inline]
    pub fn set_loglevel(&self, level: LogLevel) -> Result<()> {
        self.class.call_method("set_loglevel", level)
    }

    // TODO: set_tos
    // TODO: set_mark
    // TODO: set_priority_class
    // TODO: set_priority_offset
}

impl<'lua> FromLua<'lua> for Txn<'lua> {
    #[inline]
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(Txn {
            c: class.get("c")?,
            f: class.get("f")?,
            class,
            r#priv: Value::Nil,
        })
    }
}
