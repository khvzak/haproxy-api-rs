use std::ops::Deref;

use mlua::{
    chunk::{AsChunk, Chunk},
    FromLua, Lua, ObjectLike, Result, Table, Value,
};

use crate::{EventSub, Proxy};

/// The "Server" class provides a way for manipulating servers and retrieving information.
#[derive(Clone)]
pub struct Server(Table);

impl Server {
    /// Returns the name of the server.
    #[inline]
    pub fn get_name(&self) -> Result<Option<String>> {
        self.0.call_method("get_name", ())
    }

    /// Returns the proxy unique identifier of the server.
    #[inline]
    pub fn get_puid(&self) -> Result<Option<String>> {
        self.0.call_method("get_puid", ())
    }

    /// Returns the rid (revision ID) of the server.
    #[inline]
    pub fn get_rid(&self) -> Result<Option<u64>> {
        self.0.call_method("get_rid", ())
    }

    /// Returns true if the server is currently draining sticky connections.
    #[inline]
    pub fn is_draining(&self) -> Result<Option<bool>> {
        self.0.call_method("is_draining", ())
    }

    /// Returns true if the server is a backup server.
    #[inline]
    pub fn is_backup(&self) -> Result<Option<bool>> {
        self.0.call_method("is_backup", ())
    }

    /// Returns true if the server was instantiated at runtime (for example, from the CLI).
    #[inline]
    pub fn is_dynamic(&self) -> Result<Option<bool>> {
        self.0.call_method("is_dynamic", ())
    }

    /// Return the number of currently active sessions on the server.
    pub fn get_cur_sess(&self) -> Result<Option<u64>> {
        self.0.call_method("get_cur_sess", ())
    }

    /// Return the number of pending connections to the server.
    #[inline]
    pub fn get_pend_conn(&self) -> Result<Option<u64>> {
        self.0.call_method("get_pend_conn", ())
    }

    /// Dynamically changes the maximum connections of the server.
    #[inline]
    pub fn set_maxconn(&self, maxconn: u64) -> Result<()> {
        self.0.call_method("set_maxconn", maxconn)
    }

    /// Returns an integer representing the server maximum connections.
    #[inline]
    pub fn get_maxconn(&self) -> Result<Option<u64>> {
        self.0.call_method("get_maxconn", ())
    }

    /// Dynamically changes the weight of the server.
    /// See the management socket documentation for more information about the format of the string.
    #[inline]
    pub fn set_weight(&self, weight: &str) -> Result<()> {
        self.0.call_method("set_weight", weight)
    }

    /// Returns an integer representing the server weight.
    #[inline]
    pub fn get_weight(&self) -> Result<Option<u32>> {
        self.0.call_method("get_weight", ())
    }

    /// Dynamically changes the address of the server.
    #[inline]
    pub fn set_addr(&self, addr: String, port: Option<u16>) -> Result<()> {
        self.0.call_method("set_addr", (addr, port))
    }

    /// Returns a string describing the address of the server.
    #[inline]
    pub fn get_addr(&self) -> Result<Option<String>> {
        self.0.call_method("get_addr", ())
    }

    /// Returns a table containing the server statistics.
    #[inline]
    pub fn get_stats(&self) -> Result<Option<Table>> {
        self.0.call_method("get_stats", ())
    }

    /// Returns the parent proxy to which the server belongs.
    pub fn get_proxy(&self) -> Result<Option<Proxy>> {
        self.0.call_method("get_proxy", ())
    }

    /// Shuts down all sessions attached to the server.
    #[inline]
    pub fn shut_sess(&self) -> Result<()> {
        self.0.call_method("shut_sess", ())
    }

    /// Drains sticky sessions.
    #[inline]
    pub fn set_drain(&self) -> Result<()> {
        self.0.call_method("set_drain", ())
    }

    /// Sets maintenance mode.
    #[inline]
    pub fn set_maint(&self) -> Result<()> {
        self.0.call_method("set_maint", ())
    }

    /// Sets normal mode.
    #[inline]
    pub fn set_ready(&self) -> Result<()> {
        self.0.call_method("set_ready", ())
    }

    /// Enables health checks.
    #[inline]
    pub fn check_enable(&self) -> Result<()> {
        self.0.call_method("check_enable", ())
    }

    /// Disables health checks.
    #[inline]
    pub fn check_disable(&self) -> Result<()> {
        self.0.call_method("check_disable", ())
    }

    /// Forces health-check up.
    #[inline]
    pub fn check_force_up(&self) -> Result<()> {
        self.0.call_method("check_force_up", ())
    }

    /// Forces health-check nolb mode.
    #[inline]
    pub fn check_force_nolb(&self) -> Result<()> {
        self.0.call_method("check_force_nolb", ())
    }

    /// Forces health-check down.
    #[inline]
    pub fn check_force_down(&self) -> Result<()> {
        self.0.call_method("check_force_down", ())
    }

    /// Enables agent check.
    #[inline]
    pub fn agent_enable(&self) -> Result<()> {
        self.0.call_method("agent_enable", ())
    }

    /// Disables agent check.
    #[inline]
    pub fn agent_disable(&self) -> Result<()> {
        self.0.call_method("agent_disable", ())
    }

    /// Forces agent check up.
    #[inline]
    pub fn agent_force_up(&self) -> Result<()> {
        self.0.call_method("agent_force_up", ())
    }

    /// Forces agent check down.
    #[inline]
    pub fn agent_force_down(&self) -> Result<()> {
        self.0.call_method("agent_force_down", ())
    }

    /// Check if the current server is tracking another server.
    #[inline]
    pub fn tracking(&self) -> Result<Option<Server>> {
        self.0.call_method("tracking", ())
    }

    /// Check if the current server is being tracked by other servers.
    #[inline]
    pub fn get_trackers(&self) -> Result<Vec<Server>> {
        self.0.call_method("get_trackers", ())
    }

    /// Register a function that will be called on specific server events.
    ///
    /// It works exactly like `core.event_sub()` except that the subscription
    /// will be performed within the server dedicated subscription list instead of the global one.
    pub fn event_sub(&self, event_types: &[&str], code: impl AsChunk) -> Result<EventSub> {
        self.0
            .call_method("event_sub", (event_types, Chunk::wrap(code)))
    }
}

impl FromLua for Server {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(Server(class))
    }
}

impl Deref for Server {
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
    fn tracking_calls_the_tracking_method() {
        let lua = Lua::new();
        let server = lua.create_table().unwrap();
        let tracked = lua.create_table().unwrap();
        server
            .set(
                "tracking",
                lua.create_function(move |_, (_table, ()): (Table, ())| Ok(Some(tracked.clone())))
                    .unwrap(),
            )
            .unwrap();

        let tracking = Server(server).tracking().unwrap();

        assert!(tracking.is_some());
    }

    #[test]
    fn event_sub_passes_the_server_receiver() {
        let lua = Lua::new();
        let server = lua.create_table().unwrap();
        server
            .set(
                "event_sub",
                lua.create_function(|_, (this, events, _code): (Table, Table, mlua::Function)| {
                    assert_eq!(events.get::<String>(1).unwrap(), "change");
                    this.raw_set("receiver_seen", true)?;
                    Ok(this)
                })
                .unwrap(),
            )
            .unwrap();

        let server = Server(server);
        server.event_sub(&["change"], "return true").unwrap();

        let receiver_seen: bool = server.get("receiver_seen").unwrap();
        assert!(receiver_seen);
    }

    #[test]
    fn deleted_server_getters_preserve_haproxy_nil() {
        let lua = Lua::new();
        let server = lua.create_table().unwrap();
        let nil_function: mlua::Function =
            lua.load("return function() return nil end").eval().unwrap();
        for name in [
            "get_name",
            "get_puid",
            "get_rid",
            "is_draining",
            "is_backup",
            "is_dynamic",
            "get_cur_sess",
            "get_pend_conn",
            "get_maxconn",
            "get_weight",
            "get_addr",
            "get_stats",
            "get_proxy",
        ] {
            server.set(name, nil_function.clone()).unwrap();
        }

        let server = Server(server);
        assert!(server.get_name().unwrap().is_none());
        assert!(server.get_puid().unwrap().is_none());
        assert!(server.get_rid().unwrap().is_none());
        assert!(server.is_draining().unwrap().is_none());
        assert!(server.is_backup().unwrap().is_none());
        assert!(server.is_dynamic().unwrap().is_none());
        assert!(server.get_cur_sess().unwrap().is_none());
        assert!(server.get_pend_conn().unwrap().is_none());
        assert!(server.get_maxconn().unwrap().is_none());
        assert!(server.get_weight().unwrap().is_none());
        assert!(server.get_addr().unwrap().is_none());
        assert!(server.get_stats().unwrap().is_none());
        assert!(server.get_proxy().unwrap().is_none());
    }
}
