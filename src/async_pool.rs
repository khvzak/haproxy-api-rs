use mlua::{RegistryKey, UserData};

// Maximum number of pooled notification connections.
pub(super) const PER_WORKER_POOL_SIZE: usize = 512;

#[derive(UserData)]
pub(super) struct ObjectPool(pub(super) Vec<RegistryKey>);

impl ObjectPool {
    pub(super) fn new(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }
}
