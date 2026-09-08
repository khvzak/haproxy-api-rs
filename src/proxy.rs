use std::collections::HashMap;
use std::ops::Deref;

use mlua::{FromLua, Lua, LuaString, ObjectLike, Result, Table, Value};

use crate::pairs::collect_pairs;
use crate::{listener::Listener, Server, StickTable};

#[derive(Clone)]
pub struct Proxy(Table, Lua);

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

impl Proxy {
    /// Returns the name of the proxy.
    #[inline]
    pub fn get_name(&self) -> Result<Option<String>> {
        self.0.call_method("get_name", ())
    }

    /// Returns the UUID of the proxy.
    #[inline]
    pub fn get_uuid(&self) -> Result<Option<String>> {
        self.0.call_method("get_uuid", ())
    }

    /// Returns a map with the attached servers.
    /// The map is indexed by server name.
    #[inline]
    pub fn get_servers(&self) -> Result<HashMap<String, Server>> {
        collect_pairs(&self.0.get("servers")?, &self.1)
    }

    /// Returns the stick table attached to the proxy.
    #[inline]
    pub fn get_stktable(&self) -> Result<Option<StickTable>> {
        self.0.get("stktable")
    }

    /// Returns a table with the attached listeners.
    /// The table is indexed by listener name.
    #[inline]
    pub fn get_listeners(&self) -> Result<HashMap<String, Listener>> {
        collect_pairs(&self.0.get("listeners")?, &self.1)
    }

    /// Pauses the proxy.
    /// See the management socket documentation for more information.
    #[inline]
    pub fn pause(&self) -> Result<()> {
        self.0.call_method("pause", ())
    }

    /// Resumes the proxy.
    /// See the management socket documentation for more information.
    #[inline]
    pub fn resume(&self) -> Result<()> {
        self.0.call_method("resume", ())
    }

    /// Stops the proxy.
    /// See the management socket documentation for more information.
    #[inline]
    pub fn stop(&self) -> Result<()> {
        self.0.call_method("stop", ())
    }

    /// Kills the session attached to a backup server.
    /// See the management socket documentation for more information.
    #[inline]
    pub fn shut_bcksess(&self) -> Result<()> {
        self.0.call_method("shut_bcksess", ())
    }

    /// Returns an enum describing the capabilities of the proxy.
    #[inline]
    pub fn get_cap(&self) -> Result<Option<ProxyCapability>> {
        let cap: Option<LuaString> = self.0.call_method("get_cap", ())?;
        cap.map(|cap| {
            Ok(match cap.to_str()?.deref() {
                "frontend" => ProxyCapability::Frontend,
                "backend" => ProxyCapability::Backend,
                "proxy" => ProxyCapability::Proxy,
                _ => ProxyCapability::Ruleset,
            })
        })
        .transpose()
    }

    /// Returns an enum describing the mode of the current proxy.
    #[inline]
    pub fn get_mode(&self) -> Result<Option<ProxyMode>> {
        let mode: Option<LuaString> = self.0.call_method("get_mode", ())?;
        mode.map(|mode| {
            Ok(match mode.to_str()?.deref() {
                "tcp" => ProxyMode::Tcp,
                "http" => ProxyMode::Http,
                "health" => ProxyMode::Health,
                _ => ProxyMode::Unknown,
            })
        })
        .transpose()
    }

    /// Returns the number of current active servers for the current proxy
    /// that are eligible for LB.
    #[inline]
    pub fn get_srv_act(&self) -> Result<Option<usize>> {
        self.0.call_method("get_srv_act", ())
    }

    /// Returns the number of backup servers for the current proxy that are eligible for LB.
    #[inline]
    pub fn get_srv_bck(&self) -> Result<Option<usize>> {
        self.0.call_method("get_srv_bck", ())
    }

    /// Returns a table containing the proxy statistics.
    /// The statistics returned are not the same if the proxy is frontend or a backend.
    #[inline]
    pub fn get_stats(&self) -> Result<Option<Table>> {
        self.0.call_method("get_stats", ())
    }
}

impl FromLua for Proxy {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(Proxy(class, lua.clone()))
    }
}

impl Deref for Proxy {
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
    fn deleted_proxy_getters_preserve_haproxy_nil() {
        let lua = Lua::new();
        let proxy = lua.create_table().unwrap();
        let nil_function: mlua::Function =
            lua.load("return function() return nil end").eval().unwrap();
        for name in [
            "get_name",
            "get_uuid",
            "get_cap",
            "get_mode",
            "get_srv_act",
            "get_srv_bck",
            "get_stats",
        ] {
            proxy.set(name, nil_function.clone()).unwrap();
        }

        let proxy = Proxy(proxy, lua);
        assert!(proxy.get_name().unwrap().is_none());
        assert!(proxy.get_uuid().unwrap().is_none());
        assert!(proxy.get_cap().unwrap().is_none());
        assert!(proxy.get_mode().unwrap().is_none());
        assert!(proxy.get_srv_act().unwrap().is_none());
        assert!(proxy.get_srv_bck().unwrap().is_none());
        assert!(proxy.get_stats().unwrap().is_none());
    }
}
