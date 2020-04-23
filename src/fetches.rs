use mlua::{FromLua, Lua, Result, Table, TableExt, ToLuaMulti, Value};

#[derive(Clone)]
pub struct Fetches<'lua>(Table<'lua>);

impl<'lua> Fetches<'lua> {
    pub fn get<A, R>(&self, name: &str, args: A) -> Result<R>
    where
        A: ToLuaMulti<'lua>,
        R: FromLua<'lua>,
    {
        self.0.call_method(name, args)
    }

    pub fn get_str<A>(&self, name: &str, args: A) -> Result<Option<String>>
    where
        A: ToLuaMulti<'lua>,
    {
        self.0.call_method(name, args)
    }
}

impl<'lua> FromLua<'lua> for Fetches<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        Ok(Fetches(Table::from_lua(value, lua)?))
    }
}
