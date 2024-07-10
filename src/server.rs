use std::ops::Deref;

use mlua::{AsChunk, FromLua, Lua, Result, Table, TableExt, Value};

use crate::Proxy;

/// The "Server" class provides a way for manipulating servers and retrieving information.
#[derive(Clone)]
pub struct Server<'lua> {
    lua: &'lua Lua,
    class: Table<'lua>,
}

impl<'lua> Server<'lua> {
    /// Returns the name of the server.
    #[inline]
    pub fn get_name(&self) -> Result<String> {
        self.class.call_method("get_name", ())
    }

    /// Returns the proxy unique identifier of the server.
    #[inline]
    pub fn get_puid(&self) -> Result<String> {
        self.class.call_method("get_puid", ())
    }

    /// Returns the rid (revision ID) of the server.
    #[inline]
    pub fn get_rid(&self) -> Result<u64> {
        self.class.call_method("get_rid", ())
    }

    /// Returns true if the server is currently draining sticky connections.
    #[inline]
    pub fn is_draining(&self) -> Result<bool> {
        self.class.call_method("is_draining", ())
    }

    /// Return true if the server is a backup server.
    #[inline]
    pub fn is_backup(&self) -> Result<bool> {
        self.class.call_method("is_backup", ())
    }

    /// Return true if the server was instantiated at runtime (e.g.: from the cli).
    #[inline]
    pub fn is_dynamic(&self) -> Result<bool> {
        self.class.call_method("is_dynamic", ())
    }

    /// Return the number of currently active sessions on the server.
    pub fn get_cur_sess(&self) -> Result<u64> {
        self.class.call_method("get_cur_sess", ())
    }

    /// Return the number of pending connections to the server.
    #[inline]
    pub fn get_pend_conn(&self) -> Result<u64> {
        self.class.call_method("get_pend_sess", ())
    }

    /// Dynamically changes the maximum connections of the server.
    #[inline]
    pub fn set_maxconn(&self, maxconn: u64) -> Result<()> {
        self.class.call_method("set_maxconn", maxconn)
    }

    /// Returns an integer representing the server maximum connections.
    #[inline]
    pub fn get_maxconn(&self) -> Result<u64> {
        self.class.call_method("get_maxconn", ())
    }

    /// Dynamically changes the weight of the server.
    /// See the management socket documentation for more information about the format of the string.
    #[inline]
    pub fn set_weight(&self, weight: &str) -> Result<()> {
        self.class.call_method("set_weight", weight)
    }

    /// Returns an integer representing the server weight.
    #[inline]
    pub fn get_weight(&self) -> Result<u32> {
        self.class.call_method("get_weight", ())
    }

    /// Dynamically changes the address of the server.
    #[inline]
    pub fn set_addr(&self, addr: String, port: Option<u16>) -> Result<()> {
        self.class.call_method("set_addr", (addr, port))
    }

    /// Returns a string describing the address of the server.
    #[inline]
    pub fn get_addr(&self) -> Result<String> {
        self.class.call_method("get_addr", ())
    }

    /// Returns a table containing the server statistics.
    #[inline]
    pub fn get_stats(&self) -> Result<Table<'lua>> {
        self.class.call_method("get_stats", ())
    }

    /// Returns the parent proxy to which the server belongs.
    pub fn get_proxy(&self) -> Result<Proxy<'lua>> {
        self.class.call_method("get_proxy", ())
    }

    /// Shutdowns all the sessions attached to the server.
    #[inline]
    pub fn shut_sess(&self) -> Result<()> {
        self.class.call_method("shut_sess", ())
    }

    /// Drains sticky sessions.
    #[inline]
    pub fn set_drain(&self) -> Result<()> {
        self.class.call_method("set_drain", ())
    }

    /// Sets maintenance mode.
    #[inline]
    pub fn set_maint(&self) -> Result<()> {
        self.class.call_method("set_maint", ())
    }

    /// Sets normal mode.
    #[inline]
    pub fn set_ready(&self) -> Result<()> {
        self.class.call_method("set_ready", ())
    }

    /// Enables health checks.
    #[inline]
    pub fn check_enable(&self) -> Result<()> {
        self.class.call_method("check_enable", ())
    }

    /// Disables health checks.
    #[inline]
    pub fn check_disable(&self) -> Result<()> {
        self.class.call_method("check_disable", ())
    }

    /// Forces health-check up.
    #[inline]
    pub fn check_force_up(&self) -> Result<()> {
        self.class.call_method("check_force_up", ())
    }

    /// Forces health-check nolb mode.
    #[inline]
    pub fn check_force_nolb(&self) -> Result<()> {
        self.class.call_method("check_force_nolb", ())
    }

    /// Forces health-check down.
    #[inline]
    pub fn check_force_down(&self) -> Result<()> {
        self.class.call_method("check_force_down", ())
    }

    /// Enables agent check.
    #[inline]
    pub fn agent_enable(&self) -> Result<()> {
        self.class.call_method("agent_enable", ())
    }

    /// Disables agent check.
    #[inline]
    pub fn agent_disable(&self) -> Result<()> {
        self.class.call_method("agent_disable", ())
    }

    /// Forces agent check up.
    #[inline]
    pub fn agent_force_up(&self) -> Result<()> {
        self.class.call_method("agent_force_up", ())
    }

    /// Forces agent check down.
    #[inline]
    pub fn agent_force_down(&self) -> Result<()> {
        self.class.call_method("agent_force_down", ())
    }

    /// Check if the current server is tracking another server.
    #[inline]
    pub fn tracking(&self) -> Result<Option<Server<'lua>>> {
        self.class.call_method("tracking(", ())
    }

    /// Check if the current server is being tracked by other servers.
    #[inline]
    pub fn get_trackers(&self) -> Result<Vec<Server<'lua>>> {
        self.class.call_method("get_trackers", ())
    }

    /// Register a function that will be called on specific server events.
    ///
    /// It works exactly like `core.event_sub()`` except that the subscription
    /// will be performed within the server dedicated subscription list instead of the global one.
    pub fn event_sub<'a, S>(&self, event_types: &[&str], code: S) -> Result<()>
    where
        S: AsChunk<'lua, 'a>,
    {
        let func = self.lua.load(code).into_function()?;
        self.class.call_function("event_sub", (event_types, func))
    }
}

impl<'lua> FromLua<'lua> for Server<'lua> {
    #[inline]
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(Server { lua, class })
    }
}

impl<'lua> Deref for Server<'lua> {
    type Target = Table<'lua>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.class
    }
}
