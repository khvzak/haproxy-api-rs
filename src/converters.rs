use mlua::{FromLua, Lua, Result, Table, TableExt, ToLuaMulti, Value};

/// The "Converters" class allows to call a lot of internal HAProxy sample converters.
#[derive(Clone)]
pub struct Converters<'lua>(Table<'lua>);

impl<'lua> Converters<'lua> {
    /// Executes an internal haproxy sample converter.
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

impl<'lua> FromLua<'lua> for Converters<'lua> {
    #[inline]
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        Ok(Converters(Table::from_lua(value, lua)?))
    }
}
