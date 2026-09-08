use mlua::{Error, FromLua, IntoLuaMulti, Lua, ObjectLike, Result, Table, Value};

/// The "Converters" class allows to call a lot of internal HAProxy sample converters.
#[derive(Clone)]
pub struct Converters(Value);

impl Converters {
    /// Executes an internal haproxy sample converter.
    #[inline]
    pub fn get<R>(&self, name: &str, args: impl IntoLuaMulti) -> Result<R>
    where
        R: FromLua,
    {
        match &self.0 {
            Value::Table(table) => table.call_method(name, args),
            _ => Err(Error::runtime(
                "converters are unavailable for this transaction",
            )),
        }
    }

    /// The same as `get` but always returns string.
    #[inline]
    pub fn get_str(&self, name: &str, args: impl IntoLuaMulti) -> Result<String> {
        match &self.0 {
            Value::Table(table) => {
                Ok((table.call_method::<Option<_>>(name, args)?).unwrap_or_default())
            }
            _ => Err(Error::runtime(
                "converters are unavailable for this transaction",
            )),
        }
    }
}

impl FromLua for Converters {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        if value.is_nil() {
            return Ok(Converters(Value::Nil));
        }
        Ok(Converters(Value::Table(Table::from_lua(value, lua)?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_converters_are_a_supported_optional_capability() {
        let lua = Lua::new();
        let converters = Converters::from_lua(Value::Nil, &lua).unwrap();

        assert!(converters.get::<Value>("str", ()).is_err());
    }
}
