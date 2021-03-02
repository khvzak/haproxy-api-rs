use mlua::{FromLua, Lua, Result, Table, TableExt, Value};

/// The "Server" class provides a way for manipulating servers and retrieving information.
#[derive(Clone)]
pub struct Server<'lua> {
    class: Table<'lua>,
    pub name: String,
    pub puid: String,
}

impl<'lua> Server<'lua> {
    /// Returns true if the server is currently draining sticky connections.
    pub fn is_draining(&self) -> Result<bool> {
        self.class.call_method("is_draining", ())
    }

    /// Dynamically changes the maximum connections of the server.
    pub fn set_maxconn(&self, maxconn: u64) -> Result<()> {
        self.class.call_method("set_maxconn", maxconn)
    }

    /// Returns an integer representing the server maximum connections.
    pub fn get_maxconn(&self) -> Result<u64> {
        self.class.call_method("get_maxconn", ())
    }

    /// Dynamically changes the weight of the server.
    /// See the management socket documentation for more information about the format of the string.
    pub fn set_weight(&self, weight: &str) -> Result<()> {
        self.class.call_method("set_weight", weight)
    }

    /// Returns an integer representing the server weight.
    pub fn get_weight(&self) -> Result<u32> {
        self.class.call_method("get_weight", ())
    }

    /// Dynamically changes the address of the server.
    pub fn set_addr(&self, addr: String, port: Option<u16>) -> Result<()> {
        self.class.call_method("set_addr", (addr, port))
    }

    /// Returns a string describing the address of the server.
    pub fn get_addr(&self) -> Result<String> {
        self.class.call_method("get_addr", ())
    }

    /// Returns a table containing the server statistics.
    pub fn get_stats(&self) -> Result<Table<'lua>> {
        self.class.call_method("get_stats", ())
    }

    /// Shutdowns all the sessions attached to the server.
    pub fn shut_sess(&self) -> Result<()> {
        self.class.call_method("shut_sess", ())
    }

    /// Drains sticky sessions.
    pub fn set_drain(&self) -> Result<()> {
        self.class.call_method("set_drain", ())
    }

    /// Sets maintenance mode.
    pub fn set_maint(&self) -> Result<()> {
        self.class.call_method("set_maint", ())
    }

    /// Sets normal mode.
    pub fn set_ready(&self) -> Result<()> {
        self.class.call_method("set_ready", ())
    }

    /// Enables health checks.
    pub fn check_enable(&self) -> Result<()> {
        self.class.call_method("check_enable", ())
    }

    /// Disables health checks.
    pub fn check_disable(&self) -> Result<()> {
        self.class.call_method("check_disable", ())
    }

    /// Forces health-check up.
    pub fn check_force_up(&self) -> Result<()> {
        self.class.call_method("check_force_up", ())
    }

    /// Forces health-check nolb mode.
    pub fn check_force_nolb(&self) -> Result<()> {
        self.class.call_method("check_force_nolb", ())
    }

    /// Forces health-check down.
    pub fn check_force_down(&self) -> Result<()> {
        self.class.call_method("check_force_down", ())
    }

    /// Enables agent check.
    pub fn agent_enable(&self) -> Result<()> {
        self.class.call_method("agent_enable", ())
    }

    /// Disables agent check.
    pub fn agent_disable(&self) -> Result<()> {
        self.class.call_method("agent_disable", ())
    }

    /// Forces agent check up.
    pub fn agent_force_up(&self) -> Result<()> {
        self.class.call_method("agent_force_up", ())
    }

    /// Forces agent check down.
    pub fn agent_force_down(&self) -> Result<()> {
        self.class.call_method("agent_force_down", ())
    }
}

impl<'lua> FromLua<'lua> for Server<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(Server {
            name: class.get("name")?,
            puid: class.get("puid")?,
            class,
        })
    }
}
