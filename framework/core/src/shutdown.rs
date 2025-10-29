use std::{borrow::BorrowMut, sync::Arc};

use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct ShutdownHandle {
    sender: Sender<()>,
}

impl Default for ShutdownHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownHandle {
    pub fn new() -> Self {
        Self {
            sender: tokio::sync::broadcast::channel(1).0,
        }
    }

    pub fn shutdown(&self) {
        if let Err(e) = self.sender.send(()) {
            // Will fail if nobody is listening for a shutdown signal, in which case the log message
            // can be ignored.
            log::warn!("Failed to send shutdown signal: {e:?}");
        }
    }

    pub fn new_listener(&self) -> DelegatedShutdownListener {
        DelegatedShutdownListener::new(self.sender.subscribe())
    }
}

#[derive(Clone, Debug)]
pub struct DelegatedShutdownListener {
    receiver: Arc<Mutex<Receiver<()>>>,
}

impl DelegatedShutdownListener {
    pub(crate) fn new(receiver: Receiver<()>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    /// Point in time check if the shutdown signal has been received. If this returns true then work
    /// be stopped so that the scenario can shut down.
    pub fn should_shutdown(&mut self) -> bool {
        match self.receiver.try_lock() {
            Ok(mut guard) => {
                match guard.try_recv() {
                    Ok(_) => true,
                    Err(tokio::sync::broadcast::error::TryRecvError::Closed) => true,
                    // If the receiver is empty or lagged then we should not shutdown.
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }

    /// Wait for the shutdown signal to be received. This will wait until the shutdown signal is
    /// received. It is safe to race this with another future so that the shutdown signal can be
    /// used to cancel other work in progress.
    pub async fn wait_for_shutdown(&mut self) {
        self.receiver
            .borrow_mut()
            .lock()
            .await
            .recv()
            .await
            .expect("Failed to receive shutdown signal");
    }
}

#[derive(derive_more::Error, derive_more::Display, Debug)]
pub struct ShutdownSignalError {
    msg: String,
}

impl Default for ShutdownSignalError {
    fn default() -> Self {
        Self {
            msg: "Execution cancelled by shutdown signal".to_string(),
        }
    }
}
