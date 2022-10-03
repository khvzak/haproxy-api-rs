use mlua::{FromLua, Lua, Result, String as LuaString, Table, TableExt, Value};

use crate::{Channel, Headers};

/// This class contains all functions to manipulate an HTTP message.
/// For now, this class is only available from a filter context.
#[derive(Clone)]
pub struct HttpMessage<'lua> {
    lua: &'lua Lua,
    class: Table<'lua>,
}

impl<'lua> HttpMessage<'lua> {
    /// Appends an HTTP header field in the HTTP message whose name is specified in `name` and value is defined in `value`.
    #[inline]
    pub fn add_header(&self, name: &str, value: impl AsRef<[u8]>) -> Result<()> {
        let value = self.lua.create_string(value.as_ref())?;
        self.class.call_method("add_header", (name, value))
    }

    /// Copies the string at the end of incoming data of the HTTP message.
    /// The function returns the copied length on success or -1 if data cannot be copied.
    #[inline]
    pub fn append(&self, data: impl AsRef<[u8]>) -> Result<isize> {
        let data = self.lua.create_string(data.as_ref())?;
        self.class.call_method("append", data)
    }

    /// Returns `length` bytes of incoming data from the HTTP message, starting at the `offset`.
    /// The data are not removed from the buffer.
    #[inline]
    pub fn body(&self, offset: Option<isize>, length: Option<isize>) -> Result<Option<LuaString>> {
        let offset = offset.unwrap_or(0);
        match length {
            Some(length) => self.class.call_method("body", (offset, length)),
            None => self.class.call_method("body", offset),
        }
    }

    /// Returns a corresponding channel attached to the HTTP message.
    #[inline]
    pub fn channel(&self) -> Result<Channel> {
        self.class.raw_get("channel")
    }

    /// Returns true if the end of message is reached.
    #[inline]
    pub fn eom(&self) -> Result<bool> {
        self.class.call_method("eom", ())
    }

    /// Removes all HTTP header fields in the HTTP message whose name is specified in name.
    #[inline]
    pub fn del_header(&self, name: &str) -> Result<()> {
        self.class.call_method("del_header", name)
    }

    /// Returns a table containing all the headers of the HTTP message.
    #[inline]
    pub fn get_headers(&self) -> Result<Headers> {
        self.class.call_method("get_headers", ())
    }

    /// Returns a table containing the start-line of the HTTP message.
    #[inline]
    pub fn get_stline(&self) -> Result<Table> {
        self.class.call_method("get_stline", ())
    }

    /// Forwards `length` bytes of data from the HTTP message.
    /// Returns the amount of data forwarded.
    ///
    /// Because it is called in the filter context, it never yield.
    /// Only available incoming data may be forwarded, event if the requested length exceeds the available amount of incoming data.
    #[inline]
    pub fn forward(&self, length: usize) -> Result<usize> {
        self.class.call_method("forward", length)
    }

    /// Returns the length of incoming data in the HTTP message from the calling filter point of view.
    #[inline]
    pub fn input(&self) -> Result<usize> {
        self.class.call_method("input", ())
    }

    /// Copies the `data` at the `offset` in incoming data of the HTTP message.
    /// Returns the copied length on success or -1 if data cannot be copied.
    ///
    /// By default, if no `offset` is provided, the string is copied in front of incoming data.
    /// A positive `offset` is relative to the beginning of incoming data of the channel buffer while negative offset is relative to their end.
    #[inline]
    pub fn insert(&self, data: impl AsRef<[u8]>, offset: Option<isize>) -> Result<isize> {
        let data = self.lua.create_string(data.as_ref())?;
        let offset = offset.unwrap_or(0);
        self.class
            .call_method::<_, _, isize>("insert", (data, offset))
    }

    /// Returns true if the HTTP message is full.
    #[inline]
    pub fn is_full(&self) -> Result<bool> {
        self.class.call_method("is_full", ())
    }

    /// Returns true if the HTTP message is the response one.
    #[inline]
    pub fn is_resp(&self) -> Result<bool> {
        self.class.call_method("is_resp", ())
    }

    /// Returns true if the HTTP message may still receive data.
    #[inline]
    pub fn may_recv(&self) -> Result<bool> {
        self.class.call_method("may_recv", ())
    }

    /// Returns the length of outgoing data of the HTTP message.
    #[inline]
    pub fn output(&self) -> Result<usize> {
        self.class.call_method("output", ())
    }

    /// Copies the `data` in front of incoming data of the HTTP message.
    /// Returns the copied length on success or -1 if data cannot be copied.
    #[inline]
    pub fn prepend(&self, data: impl AsRef<[u8]>) -> Result<isize> {
        let data = self.lua.create_string(data.as_ref())?;
        self.class.call_method::<_, _, isize>("prepend", data)
    }

    /// Removes `length` bytes of incoming data of the HTTP message, starting at `offset`.
    /// Returns number of bytes removed on success.
    #[inline]
    pub fn remove(&self, offset: Option<isize>, length: Option<usize>) -> Result<isize> {
        let offset = offset.unwrap_or(0);
        match length {
            Some(length) => self.class.call_method("remove", (offset, length)),
            None => self.class.call_method("remove", offset),
        }
    }

    /// Matches the regular expression in all occurrences of header field `name` according to `regex`,
    /// and replaces them with the `replace`.
    ///
    /// The replacement value can contain back references like 1, 2, ...
    /// This function acts on whole header lines, regardless of the number of values they may contain.
    #[inline]
    pub fn rep_header(&self, name: &str, regex: &str, replace: &str) -> Result<()> {
        self.class.call_method("rep_header", (name, regex, replace))
    }

    /// Matches the regular expression on every comma-delimited value of header field `name` according to `regex`,
    /// and replaces them with the `replace`.
    ///
    /// The replacement value can contain back references like 1, 2, ...
    #[inline]
    pub fn rep_value(&self, name: &str, regex: &str, replace: &str) -> Result<()> {
        self.class.call_method("rep_value", (name, regex, replace))
    }

    /// Requires immediate send of the `data`.
    /// It means the `data` is copied at the beginning of incoming data of the HTTP message and immediately forwarded.
    ///
    /// Because it is called in the filter context, it never yield.
    #[inline]
    pub fn send(&self, data: impl AsRef<[u8]>) -> Result<isize> {
        let data = self.lua.create_string(data.as_ref())?;
        self.class.call_method("send", data)
    }

    /// Replaces `length` bytes of incoming data of the HTTP message, starting at `offset`, by the string `data`.
    /// Returns the copied length on success or -1 if data cannot be copied.
    #[inline]
    pub fn set(
        &self,
        data: impl AsRef<[u8]>,
        offset: Option<isize>,
        length: Option<usize>,
    ) -> Result<isize> {
        let data = self.lua.create_string(data.as_ref())?;
        let offset = offset.unwrap_or(0);
        match length {
            Some(length) => self.class.call_method("set", (data, offset, length)),
            None => self.class.call_method("set", (data, offset)),
        }
    }

    /// Sets or removes the flag that indicates end of message.
    #[inline]
    pub fn set_eom(&self, eom: bool) -> Result<()> {
        match eom {
            true => self.class.call_method("set_eom", ()),
            false => self.class.call_method("unset_eom", ()),
        }
    }

    /// Replaces all occurrence of all header matching the `name`, by only one containing the `value`.
    #[inline]
    pub fn set_header(&self, name: &str, value: impl AsRef<[u8]>) -> Result<()> {
        let value = self.lua.create_string(value.as_ref())?;
        self.class.call_method("set_header", (name, value))
    }

    /// Rewrites the request method.
    #[inline]
    pub fn set_method(&self, method: &str) -> Result<()> {
        self.class.call_method("set_method", method)
    }

    /// Rewrites the request path.
    #[inline]
    pub fn set_path(&self, path: &str) -> Result<()> {
        self.class.call_method("set_path", path)
    }

    /// Rewrites the request’s query string which appears after the first question mark "?".
    #[inline]
    pub fn set_query(&self, query: &str) -> Result<()> {
        self.class.call_method("set_query", query)
    }

    /// Rewrites the response status code with the new `status` and optional `reason`.
    /// If no custom reason is provided, it will be generated from the status.
    #[inline]
    pub fn set_status(&self, status: u16, reason: Option<&str>) -> Result<()> {
        self.class.call_method("set_status", (status, reason))
    }

    /// Rewrites the request URI.
    #[inline]
    pub fn set_uri(&self, uri: &str) -> Result<()> {
        self.class.call_method("set_uri", uri)
    }
}

impl<'lua> FromLua<'lua> for HttpMessage<'lua> {
    #[inline]
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(HttpMessage { lua, class })
    }
}
