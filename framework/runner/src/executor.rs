use std::future::Future;

pub struct Executor {
    runtime: tokio::runtime::Runtime,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            runtime: tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"),
        }
    }

    pub fn execute(&self, fut: impl Future<Output = ()>) {
        self.runtime.block_on(fut);
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}
