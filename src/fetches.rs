use mlua::{
    BorrowedBytes, BorrowedStr, FromLua, IntoLuaMulti, Lua, LuaString, ObjectLike, Result, Table,
    Value, Variadic,
};

/// The "Fetches" class allows to call a lot of internal HAProxy sample fetches.
#[derive(Clone)]
pub struct Fetches(Table);

/// Request header values kept alive for a scoped callback.
pub struct RequestHeaders(Variadic<Option<LuaString>>);

impl RequestHeaders {
    /// Returns a header value as borrowed bytes, if present.
    #[inline]
    pub fn get(&self, index: usize) -> Option<BorrowedBytes> {
        self.0.get(index)?.as_ref().map(LuaString::as_bytes)
    }

    /// Returns a valid UTF-8 header value without copying its Lua-owned bytes.
    #[inline]
    pub fn get_str(&self, index: usize) -> Option<BorrowedStr> {
        self.0.get(index)?.as_ref()?.to_str().ok()
    }

    /// Returns the number of requested header values.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether no header values were requested.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Fetches {
    /// Executes an internal haproxy sample fetch.
    #[inline]
    pub fn get<R>(&self, name: &str, args: impl IntoLuaMulti) -> Result<R>
    where
        R: FromLua,
    {
        self.0.call_method(name, args)
    }

    /// The same as `get` but always returns string.
    #[inline]
    pub fn get_str(&self, name: &str, args: impl IntoLuaMulti) -> Result<String> {
        Ok((self.0.call_method::<Option<_>>(name, args)?).unwrap_or_default())
    }

    /// Returns a string sample fetch without copying its Lua string contents.
    #[inline]
    pub fn get_lua_str(&self, name: &str, args: impl IntoLuaMulti) -> Result<Option<LuaString>> {
        self.0.call_method(name, args)
    }

    /// Returns the first request header through HAProxy's direct `req.fhdr`
    /// sample fetch, without materializing the complete header table.
    #[inline]
    pub fn get_req_header(&self, name: &str) -> Result<Option<LuaString>> {
        self.get("req_fhdr", name)
    }

    /// Returns several request headers in one Lua call, preserving `names`
    /// order and using `None` for missing headers.
    #[inline]
    pub fn get_req_headers<'a, I>(&self, names: I) -> Result<Vec<Option<LuaString>>>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let names: Vec<&str> = names.into_iter().collect();
        if self.0.get::<Option<mlua::Function>>("req_fhdrs")?.is_none() {
            return names
                .into_iter()
                .map(|name| self.get_req_header(name))
                .collect();
        }
        self.0
            .call_method::<Variadic<Option<LuaString>>>(
                "req_fhdrs",
                Variadic::from_iter(names.iter().copied()),
            )
            .map(Into::into)
    }

    /// Runs a callback while a fetched Lua string is alive, without copying
    /// its bytes into a Rust-owned string.
    #[inline]
    pub fn with_req_header<R>(
        &self,
        name: &str,
        callback: impl FnOnce(Option<&[u8]>) -> R,
    ) -> Result<R> {
        let value = self.get_req_header(name)?;
        let bytes = value.as_ref().map(LuaString::as_bytes);
        Ok(callback(bytes.as_deref()))
    }

    /// Runs a callback while fetched request header strings are alive, without
    /// copying their bytes into a second Rust-owned collection.
    #[inline]
    pub fn with_req_headers<'a, I, R>(
        &self,
        names: I,
        callback: impl FnOnce(&RequestHeaders) -> R,
    ) -> Result<R>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let names: Vec<&str> = names.into_iter().collect();
        self.with_req_headers_slice(&names, callback)
    }

    /// Runs a callback for a fixed header-name slice without allocating a
    /// temporary Rust vector for the names.
    #[inline]
    pub fn with_req_headers_slice<R>(
        &self,
        names: &[&str],
        callback: impl FnOnce(&RequestHeaders) -> R,
    ) -> Result<R> {
        let values = if self.0.get::<Option<mlua::Function>>("req_fhdrs")?.is_none() {
            Variadic::from(
                names
                    .iter()
                    .copied()
                    .map(|name| self.get_req_header(name))
                    .collect::<Result<Vec<_>>>()?,
            )
        } else {
            self.0
                .call_method("req_fhdrs", Variadic::from_iter(names.iter().copied()))?
        };
        Ok(callback(&RequestHeaders(values)))
    }
}

impl FromLua for Fetches {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        Ok(Fetches(Table::from_lua(value, lua)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_req_header_borrows_lua_string_for_callback() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        let value = lua.create_string(b"example").unwrap();
        table
            .set(
                "req_fhdr",
                lua.create_function(move |_, (_table, _name): (Table, String)| {
                    Ok(Some(value.clone()))
                })
                .unwrap(),
            )
            .unwrap();

        let fetches = Fetches(table);
        let bytes = fetches
            .with_req_header("host", |value| value.map(ToOwned::to_owned))
            .unwrap();

        assert_eq!(bytes.as_deref(), Some(b"example".as_slice()));
    }

    #[test]
    fn with_req_headers_borrows_values_for_callback() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        let value = lua.create_string(b"example").unwrap();
        table
            .set(
                "req_fhdr",
                lua.create_function(move |_, (_table, _name): (Table, String)| {
                    Ok(Some(value.clone()))
                })
                .unwrap(),
            )
            .unwrap();

        let fetches = Fetches(table);
        let values = fetches
            .with_req_headers_slice(&["host", "traceparent"], |headers| {
                (
                    headers.len(),
                    headers.get(0).map(|value| value.as_ref().to_vec()),
                    headers.get(1).map(|value| value.as_ref().to_vec()),
                )
            })
            .unwrap();

        assert_eq!(values.0, 2);
        assert_eq!(values.1.as_deref(), Some(b"example".as_slice()));
        assert_eq!(values.2.as_deref(), Some(b"example".as_slice()));
    }

    #[test]
    fn with_req_headers_uses_the_batched_fetch_without_copying_values() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        let value = lua.create_string(b"example").unwrap();
        table
            .set(
                "req_fhdrs",
                lua.create_function(move |_, (_table, _names): (Table, Variadic<String>)| {
                    Ok(Variadic::from_iter([Some(value.clone()), None]))
                })
                .unwrap(),
            )
            .unwrap();

        let fetches = Fetches(table);
        let values = fetches
            .with_req_headers(["host", "traceparent"], |headers| {
                (
                    headers.len(),
                    headers.get(0).map(|value| value.as_ref().to_vec()),
                    headers.get(1).is_some(),
                )
            })
            .unwrap();

        assert_eq!(values.0, 2);
        assert_eq!(values.1.as_deref(), Some(b"example".as_slice()));
        assert!(!values.2);
    }

    #[test]
    fn request_headers_expose_valid_utf8_without_copying() {
        let lua = Lua::new();
        let headers = RequestHeaders(Variadic::from_iter([Some(
            lua.create_string("example").unwrap(),
        )]));

        assert_eq!(headers.get_str(0).as_deref(), Some("example"));
    }

    #[test]
    fn batched_fetch_errors_are_not_hidden_by_the_fallback() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table
            .set(
                "req_fhdrs",
                lua.create_function(|_, (_table, _names): (Table, Variadic<String>)| {
                    Err::<Variadic<Option<LuaString>>, _>(mlua::Error::RuntimeError(
                        "batch failed".into(),
                    ))
                })
                .unwrap(),
            )
            .unwrap();
        table
            .set(
                "req_fhdr",
                lua.create_function(|_, (_table, _name): (Table, String)| Ok(Some("fallback")))
                    .unwrap(),
            )
            .unwrap();

        let error = Fetches(table).get_req_headers(["host"]).unwrap_err();

        assert!(error.to_string().contains("batch failed"));
    }
}
