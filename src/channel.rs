use std::ops::Deref;

use mlua::{FromLua, IntoLua, Lua, LuaString, ObjectLike, Result, Table, Value};

#[derive(Clone)]
pub struct Channel(Table);

impl Channel {
    /// Appends `data` to incoming data in the channel buffer.
    ///
    /// Returns the copied length on success or -1 if data cannot be copied.
    #[inline]
    pub fn append(&self, data: impl AsRef<[u8]>) -> Result<isize> {
        self.0.call_method("append", LuaString::wrap(data))
    }

    /// Returns `length` bytes of incoming data from the channel buffer, starting at the `offset`.
    ///
    /// The data is not removed from the buffer.
    #[inline]
    pub fn data(&self, offset: Option<isize>, length: Option<isize>) -> Result<Option<LuaString>> {
        let offset = offset.unwrap_or(0);
        match length {
            Some(length) => self.0.call_method("data", (offset, length)),
            None => self.0.call_method("data", offset),
        }
    }

    /// Forwards `length` bytes of data from the channel buffer.
    ///
    /// Returns the amount of data forwarded and must not be called from an action to avoid yielding.
    #[inline]
    pub fn forward(&self, length: usize) -> Result<usize> {
        self.0.call_method("forward", length)
    }

    /// Returns the length of incoming data in the channel buffer.
    #[inline]
    pub fn input(&self) -> Result<usize> {
        self.0.call_method("input", ())
    }

    /// Inserts `data` into incoming channel data at the `offset`.
    ///
    /// Returns the copied length on success or -1 if data cannot be copied.
    ///
    /// By default, if no `offset` is provided, the string is copied in front of incoming data.
    /// A positive `offset` is relative to the beginning of incoming data in the channel buffer; a negative offset is relative to its end.
    #[inline]
    pub fn insert(&self, data: impl AsRef<[u8]>, offset: Option<isize>) -> Result<isize> {
        let offset = offset.unwrap_or(0);
        self.0
            .call_method("insert", (LuaString::wrap(data), offset))
    }

    /// Returns true if the channel buffer is full.
    #[inline]
    pub fn is_full(&self) -> Result<bool> {
        self.0.call_method("is_full", ())
    }

    /// Returns true if the channel is the response one.
    #[inline]
    pub fn is_resp(&self) -> Result<bool> {
        self.0.call_method("is_resp", ())
    }

    /// Parses `length` bytes of incoming data of the channel buffer, starting at `offset`,
    /// and returns the first line found, including the `\n`.
    ///
    /// The data is not removed from the buffer. If no line is found, all data is returned.
    #[inline]
    pub fn line(&self, offset: Option<isize>, length: Option<isize>) -> Result<Option<LuaString>> {
        let offset = offset.unwrap_or(0);
        match length {
            Some(length) => self.0.call_method("line", (offset, length)),
            None => self.0.call_method("line", offset),
        }
    }

    /// Returns true if the channel may still receive data.
    #[inline]
    pub fn may_recv(&self) -> Result<bool> {
        self.0.call_method("may_recv", ())
    }

    /// Returns the length of outgoing data of the channel buffer.
    #[inline]
    pub fn output(&self) -> Result<usize> {
        self.0.call_method("output", ())
    }

    /// Copies the `data` in front of incoming data of the channel buffer.
    ///
    /// Returns the copied length on success or -1 if data cannot be copied.
    #[inline]
    pub fn prepend(&self, data: impl AsRef<[u8]>) -> Result<isize> {
        self.0.call_method("prepend", LuaString::wrap(data))
    }

    /// Removes `length` bytes of incoming data of the channel buffer, starting at `offset`.
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

    /// Requires immediate send of the `data`.
    ///
    /// It means the `data` is copied at the beginning of incoming data of the channel buffer and immediately forwarded.
    #[inline]
    pub fn send(&self, data: impl AsRef<[u8]>) -> Result<isize> {
        self.0.call_method("send", LuaString::wrap(data))
    }

    /// Replaces `length` bytes of incoming data of the channel buffer, starting at `offset`, by the new `data`.
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
}

impl FromLua for Channel {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let class = Table::from_lua(value, lua)?;
        Ok(Channel(class))
    }
}

impl IntoLua for Channel {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Table(self.0))
    }
}

impl Deref for Channel {
    type Target = Table;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
