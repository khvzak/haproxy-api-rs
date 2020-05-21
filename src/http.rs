use mlua::{FromLua, Lua, Result, Table, TableExt, ToLua, Value};

#[derive(Clone)]
pub struct Http<'lua>(Table<'lua>);

#[derive(Clone)]
pub struct Headers<'lua>(Table<'lua>);

impl<'lua> Http<'lua> {
    pub fn req_get_headers(&self) -> Result<Headers> {
        self.0.call_method("req_get_headers", ())
    }

    pub fn res_get_headers(&self) -> Result<Headers> {
        self.0.call_method("res_get_headers", ())
    }

    pub fn req_add_header<V: ToLua<'lua>>(&self, name: &str, value: V) -> Result<()> {
        self.0.call_method("req_add_header", (name, value))
    }

    pub fn res_add_header<V: ToLua<'lua>>(&self, name: &str, value: V) -> Result<()> {
        self.0.call_method("res_add_header", (name, value))
    }

    pub fn req_del_header(&self, name: &str) -> Result<()> {
        self.0.call_method("req_del_header", name)
    }

    pub fn res_del_header(&self, name: &str) -> Result<()> {
        self.0.call_method("res_del_header", name)
    }

    pub fn req_set_header<V: ToLua<'lua>>(&self, name: &str, value: V) -> Result<()> {
        self.0.call_method("req_set_header", (name, value))
    }

    pub fn res_set_header<V: ToLua<'lua>>(&self, name: &str, value: V) -> Result<()> {
        self.0.call_method("res_set_header", (name, value))
    }

    pub fn req_rep_header(&self, name: &str, regex: &str, replace: &str) -> Result<()> {
        self.0.call_method("req_rep_header", (name, regex, replace))
    }

    pub fn res_rep_header(&self, name: &str, regex: &str, replace: &str) -> Result<()> {
        self.0.call_method("res_rep_header", (name, regex, replace))
    }

    pub fn req_set_method(&self, method: &str) -> Result<()> {
        self.0.call_method("req_set_method", method)
    }

    pub fn req_set_path(&self, path: &str) -> Result<()> {
        self.0.call_method("req_set_path", path)
    }

    pub fn req_set_query(&self, query: &str) -> Result<()> {
        self.0.call_method("req_set_query", query)
    }

    pub fn req_set_uri(&self, uri: &str) -> Result<()> {
        self.0.call_method("req_set_uri", uri)
    }

    pub fn res_set_status(&self, status: u16, reason: Option<&str>) -> Result<()> {
        self.0.call_method("res_set_status", (status, reason))
    }
}

impl<'lua> Headers<'lua> {
    pub fn get(&self, name: &str) -> Result<Vec<String>> {
        let name = name.to_ascii_lowercase();
        let mut result = Vec::new();
        if let Some(values) = self.0.get::<_, Option<Table>>(name)? {
            for v in values.pairs::<i32, _>() {
                result.push(v?.1);
            }
        }
        Ok(result)
    }

    pub fn get_first(&self, name: &str) -> Result<Option<String>> {
        let name = name.to_ascii_lowercase();
        if let Some(values) = self.0.get::<_, Option<Table>>(name)? {
            return values.get(0); // Indexes start from "0"
        }
        Ok(None)
    }
}

impl<'lua> FromLua<'lua> for Http<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        Ok(Http(Table::from_lua(value, lua)?))
    }
}

impl<'lua> FromLua<'lua> for Headers<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        Ok(Headers(Table::from_lua(value, lua)?))
    }
}
