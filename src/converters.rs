use mlua::{FromLua, Lua, Result, Table, TableExt, ToLuaMulti, Value};

pub struct Converters<'lua>(&'lua Lua, Table<'lua>);

impl<'lua> Converters<'lua> {
    pub fn call<A, R>(&self, name: &str, args: A) -> Result<R>
    where
        A: ToLuaMulti<'lua>,
        R: FromLua<'lua>,
    {
        self.1.call_method(name, args)
    }
}

impl<'lua> FromLua<'lua> for Converters<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let t = Table::from_lua(value, lua)?;
        Ok(Converters(lua, t))
    }
}
