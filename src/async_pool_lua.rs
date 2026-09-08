use mlua::{RegistryKey, Result};

use crate::async_pool::{ObjectPool, PER_WORKER_POOL_SIZE};

#[mlua::userdata_impl]
impl ObjectPool {
    fn get(&mut self) -> Result<Option<RegistryKey>> {
        Ok(self.0.pop())
    }

    fn put(&mut self, obj: RegistryKey) -> Result<bool> {
        if self.0.len() == PER_WORKER_POOL_SIZE {
            return Ok(false);
        }
        self.0.push(obj);
        Ok(true)
    }
}
