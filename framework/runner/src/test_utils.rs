use std::sync::Arc;

use wind_tunnel_core::prelude::ShutdownHandle;
use wind_tunnel_instruments::{ReportConfig, Reporter};

pub(crate) fn test_reporter() -> Arc<Reporter> {
    let runtime = tokio::runtime::Handle::current();
    let shutdown_listener = ShutdownHandle::new().new_listener();
    Arc::new(
        ReportConfig::new("".to_string(), "".to_string())
            .enable_in_memory()
            .init_reporter(&runtime, shutdown_listener)
            .unwrap(),
    )
}
