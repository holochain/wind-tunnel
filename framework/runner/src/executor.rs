use std::future::Future;

use crate::shutdown::{ShutdownHandle, ShutdownSignalError};

#[derive(Debug)]
pub struct Executor {
    runtime: tokio::runtime::Runtime,
    shutdown_handle: ShutdownHandle,
}

impl Executor {
    pub(crate) fn new(runtime: tokio::runtime::Runtime, shutdown_handle: ShutdownHandle) -> Self {
        Self {
            runtime,
            shutdown_handle,
        }
    }

    /// Run async code in place, blocking until it completes.
    ///
    /// Note that the future will be cancelled if the runner is shutdown. You do not need to do anything
    /// special to handle this, but you should be aware that submitting a future which does not support
    /// cancelling may prevent the runner from shutting down.
    pub fn execute_in_place<T>(
        &self,
        fut: impl Future<Output = anyhow::Result<T>>,
    ) -> anyhow::Result<T> {
        let mut shutdown_listener = self.shutdown_handle.new_listener();
        self.runtime.block_on(async move {
            tokio::select! {
                result = fut => result,
                _ = shutdown_listener.wait_for_shutdown() => {
                    Err(anyhow::anyhow!(ShutdownSignalError::default()))
                },
            }
        })
    }

    /// Submit async code to be run in the background.
    ///
    /// Note that the future will not be cancelled if the runner is shutdown. It is also not guaranteed
    /// that the runner will wait for the future to complete before shutting down.
    ///
    /// In agent behaviour hooks, you should use [Executor::execute] instead of [Executor::submit] to ensure that your
    /// your future completes before the behaviour completes and is scheduled again.
    pub fn spawn(&self, fut: impl Future<Output = ()> + Send + 'static) {
        self.runtime.spawn(fut);
    }
}
