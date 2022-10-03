use std::collections::HashMap;

use mlua::{FromLua, Lua, Result, Table, TableExt, Value};

use crate::{Server, StickTable};

/// The "Proxy" class provides a way for manipulating proxy
/// and retrieving information like statistics.
#[derive(Clone)]
pub struct Proxy<'lua> {
    class: Table<'lua>,
    pub name: String,
    pub uuid: String,
    pub stktable: Option<StickTable<'lua>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProxyCapability {
    Frontend,
    Backend,
    Proxy,
    Ruleset,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProxyMode {
    Tcp,
    Http,
    Health,
    Unknown,
}

impl<'lua> Proxy<'lua> {
    /// Pauses the proxy.
    /// See the management socket documentation for more information.
    #[inline]
    pub fn pause(&self) -> Result<()> {
        self.class.call_method("pause", ())
    }

    /// Resumes the proxy.
    /// See the management socket documentation for more information.
    #[inline]
    pub fn resume(&self) -> Result<()> {
        self.class.call_method("resume", ())
    }

    /// Stops the proxy.
    /// See the management socket documentation for more information.
    #[inline]
    pub fn stop(&self) -> Result<()> {
        self.class.call_method("stop", ())
    }

    /// Kills the session attached to a backup server.
    /// See the management socket documentation for more information.
    #[inline]
    pub fn shut_bcksess(&self) -> Result<()> {
        self.class.call_method("shut_bcksess", ())
    }

    /// Returns a enum describing the capabilities of the proxy.
    #[inline]
    pub fn get_cap(&self) -> Result<ProxyCapability> {
        let cap: String = self.class.call_method("get_cap", ())?;
        match cap.as_str() {
            "frontend" => Ok(ProxyCapability::Frontend),
            "backend" => Ok(ProxyCapability::Backend),
            "proxy" => Ok(ProxyCapability::Proxy),
            _ => Ok(ProxyCapability::Ruleset),
        }
    }

    /// Returns a enum describing the mode of the current proxy.
    #[inline]
    pub fn get_mode(&self) -> Result<ProxyMode> {
        let mode: String = self.class.call_method("get_mode", ())?;
        match mode.as_str() {
            "tcp" => Ok(ProxyMode::Tcp),
            "http" => Ok(ProxyMode::Http),
            "health" => Ok(ProxyMode::Health),
            _ => Ok(ProxyMode::Unknown),
        }
    }

    /// Returns a table containing the proxy statistics.
    /// The statistics returned are not the same if the proxy is frontend or a backend.
    #[inline]
    pub fn get_stats(&self) -> Result<Table<'lua>> {
        self.class.call_method("get_stats", ())
    }

    /// Returns a map with the attached servers.
    /// The map is indexed by server name.
    #[inline]
    pub fn servers(&self) -> Result<HashMap<String, Server<'lua>>> {
        self.class.get("servers")
    }

    // TODO: listeners
}

impl<'lua> FromLua<'lua> for Proxy<'lua> {
    #[inline]
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(Proxy {
            name: class.get("name")?,
            uuid: class.get("uuid")?,
            stktable: class.get("stktable")?,
            class,
        })
    }
}
