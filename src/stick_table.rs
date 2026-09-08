use std::ops::Deref;

use mlua::{FromLua, Lua, ObjectLike, Result, Table, Value};

/// The "StickTable" class can be used to access the HAProxy stick tables.
#[derive(Clone)]
pub struct StickTable(Table);

impl StickTable {
    /// Returns stick table attributes as a Lua table.
    #[inline]
    pub fn info(&self) -> Result<Table> {
        self.call_method("info", ())
    }

    /// Returns stick table entry for given `key`.
    #[inline]
    pub fn lookup(&self, key: &str) -> Result<Option<Table>> {
        self.call_method("lookup", key)
    }

    /// Returns all entries in stick table.
    ///
    /// An optional `filter` can be used to extract entries with specific data values.
    /// Filter is a table with valid comparison operators as keys followed by data type name and value pairs.
    /// Check out the HAProxy docs for "show table" for more details.
    #[inline]
    pub fn dump(&self, filter: Option<Table>) -> Result<Table> {
        self.call_method("dump", filter)
    }
}

impl FromLua for StickTable {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(StickTable(class))
    }
}

impl Deref for StickTable {
    type Target = Table;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_missing_entry_is_none() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table
            .set(
                "lookup",
                lua.create_function(|_, _: (Table, String)| Ok::<Option<Table>, mlua::Error>(None))
                    .unwrap(),
            )
            .unwrap();

        assert!(StickTable(table).lookup("missing").unwrap().is_none());
    }

    #[test]
    fn dump_passes_the_documented_table_filter() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table
            .set(
                "dump",
                lua.create_function(|_, (this, filter): (Table, Table)| {
                    this.raw_set("filter_len", filter.raw_len())?;
                    Ok(this)
                })
                .unwrap(),
            )
            .unwrap();
        let filter = lua.create_table().unwrap();
        filter.set(1, "gpc0").unwrap();

        let result = StickTable(table).dump(Some(filter)).unwrap();

        assert_eq!(result.get::<usize>("filter_len").unwrap(), 1);
    }
}
