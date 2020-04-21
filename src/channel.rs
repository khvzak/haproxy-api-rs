use bstr::BString;
use mlua::{FromLua, Lua, Result, Table, TableExt, Value};

pub struct Channel<'lua>(&'lua Lua, Table<'lua>);

impl<'lua> Channel<'lua> {
    pub fn dup(&self) -> Result<Option<BString>> {
        self.1.call_method("dup", ())
    }

    pub fn get(&self) -> Result<Option<BString>> {
        self.1.call_method("get", ())
    }

    pub fn getline(&self) -> Result<Option<BString>> {
        self.1.call_method("getline", ())
    }

    // TODO: Use BStr
    pub fn set(&self, buf: BString) -> Result<isize> {
        self.1.call_method("set", buf)
    }

    pub fn append(&self, buf: BString) -> Result<isize> {
        self.1.call_method("append", buf)
    }

    pub fn send(&self, buf: BString) -> Result<isize> {
        self.1.call_method("send", buf)
    }

    pub fn get_in_length(&self) -> Result<usize> {
        self.1.call_method("get_in_length", ())
    }

    pub fn get_out_length(&self) -> Result<usize> {
        self.1.call_method("get_out_length", ())
    }

    pub fn forward(&self, size: usize) -> Result<()> {
        self.1.call_method("forward", size)
    }

    pub fn is_full(&self) -> Result<bool> {
        self.1.call_method("is_full", ())
    }
}

impl<'lua> FromLua<'lua> for Channel<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let t = Table::from_lua(value, lua)?;
        Ok(Channel(lua, t))
    }
}
