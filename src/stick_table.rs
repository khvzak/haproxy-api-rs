use mlua::{FromLua, Lua, Result, Table, TableExt, Value};

/// The "StickTable" class can be used to access the HAProxy stick tables.
#[derive(Clone)]
pub struct StickTable<'lua> {
    class: Table<'lua>,
}

impl<'lua> StickTable<'lua> {
    /// Returns stick table attributes as a Lua table.
    #[inline]
    pub fn info(&self) -> Result<Table<'lua>> {
        self.class.call_method("info", ())
    }

    /// Returns stick table entry for given `key`.
    #[inline]
    pub fn lookup(&self, key: &str) -> Result<Table<'lua>> {
        self.class.call_method("lookup", key)
    }

    /// Returns all entries in stick table.
    ///
    /// An optional `filter` can be used to extract entries with specific data values.
    /// Filter is a table with valid comparison operators as keys followed by data type name and value pairs.
    /// Check out the HAProxy docs for "show table" for more details.
    #[inline]
    pub fn dump(&self, filter: Option<&str>) -> Result<Table<'lua>> {
        self.class.call_method("dump", filter)
    }
}

impl<'lua> FromLua<'lua> for StickTable<'lua> {
    #[inline]
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(StickTable { class })
    }
}
