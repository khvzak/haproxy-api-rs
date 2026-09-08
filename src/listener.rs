use mlua::{FromLua, Lua, ObjectLike, Result, Table, Value};

/// A "Listener" class which indicates the manipulated listener.
#[derive(Clone)]
pub struct Listener(Table);

impl Listener {
    /// Returns server statistics.
    #[inline]
    pub fn get_stats(&self) -> Result<Option<Table>> {
        self.0.call_method("get_stats", ())
    }
}

impl FromLua for Listener {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        Ok(Listener(Table::from_lua(value, lua)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listener_without_frontend_preserves_haproxy_nil() {
        let lua = Lua::new();
        let listener = lua.create_table().unwrap();
        listener
            .set(
                "get_stats",
                lua.create_function(|_, _: Table| Ok::<Option<Table>, mlua::Error>(None))
                    .unwrap(),
            )
            .unwrap();

        assert!(Listener(listener).get_stats().unwrap().is_none());
    }
}
