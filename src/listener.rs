use mlua::{FromLua, Lua, Result, Table, TableExt, Value};

/// A "Listener" class which indicates the manipulated listener.
#[derive(Clone)]
pub struct Listener<'lua>(Table<'lua>);

impl<'lua> Listener<'lua> {
    /// Returns server statistics.
    #[inline]
    pub fn get_stats(&self) -> Result<Table<'lua>> {
        self.0.call_method("get_stats", ())
    }
}

impl<'lua> FromLua<'lua> for Listener<'lua> {
    #[inline]
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        Ok(Listener(Table::from_lua(value, lua)?))
    }
}
