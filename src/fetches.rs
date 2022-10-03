use mlua::{FromLua, Lua, Result, Table, TableExt, ToLuaMulti, Value};

/// The "Fetches" class allows to call a lot of internal HAProxy sample fetches.
#[derive(Clone)]
pub struct Fetches<'lua>(Table<'lua>);

impl<'lua> Fetches<'lua> {
    /// Executes an internal haproxy sample fetch.
    #[inline]
    pub fn get<A, R>(&self, name: &str, args: A) -> Result<R>
    where
        A: ToLuaMulti<'lua>,
        R: FromLua<'lua>,
    {
        self.0.call_method(name, args)
    }

    /// The same as `get` but always returns string.
    #[inline]
    pub fn get_str<A>(&self, name: &str, args: A) -> Result<String>
    where
        A: ToLuaMulti<'lua>,
    {
        Ok(match self.0.call_method(name, args)? {
            Some(val) => val,
            None => String::new(),
        })
    }
}

impl<'lua> FromLua<'lua> for Fetches<'lua> {
    #[inline]
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        Ok(Fetches(Table::from_lua(value, lua)?))
    }
}
