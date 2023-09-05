use std::marker::PhantomData;

use mlua::{
    FromLua, IntoLua, Lua, Result, String as LuaString, Table, TableExt, TablePairs, Value,
};

/// The "Http" class contain all the HTTP manipulation functions.
#[derive(Clone)]
pub struct Http<'lua>(Table<'lua>);

#[derive(Clone)]
pub struct Headers<'lua>(Table<'lua>);

impl<'lua> Http<'lua> {
    /// Returns a `Headers` table containing all the request headers.
    #[inline]
    pub fn req_get_headers(&self) -> Result<Headers<'lua>> {
        self.0.call_method("req_get_headers", ())
    }

    /// Returns a `Headers` table containing all the response headers.
    #[inline]
    pub fn res_get_headers(&self) -> Result<Headers<'lua>> {
        self.0.call_method("res_get_headers", ())
    }

    /// Appends an HTTP header field `name` with `value` in the request.
    #[inline]
    pub fn req_add_header<V: IntoLua<'lua>>(&self, name: &str, value: V) -> Result<()> {
        self.0.call_method("req_add_header", (name, value))
    }

    /// Appends an HTTP header field `name` with `value` in the response.
    #[inline]
    pub fn res_add_header<V: IntoLua<'lua>>(&self, name: &str, value: V) -> Result<()> {
        self.0.call_method("res_add_header", (name, value))
    }

    /// Removes all HTTP header fields in the request by `name`.
    #[inline]
    pub fn req_del_header(&self, name: &str) -> Result<()> {
        self.0.call_method("req_del_header", name)
    }

    /// Removes all HTTP header fields in the response by `name`.
    #[inline]
    pub fn res_del_header(&self, name: &str) -> Result<()> {
        self.0.call_method("res_del_header", name)
    }

    /// Replaces all occurrence of HTTP request header `name`, by only one containing the `value`.
    #[inline]
    pub fn req_set_header<V: IntoLua<'lua>>(&self, name: &str, value: V) -> Result<()> {
        self.0.call_method("req_set_header", (name, value))
    }

    /// Replaces all occurrence of HTTP response header `name`, by only one containing the `value`.
    #[inline]
    pub fn res_set_header<V: IntoLua<'lua>>(&self, name: &str, value: V) -> Result<()> {
        self.0.call_method("res_set_header", (name, value))
    }

    /// Matches the regular expression in all occurrences of HTTP request header `name` according to `regex`,
    /// and replaces them with the `replace` argument.
    ///
    /// The replacement value can contain back references like 1, 2, ...
    #[inline]
    pub fn req_rep_header(&self, name: &str, regex: &str, replace: &str) -> Result<()> {
        self.0.call_method("req_rep_header", (name, regex, replace))
    }

    /// Matches the regular expression in all occurrences of HTTP response header `name` according to `regex`,
    /// and replaces them with the `replace` argument.
    ///
    /// The replacement value can contain back references like 1, 2, ...
    #[inline]
    pub fn res_rep_header(&self, name: &str, regex: &str, replace: &str) -> Result<()> {
        self.0.call_method("res_rep_header", (name, regex, replace))
    }

    /// Rewrites the request method with the `method`.
    #[inline]
    pub fn req_set_method(&self, method: &str) -> Result<()> {
        self.0.call_method("req_set_method", method)
    }

    /// Rewrites the request path with the `path`.
    #[inline]
    pub fn req_set_path(&self, path: &str) -> Result<()> {
        self.0.call_method("req_set_path", path)
    }

    /// Rewrites the request’s query string which appears after the first question mark (`?`)
    /// with the `query`.
    #[inline]
    pub fn req_set_query(&self, query: &str) -> Result<()> {
        self.0.call_method("req_set_query", query)
    }

    /// Rewrites the request URI with the `uri`.
    #[inline]
    pub fn req_set_uri(&self, uri: &str) -> Result<()> {
        self.0.call_method("req_set_uri", uri)
    }

    /// Rewrites the response status code.
    /// If no custom reason is provided, it will be generated from the status.
    #[inline]
    pub fn res_set_status(&self, status: u16, reason: Option<&str>) -> Result<()> {
        self.0.call_method("res_set_status", (status, reason))
    }
}

impl<'lua> Headers<'lua> {
    #[inline]
    pub fn pairs<V: FromLua<'lua>>(self) -> HeaderPairs<'lua, V> {
        HeaderPairs {
            pairs: self.0.pairs(),
            phantom: PhantomData,
        }
    }

    /// Returns all header fields by `name`.
    #[inline]
    pub fn get<V: FromLua<'lua>>(&self, name: &str) -> Result<Vec<V>> {
        let name = name.to_ascii_lowercase();
        let mut result = Vec::new();
        if let Some(values) = self.0.get::<_, Option<Table>>(name)? {
            let mut pairs = values.pairs::<i32, V>().collect::<Result<Vec<_>>>()?;
            pairs.sort_by_key(|x| x.0);
            result = pairs.into_iter().map(|(_, v)| v).collect();
        }
        Ok(result)
    }

    /// Returns first header field by `name`.
    #[inline]
    pub fn get_first<V: FromLua<'lua>>(&self, name: &str) -> Result<Option<V>> {
        let name = name.to_ascii_lowercase();
        if let Some(values) = self.0.get::<_, Option<Table>>(name)? {
            return values.get(0); // Indexes starts from "0"
        }
        Ok(None)
    }
}

impl<'lua> FromLua<'lua> for Http<'lua> {
    #[inline]
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        Ok(Http(Table::from_lua(value, lua)?))
    }
}

impl<'lua> FromLua<'lua> for Headers<'lua> {
    #[inline]
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        Ok(Headers(Table::from_lua(value, lua)?))
    }
}

pub struct HeaderPairs<'lua, V: FromLua<'lua>> {
    pairs: TablePairs<'lua, LuaString<'lua>, Table<'lua>>,
    phantom: PhantomData<V>,
}

impl<'lua, V: FromLua<'lua>> Iterator for HeaderPairs<'lua, V> {
    type Item = Result<(String, Vec<V>)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.pairs.next() {
            Some(Ok(item)) => {
                let name = item.0.to_string_lossy().into_owned();
                let pairs = item.1.pairs::<i32, V>().collect::<Result<Vec<_>>>();
                match pairs {
                    Ok(mut pairs) => {
                        pairs.sort_by_key(|x| x.0);
                        Some(Ok((name, pairs.into_iter().map(|(_, v)| v).collect())))
                    }
                    Err(e) => Some(Err(e)),
                }
            }
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}
