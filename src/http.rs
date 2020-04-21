use bstr::BString;
use mlua::{FromLua, Lua, Result, Table, TableExt, ToLua, Value};

use crate::{Converters, Fetches};

pub struct Http<'lua>(&'lua Lua, Table<'lua>);

pub struct Headers<'lua>(Table<'lua>);

pub struct AppletHttp<'lua> {
    class: Table<'lua>,
    pub method: String,
    pub version: String,
    pub path: String,
    pub query_string: String,
    pub body_length: usize,
    pub headers: Headers<'lua>,
}

impl<'lua> Http<'lua> {
    pub fn req_get_headers(&self) -> Result<Headers> {
        self.1.call_method("req_get_headers", ())
    }

    pub fn res_get_headers(&self) -> Result<Headers> {
        self.1.call_method("res_get_headers", ())
    }

    pub fn req_add_header(&self, name: &str, value: &str) -> Result<()> {
        self.1.call_method("req_add_header", (name, value))
    }

    pub fn res_add_header(&self, name: &str, value: &str) -> Result<()> {
        self.1.call_method("res_add_header", (name, value))
    }

    pub fn req_del_header(&self, name: &str) -> Result<()> {
        self.1.call_method("req_del_header", name)
    }

    pub fn res_del_header(&self, name: &str) -> Result<()> {
        self.1.call_method("res_del_header", name)
    }

    pub fn req_set_header(&self, name: &str, value: &str) -> Result<()> {
        self.1.call_method("req_set_header", (name, value))
    }

    pub fn res_set_header(&self, name: &str, value: &str) -> Result<()> {
        self.1.call_method("res_set_header", (name, value))
    }

    pub fn req_rep_header(&self, name: &str, regex: &str, replace: &str) -> Result<()> {
        self.1.call_method("req_rep_header", (name, regex, replace))
    }

    pub fn res_rep_header(&self, name: &str, regex: &str, replace: &str) -> Result<()> {
        self.1.call_method("res_rep_header", (name, regex, replace))
    }

    pub fn req_set_method(&self, method: &str) -> Result<()> {
        self.1.call_method("req_set_method", method)
    }

    pub fn req_set_path(&self, path: &str) -> Result<()> {
        self.1.call_method("req_set_path", path)
    }

    pub fn req_set_query(&self, query: &str) -> Result<()> {
        self.1.call_method("req_set_query", query)
    }

    pub fn req_set_uri(&self, uri: &str) -> Result<()> {
        self.1.call_method("req_set_uri", uri)
    }

    pub fn res_set_status(&self, status: u16, reason: Option<&str>) -> Result<()> {
        self.1.call_method("res_set_status", (status, reason))
    }
}

impl<'lua> Headers<'lua> {
    pub fn get(&self, name: &str) -> Result<Vec<String>> {
        let values: Option<Vec<String>> = self.0.get(name)?;
        Ok(values.unwrap_or_default())
    }

    pub fn get_first(&self, name: &str) -> Result<Option<String>> {
        let values: Option<Table> = self.0.get(name)?;
        if let Some(values) = values {
            return values.get(1);
        }
        Ok(None)
    }
}

impl<'lua> AppletHttp<'lua> {
    pub fn converters(&self) -> Result<Converters> {
        self.class.get("c")
    }

    pub fn fetches(&self) -> Result<Fetches> {
        self.class.get("f")
    }

    pub fn set_status(&self, status: u16, reason: Option<&str>) -> Result<()> {
        self.class.call_method("set_status", (status, reason))
    }

    pub fn add_header(&self, name: &str, value: &str) -> Result<()> {
        self.class.call_method("add_header", (name, value))
    }

    pub fn start_response(&self) -> Result<()> {
        self.class.call_method("start_response", ())
    }

    pub fn getline(&self) -> Result<BString> {
        self.class.call_method("getline", ())
    }

    pub fn receive(&self, size: Option<usize>) -> Result<BString> {
        self.class.call_method("receive", size)
    }

    pub fn send(&self, msg: BString) -> Result<()> {
        self.class.call_method("send", msg)
    }

    pub fn get_priv<R: FromLua<'lua>>(&self) -> Result<R> {
        self.class.call_method("get_priv", ())
    }

    pub fn set_priv<A: ToLua<'lua>>(&self, val: A) -> Result<()> {
        self.class.call_method("set_priv", val)
    }

    pub fn get_var<R: FromLua<'lua>>(&self, name: &str) -> Result<R> {
        self.class.call_method("get_var", name)
    }

    pub fn set_var<A: ToLua<'lua>>(&self, name: &str, val: A) -> Result<()> {
        self.class.call_method("set_var", (name, val))
    }

    pub fn unset_var(&self, name: &str) -> Result<()> {
        self.class.call_method("unset_var", name)
    }
}

impl<'lua> FromLua<'lua> for Http<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let t = Table::from_lua(value, lua)?;
        Ok(Http(lua, t))
    }
}

impl<'lua> FromLua<'lua> for Headers<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let t = Table::from_lua(value, lua)?;
        Ok(Headers(t))
    }
}

impl<'lua> FromLua<'lua> for AppletHttp<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(AppletHttp {
            method: class.get("method")?,
            version: class.get("version")?,
            path: class.get("path")?,
            query_string: class.get("query_string")?,
            body_length: class.get("body_length")?,
            headers: class.get("headers")?,
            class: class,
        })
    }
}
