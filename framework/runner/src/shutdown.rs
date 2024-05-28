use tokio::signal;
use wind_tunnel_core::prelude::ShutdownHandle;

pub(crate) fn start_shutdown_listener(
    runtime: &tokio::runtime::Runtime,
) -> anyhow::Result<ShutdownHandle> {
    let handle = ShutdownHandle::default();

    let listener_handle = handle.clone();
    runtime.spawn(async move {
        signal::ctrl_c()
            .await
            .expect("Failed to receive Ctrl-C signal");
        listener_handle.shutdown();
        println!("Received shutdown signal, shutting down...");
    });

    Ok(handle)
}
