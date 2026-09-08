use std::marker::PhantomData;
use std::ops::Deref;

use mlua::{table::TablePairs, FromLua, IntoLua, Lua, LuaString, ObjectLike, Result, Table, Value};

#[derive(Clone)]
pub struct Http(Table);

#[derive(Clone)]
pub struct Headers(Table);

impl Http {
    /// Returns a `Headers` table containing all the request headers.
    #[inline]
    pub fn req_get_headers(&self) -> Result<Headers> {
        self.0.call_method("req_get_headers", ())
    }

    /// Returns a `Headers` table containing all the response headers.
    #[inline]
    pub fn res_get_headers(&self) -> Result<Headers> {
        self.0.call_method("res_get_headers", ())
    }

    /// Appends an HTTP header field `name` with `value` in the request.
    #[inline]
    pub fn req_add_header(&self, name: &str, value: impl IntoLua) -> Result<()> {
        self.0.call_method("req_add_header", (name, value))
    }

    /// Appends an HTTP header field `name` with `value` in the response.
    #[inline]
    pub fn res_add_header(&self, name: &str, value: impl IntoLua) -> Result<()> {
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
    pub fn req_set_header(&self, name: &str, value: impl IntoLua) -> Result<()> {
        self.0.call_method("req_set_header", (name, value))
    }

    /// Replaces all occurrence of HTTP response header `name`, by only one containing the `value`.
    #[inline]
    pub fn res_set_header(&self, name: &str, value: impl IntoLua) -> Result<()> {
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

impl Deref for Http {
    type Target = Table;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromLua for Http {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        Ok(Http(Table::from_lua(value, lua)?))
    }
}

impl Headers {
    #[inline]
    fn values(&self, name: &str) -> Result<Option<Table>> {
        if name.bytes().any(|byte| byte.is_ascii_uppercase()) {
            self.0.get(name.to_ascii_lowercase())
        } else {
            self.0.get(name)
        }
    }

    #[inline]
    pub fn pairs<V: FromLua>(&self) -> HeaderPairs<'_, V> {
        HeaderPairs {
            pairs: self.0.pairs(),
            phantom: PhantomData,
        }
    }

    /// Returns all header fields by `name`.
    #[inline]
    pub fn get<V: FromLua>(&self, name: &str) -> Result<Vec<V>> {
        let mut result = Vec::new();
        if let Some(values) = self.values(name)? {
            let mut pairs = values.pairs::<i32, V>().collect::<Result<Vec<_>>>()?;
            pairs.sort_by_key(|x| x.0);
            result = pairs.into_iter().map(|(_, v)| v).collect();
        }
        Ok(result)
    }

    /// Returns first header field by `name`.
    #[inline]
    pub fn get_first<V: FromLua>(&self, name: &str) -> Result<Option<V>> {
        if let Some(values) = self.values(name)? {
            return values.get(0); // Indexes starts from "0"
        }
        Ok(None)
    }
}

impl Deref for Headers {
    type Target = Table;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromLua for Headers {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        Ok(Headers(Table::from_lua(value, lua)?))
    }
}

pub struct HeaderPairs<'a, V: FromLua> {
    pairs: TablePairs<'a, LuaString, Table>,
    phantom: PhantomData<V>,
}

impl<V: FromLua> Iterator for HeaderPairs<'_, V> {
    type Item = Result<(String, Vec<V>)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.pairs.next() {
            Some(Ok(item)) => {
                let name = item.0.to_string_lossy();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_lookup_accepts_lowercase_and_mixed_case_names() {
        let lua = Lua::new();
        let headers = lua.create_table().unwrap();
        let values = lua.create_table().unwrap();
        values.set(0, "value").unwrap();
        headers.set("traceparent", values).unwrap();
        let headers = Headers(headers);

        assert_eq!(
            headers.get_first::<String>("traceparent").unwrap(),
            Some("value".to_owned())
        );
        assert_eq!(
            headers.get_first::<String>("TraceParent").unwrap(),
            Some("value".to_owned())
        );
    }
}
