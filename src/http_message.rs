use std::ops::Deref;

use mlua::{FromLua, Lua, LuaString, ObjectLike, Result, Table, Value};

use crate::{Channel, Headers};

/// For now, this class is only available from a filter context.
#[derive(Clone)]
pub struct HttpMessage(Table);

impl HttpMessage {
    /// Appends an HTTP header field in the HTTP message whose name is specified in `name`
    /// and value is defined in `value`.
    #[inline]
    pub fn add_header(&self, name: &str, value: impl AsRef<[u8]>) -> Result<()> {
        (self.0).call_method("add_header", (name, LuaString::wrap(value)))
    }

    /// Copies the string at the end of incoming data of the HTTP message.
    ///
    /// The function returns the copied length on success or -1 if data cannot be copied.
    #[inline]
    pub fn append(&self, data: impl AsRef<[u8]>) -> Result<isize> {
        self.0.call_method("append", LuaString::wrap(data))
    }

    /// Returns `length` bytes of incoming data from the HTTP message, starting at the `offset`.
    ///
    /// The data is not removed from the buffer.
    #[inline]
    pub fn body(&self, offset: Option<isize>, length: Option<isize>) -> Result<Option<LuaString>> {
        let offset = offset.unwrap_or(0);
        match length {
            Some(length) => self.0.call_method("body", (offset, length)),
            None => self.0.call_method("body", offset),
        }
    }

    /// Returns a corresponding channel attached to the HTTP message.
    #[inline]
    pub fn channel(&self) -> Result<Channel> {
        self.0.raw_get("channel")
    }

    /// Returns true if the end of message is reached.
    #[inline]
    pub fn eom(&self) -> Result<bool> {
        self.0.call_method("eom", ())
    }

    /// Removes all HTTP header fields in the HTTP message whose name is specified in name.
    #[inline]
    pub fn del_header(&self, name: &str) -> Result<()> {
        self.0.call_method("del_header", name)
    }

    /// Returns a table containing all the headers of the HTTP message.
    #[inline]
    pub fn get_headers(&self) -> Result<Headers> {
        self.0.call_method("get_headers", ())
    }

    /// Returns a table containing the start-line of the HTTP message.
    #[inline]
    pub fn get_stline(&self) -> Result<Table> {
        self.0.call_method("get_stline", ())
    }

    /// Forwards `length` bytes of data from the HTTP message.
    ///
    /// Returns the amount of data forwarded.
    ///
    /// Because it is called in the filter context, it never yield.
    /// Only available incoming data may be forwarded, even if the requested length exceeds the available amount.
    #[inline]
    pub fn forward(&self, length: usize) -> Result<usize> {
        self.0.call_method("forward", length)
    }

    /// Returns the length of incoming data in the HTTP message from the calling filter point of view.
    #[inline]
    pub fn input(&self) -> Result<usize> {
        self.0.call_method("input", ())
    }

    /// Copies the `data` at the `offset` in incoming data of the HTTP message.
    ///
    /// Returns the copied length on success or -1 if data cannot be copied.
    ///
    /// By default, if no `offset` is provided, the string is copied in front of incoming data.
    /// A positive `offset` is relative to the beginning of incoming data in the message buffer; a negative offset is relative to its end.
    #[inline]
    pub fn insert(&self, data: impl AsRef<[u8]>, offset: Option<isize>) -> Result<isize> {
        let offset = offset.unwrap_or(0);
        (self.0).call_method::<isize>("insert", (LuaString::wrap(data), offset))
    }

    /// Returns true if the HTTP message is full.
    #[inline]
    pub fn is_full(&self) -> Result<bool> {
        self.0.call_method("is_full", ())
    }

    /// Returns true if the HTTP message is the response one.
    #[inline]
    pub fn is_resp(&self) -> Result<bool> {
        self.0.call_method("is_resp", ())
    }

    /// Returns true if the HTTP message may still receive data.
    #[inline]
    pub fn may_recv(&self) -> Result<bool> {
        self.0.call_method("may_recv", ())
    }

    /// Returns the length of outgoing data of the HTTP message.
    #[inline]
    pub fn output(&self) -> Result<usize> {
        self.0.call_method("output", ())
    }

    /// Copies the `data` in front of incoming data of the HTTP message.
    ///
    /// Returns the copied length on success or -1 if data cannot be copied.
    #[inline]
    pub fn prepend(&self, data: impl AsRef<[u8]>) -> Result<isize> {
        (self.0).call_method::<isize>("prepend", LuaString::wrap(data))
    }

    /// Removes `length` bytes of incoming data of the HTTP message, starting at `offset`.
    ///
    /// Returns number of bytes removed on success.
    #[inline]
    pub fn remove(&self, offset: Option<isize>, length: Option<usize>) -> Result<isize> {
        let offset = offset.unwrap_or(0);
        match length {
            Some(length) => self.0.call_method("remove", (offset, length)),
            None => self.0.call_method("remove", offset),
        }
    }

    /// Matches the regular expression in all occurrences of header field `name` according to `regex`,
    /// and replaces them with the `replace`.
    ///
    /// The replacement value can contain back references like 1, 2, ...
    /// This function acts on whole header lines, regardless of the number of values they may contain.
    #[inline]
    pub fn rep_header(&self, name: &str, regex: &str, replace: &str) -> Result<()> {
        self.0.call_method("rep_header", (name, regex, replace))
    }

    /// Matches the regular expression on every comma-delimited value of header field `name` according to `regex`,
    /// and replaces them with the `replace`.
    ///
    /// The replacement value can contain back references like 1, 2, ...
    #[inline]
    pub fn rep_value(&self, name: &str, regex: &str, replace: &str) -> Result<()> {
        self.0.call_method("rep_value", (name, regex, replace))
    }

    /// Requires immediate send of the `data`.
    ///
    /// It means the `data` is copied at the beginning of incoming data of the HTTP message and immediately forwarded.
    ///
    /// Because it is called in the filter context, it never yield.
    #[inline]
    pub fn send(&self, data: impl AsRef<[u8]>) -> Result<isize> {
        self.0.call_method("send", LuaString::wrap(data))
    }

    /// Replaces `length` bytes of incoming data of the HTTP message, starting at `offset`, by the string `data`.
    ///
    /// Returns the copied length on success or -1 if data cannot be copied.
    #[inline]
    pub fn set(
        &self,
        data: impl AsRef<[u8]>,
        offset: Option<isize>,
        length: Option<usize>,
    ) -> Result<isize> {
        let data = LuaString::wrap(data);
        let offset = offset.unwrap_or(0);
        match length {
            Some(length) => self.0.call_method("set", (data, offset, length)),
            None => self.0.call_method("set", (data, offset)),
        }
    }

    /// Changes the expected payload length of the HTTP message.
    ///
    /// Returns `true` if the payload length was successfully updated, `false` otherwise.
    ///
    /// If `length` is `None`, the HTTP message is forced to be chunk-encoded.
    /// In that case, a `Transfer-Encoding` header is added with the “chunked” value.
    ///
    /// This function should be used in the filter context to be able to alter the payload of the HTTP message.
    #[inline]
    pub fn set_body_len(&self, length: Option<usize>) -> Result<bool> {
        match length {
            Some(length) => self.0.call_method("set_body_len", length),
            None => self.0.call_method("set_body_len", "chunked"),
        }
    }

    /// Sets or removes the flag that indicates end of message.
    #[inline]
    pub fn set_eom(&self, eom: bool) -> Result<()> {
        match eom {
            true => self.0.call_method("set_eom", ()),
            false => self.0.call_method("unset_eom", ()),
        }
    }

    /// Replaces all occurrence of all header matching the `name`, by only one containing the `value`.
    #[inline]
    pub fn set_header(&self, name: &str, value: impl AsRef<[u8]>) -> Result<()> {
        (self.0).call_method("set_header", (name, LuaString::wrap(value)))
    }

    /// Rewrites the request method.
    #[inline]
    pub fn set_method(&self, method: &str) -> Result<()> {
        self.0.call_method("set_method", method)
    }

    /// Rewrites the request path.
    #[inline]
    pub fn set_path(&self, path: &str) -> Result<()> {
        self.0.call_method("set_path", path)
    }

    /// Rewrites the request’s query string which appears after the first question mark "?".
    #[inline]
    pub fn set_query(&self, query: &str) -> Result<()> {
        self.0.call_method("set_query", query)
    }

    /// Rewrites the response status code with the new `status` and optional `reason`.
    /// If no custom reason is provided, it will be generated from the status.
    #[inline]
    pub fn set_status(&self, status: u16, reason: Option<&str>) -> Result<()> {
        self.0.call_method("set_status", (status, reason))
    }

    /// Rewrites the request URI.
    #[inline]
    pub fn set_uri(&self, uri: &str) -> Result<()> {
        self.0.call_method("set_uri", uri)
    }
}

impl FromLua for HttpMessage {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(HttpMessage(class))
    }
}

impl Deref for HttpMessage {
    type Target = Table;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
