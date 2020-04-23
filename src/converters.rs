use mlua::{FromLua, Lua, Result, Table, TableExt, ToLuaMulti, Value};

#[derive(Clone)]
pub struct Converters<'lua>(Table<'lua>);

impl<'lua> Converters<'lua> {
    pub fn call<A, R>(&self, name: &str, args: A) -> Result<R>
    where
        A: ToLuaMulti<'lua>,
        R: FromLua<'lua>,
    {
        self.0.call_method(name, args)
    }
}

impl<'lua> FromLua<'lua> for Converters<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        Ok(Converters(Table::from_lua(value, lua)?))
    }
}
