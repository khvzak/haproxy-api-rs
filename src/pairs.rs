use std::collections::HashMap;
use std::marker::PhantomData;

use mlua::{FromLua, FromLuaMulti, Function, Lua, LuaString, MultiValue, Result, Table, Value};

pub(crate) struct Pairs<'lua, T> {
    next: Function,
    lua: &'lua Lua,
    _marker: PhantomData<fn() -> T>,
}

impl<'lua, T> Pairs<'lua, T> {
    pub(crate) fn from(table: &Table, lua: &'lua Lua) -> Result<Self> {
        let metatable = table
            .metatable()
            .ok_or_else(|| mlua::Error::RuntimeError("table has no __pairs metamethod".into()))?;
        let pairs: Function = metatable.raw_get("__pairs")?;
        Ok(Self {
            next: pairs.call(table.clone())?,
            lua,
            _marker: PhantomData,
        })
    }
}

impl<T> Iterator for Pairs<'_, T>
where
    T: FromLua,
{
    type Item = Result<(LuaString, T)>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut values = match self.next.call::<MultiValue>(()) {
            Ok(values) => values,
            Err(error) => return Some(Err(error)),
        };
        let key = match values.pop_front() {
            None | Some(Value::Nil) => return None,
            Some(value) => match LuaString::from_lua(value, self.lua) {
                Ok(key) => key,
                Err(error) => return Some(Err(error)),
            },
        };
        Some(T::from_lua_multi(values, self.lua).map(|value| (key, value)))
    }
}

pub(crate) fn collect_pairs<T>(table: &Table, lua: &Lua) -> Result<HashMap<String, T>>
where
    T: FromLua,
{
    let custom_pairs = match table.metatable() {
        Some(metatable) => metatable.raw_get::<Option<Function>>("__pairs")?,
        None => None,
    };
    if custom_pairs.is_some() {
        return Pairs::from(table, lua)
            .map(|pairs| {
                pairs.map(|pair| {
                    let (key, value) = pair?;
                    Ok((key.to_str()?.to_owned(), value))
                })
            })
            .and_then(Iterator::collect);
    }

    table.pairs().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_pairs_are_used_before_raw_table_iteration() {
        let lua = mlua::Lua::new();
        let table: Table = lua
            .load(
                r#"
                local index = 0
                return setmetatable({}, {
                    __pairs = function()
                        return function()
                            index = index + 1
                            if index == 1 then return "first", "one" end
                            if index == 2 then return "second", "two" end
                        end
                    end,
                })
                "#,
            )
            .eval()
            .unwrap();

        let values = collect_pairs::<String>(&table, &lua).unwrap();

        assert_eq!(values.get("first"), Some(&"one".to_owned()));
        assert_eq!(values.get("second"), Some(&"two".to_owned()));
    }

    #[test]
    fn ordinary_tables_keep_the_raw_pairs_fallback() {
        let lua = mlua::Lua::new();
        let table = lua.create_table().unwrap();
        table.set("first", "one").unwrap();

        let values = collect_pairs::<String>(&table, &lua).unwrap();

        assert_eq!(values.get("first"), Some(&"one".to_owned()));
    }

    #[test]
    fn custom_pairs_propagate_iterator_errors() {
        let lua = mlua::Lua::new();
        let table: Table = lua
            .load(
                r#"
                return setmetatable({}, {
                    __pairs = function()
                        return function()
                            error("iterator failed")
                        end
                    end,
                })
                "#,
            )
            .eval()
            .unwrap();

        assert!(collect_pairs::<String>(&table, &lua).is_err());
    }
}
