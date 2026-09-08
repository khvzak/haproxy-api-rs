use std::ops::Deref;

use mlua::{FromLua, Lua, ObjectLike, Result, Table, Value};

/// The "EventSub" class that can be used to manipulate HAProxy subscription.
#[derive(Clone)]
pub struct EventSub(Table);

impl EventSub {
    /// Unsubscribes this event subscription.
    #[inline]
    pub fn unsub(&self) -> Result<()> {
        self.0.call_method("unsub", ())
    }
}

impl FromLua for EventSub {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(EventSub(class))
    }
}

impl Deref for EventSub {
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
    fn unsub_accepts_haproxy_no_return() {
        let lua = Lua::new();
        let subscription = lua.create_table().unwrap();
        subscription
            .set(
                "unsub",
                lua.create_function(|_, _: Table| Ok::<(), mlua::Error>(()))
                    .unwrap(),
            )
            .unwrap();

        EventSub(subscription).unsub().unwrap();
    }
}
