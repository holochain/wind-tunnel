use std::{borrow::BorrowMut, sync::Arc};

use tokio::{signal, sync::broadcast::{Receiver, Sender}};
use parking_lot::Mutex;
use crate::executor::Executor;

pub(crate) struct ShutdownListener {
    sender: Sender<()>,
}

impl ShutdownListener {
    pub(crate) fn new(sender: Sender<()>) -> Self {
        Self { sender }
    }

    pub(crate) fn new_listener(&self) -> DelegatedShutdownListener {
        DelegatedShutdownListener::new(self.sender.subscribe())
    }
}

#[derive(Clone)]
pub struct DelegatedShutdownListener {
    receiver: Arc<Mutex<Receiver<()>>>,
}

impl DelegatedShutdownListener {
    pub(crate) fn new(receiver: Receiver<()>) -> Self {
        Self { receiver: Arc::new(Mutex::new(receiver)) }
    }

    /// Point in time check if the shutdown signal has been received. If this returns true then work
    /// be stopped so that the scenario can shut down.
    pub fn should_shutdown(&mut self) -> bool {
        match self.receiver.lock().try_recv() {
            Ok(_) => true,
            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => true,
            // If the receiver is empty or lagged then we should not shutdown.
            Err(_) => false,
        }
    }

    /// Wait for the shutdown signal to be received. This will wait until the shutdown signal is
    /// received. It is safe to race this with another future so that the shutdown signal can be
    /// used to cancel other work in progress.
    pub async fn wait_for_shutdown(&mut self) {
        self.receiver.borrow_mut().lock().recv().await.expect("Failed to receive shutdown signal");
    }
}

pub(crate) fn start_shutdown_listener(executor: &Arc<Executor>) -> anyhow::Result<ShutdownListener> {
    let (tx, _) = tokio::sync::broadcast::channel(1);

    let sender = tx.clone();
    executor.submit(async move {
        signal::ctrl_c().await.expect("Failed to receive Ctrl-C signal");
        sender.send(()).expect("Received shutdown signal but failed to notify listeners");
        println!("Received shutdown signal, shutting down...");
    });

    Ok(ShutdownListener::new(tx))
}
