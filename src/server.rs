use mlua::{FromLua, Lua, Result, Table, TableExt, Value};

#[derive(Clone)]
pub struct Server<'lua> {
    class: Table<'lua>,
    pub name: String,
    pub puid: String,
}

impl<'lua> Server<'lua> {
    pub fn is_draining(&self) -> Result<bool> {
        self.class.call_method("is_draining", ())
    }

    pub fn set_maxconn(&self, maxconn: u64) -> Result<()> {
        self.class.call_method("set_maxconn", maxconn)
    }

    pub fn get_maxconn(&self) -> Result<u64> {
        self.class.call_method("get_maxconn", ())
    }

    pub fn set_weight(&self, weight: u32) -> Result<()> {
        self.class.call_method("set_weight", weight)
    }

    pub fn get_weight(&self) -> Result<u32> {
        self.class.call_method("get_weight", ())
    }

    // TODO: addr to rust
    pub fn set_addr(&self, addr: String, port: Option<u16>) -> Result<()> {
        self.class.call_method("set_addr", (addr, port))
    }

    pub fn get_addr(&self) -> Result<String> {
        self.class.call_method("get_addr", ())
    }

    // TODO: get_stats

    pub fn shut_sess(&self) -> Result<()> {
        self.class.call_method("shut_sess", ())
    }

    pub fn set_drain(&self) -> Result<()> {
        self.class.call_method("set_drain", ())
    }

    pub fn set_maint(&self) -> Result<()> {
        self.class.call_method("set_maint", ())
    }

    pub fn set_ready(&self) -> Result<()> {
        self.class.call_method("set_ready", ())
    }

    pub fn check_enable(&self) -> Result<()> {
        self.class.call_method("check_enable", ())
    }

    pub fn check_disable(&self) -> Result<()> {
        self.class.call_method("check_disable", ())
    }

    pub fn check_force_up(&self) -> Result<()> {
        self.class.call_method("check_force_up", ())
    }

    pub fn check_force_nolb(&self) -> Result<()> {
        self.class.call_method("check_force_nolb", ())
    }

    pub fn check_force_down(&self) -> Result<()> {
        self.class.call_method("check_force_down", ())
    }

    pub fn agent_enable(&self) -> Result<()> {
        self.class.call_method("agent_enable", ())
    }

    pub fn agent_disable(&self) -> Result<()> {
        self.class.call_method("agent_disable", ())
    }

    pub fn agent_force_up(&self) -> Result<()> {
        self.class.call_method("agent_force_up", ())
    }

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
