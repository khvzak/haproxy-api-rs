use bstr::{BString, ByteSlice};
use mlua::{FromLua, Lua, Result, Table, TableExt, Value};

#[derive(Clone)]
pub struct Channel<'lua>(Table<'lua>);

impl<'lua> Channel<'lua> {
    pub fn dup(&self) -> Result<Option<BString>> {
        self.0.call_method("dup", ())
    }

    pub fn get(&self) -> Result<Option<BString>> {
        self.0.call_method("get", ())
    }

    pub fn getline(&self) -> Result<Option<BString>> {
        self.0.call_method("getline", ())
    }

    pub fn set<T: AsRef<[u8]> + ?Sized>(&self, buf: &T) -> Result<isize> {
        self.0.call_method("set", buf.as_ref().as_bstr())
    }

    pub fn append<T: AsRef<[u8]> + ?Sized>(&self, buf: &T) -> Result<isize> {
        self.0.call_method("append", buf.as_ref().as_bstr())
    }

    pub fn send<T: AsRef<[u8]> + ?Sized>(&self, buf: &T) -> Result<isize> {
        self.0.call_method("send", buf.as_ref().as_bstr())
    }

    pub fn get_in_length(&self) -> Result<usize> {
        self.0.call_method("get_in_len", ())
    }

    pub fn get_out_length(&self) -> Result<usize> {
        self.0.call_method("get_out_len", ())
    }

    pub fn forward(&self, size: usize) -> Result<()> {
        self.0.call_method("forward", size)
    }

    pub fn is_full(&self) -> Result<bool> {
        self.0.call_method("is_full", ())
    }
}

impl<'lua> FromLua<'lua> for Channel<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        Ok(Channel(Table::from_lua(value, lua)?))
    }
}
