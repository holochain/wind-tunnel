use tokio::signal;
use wind_tunnel_core::prelude::ShutdownHandle;

pub(crate) fn start_shutdown_listener(
    runtime: &tokio::runtime::Runtime,
) -> anyhow::Result<ShutdownHandle> {
    let (tx, _) = tokio::sync::broadcast::channel(1);

    let sender = tx.clone();
    runtime.spawn(async move {
        signal::ctrl_c()
            .await
            .expect("Failed to receive Ctrl-C signal");
        sender
            .send(())
            .expect("Received shutdown signal but failed to notify listeners");
        println!("Received shutdown signal, shutting down...");
    });

    Ok(ShutdownHandle::new(tx))
}
