use std::collections::HashMap;

use mlua::{FromLua, Lua, Result, Table, TableExt, Value};

use crate::Server;

#[derive(Clone)]
pub struct Proxy<'lua> {
    class: Table<'lua>,
    pub name: String,
    pub uuid: String,
}

#[derive(Debug)]
pub enum ProxyCapability {
    Frontend,
    Backend,
    Proxy,
    Ruleset,
}

#[derive(Debug)]
pub enum ProxyMode {
    Tcp,
    Http,
    Health,
    Unknown,
}

impl<'lua> Proxy<'lua> {
    pub fn pause(&self) -> Result<()> {
        self.class.call_method("pause", ())
    }

    pub fn resume(&self) -> Result<()> {
        self.class.call_method("resume", ())
    }

    pub fn stop(&self) -> Result<()> {
        self.class.call_method("stop", ())
    }

    pub fn shut_bcksess(&self) -> Result<()> {
        self.class.call_method("shut_bcksess", ())
    }

    pub fn get_cap(&self) -> Result<ProxyCapability> {
        let cap: String = self.class.call_method("get_cap", ())?;
        match cap.as_str() {
            "frontend" => Ok(ProxyCapability::Frontend),
            "backend" => Ok(ProxyCapability::Backend),
            "proxy" => Ok(ProxyCapability::Proxy),
            _ => Ok(ProxyCapability::Ruleset),
        }
    }

    pub fn get_mode(&self) -> Result<ProxyMode> {
        let mode: String = self.class.call_method("get_mode", ())?;
        match mode.as_str() {
            "tcp" => Ok(ProxyMode::Tcp),
            "http" => Ok(ProxyMode::Http),
            "health" => Ok(ProxyMode::Health),
            _ => Ok(ProxyMode::Unknown),
        }
    }

    // TODO: get_stats

    pub fn servers(&self) -> Result<HashMap<String, Server>> {
        self.class.get("servers")
    }

    // TODO: stktable
    // TODO: listeners
}

impl<'lua> FromLua<'lua> for Proxy<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(Proxy {
            name: class.get("name")?,
            uuid: class.get("uuid")?,
            class,
        })
    }
}
