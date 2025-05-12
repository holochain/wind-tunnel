use kitsune_client_instrumented::WtChatter;
use std::{fmt::Debug, sync::Arc};
use wind_tunnel_runner::prelude::UserValuesConstraint;

/// Kitsune specific agent context values.
#[derive(Debug, Default)]
pub struct KitsuneAgentContext {
    /// The chatter instance.
    pub(crate) chatter: Option<Arc<WtChatter>>,
}

impl UserValuesConstraint for KitsuneAgentContext {}

impl KitsuneAgentContext {
    /// Get chatter instance.
    pub fn chatter(&self) -> Arc<WtChatter> {
        self.chatter.clone().expect(
            "chatter is not set, did you forget to call `create_chatter` in your agent setup?",
        )
    }

    /// Get chatter id.
    pub fn chatter_id(&self) -> String {
        self.chatter
            .clone()
            .expect(
                "chatter is not set, did you forget to call `create_chatter` in your agent setup?",
            )
            .id()
            .to_string()
    }
}
