use std::sync::OnceLock;

use tokio::runtime::{self, Runtime};

/// Returns the process-wide Tokio runtime used by native and Lua integrations.
pub fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime")
    })
}
