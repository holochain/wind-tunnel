use std::future::Future;

#[derive(Debug)]
pub struct Executor {
    runtime: tokio::runtime::Runtime,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            runtime: tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"),
        }
    }

    /// Submit async code to be run in the background.
    pub fn submit(&self, fut: impl Future<Output = ()> + Send + 'static) {
        self.runtime.spawn(fut);
    }

    /// Run async code in place, blocking until it completes.
    pub fn execute(&self, fut: impl Future<Output = ()>) {
        self.runtime.block_on(fut);
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}
