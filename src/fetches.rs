use mlua::{FromLua, Lua, Result, Table, TableExt, ToLuaMulti, Value};

pub struct Fetches<'lua>(&'lua Lua, Table<'lua>);

impl<'lua> Fetches<'lua> {
    pub fn call<A, R>(&self, name: &str, args: A) -> Result<R>
    where
        A: ToLuaMulti<'lua>,
        R: FromLua<'lua>,
    {
        self.1.call_method(name, args)
    }
}

impl<'lua> FromLua<'lua> for Fetches<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let t = Table::from_lua(value, lua)?;
        Ok(Fetches(lua, t))
    }
}
