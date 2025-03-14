use kitsune_client_instrumented::WtChatter;
use std::{fmt::Debug, sync::Arc};
use wind_tunnel_runner::prelude::UserValuesConstraint;

/// Kitsune specific agent context values.
#[derive(Debug, Default)]
pub struct KitsuneAgentContext {
    /// The chatter instance.
    pub(crate) chatter: Option<Arc<WtChatter>>,
    /// The number of messages to be create per interval.
    /// Defaults to 3.
    pub(crate) num_messages: Option<u8>,
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

    /// Get configured number of messages per interval.
    pub fn number_of_messages(&mut self) -> u8 {
        if self.num_messages.is_none() {
            let number_of_messages = std::env::var("NUM_MESSAGES")
                .unwrap_or("3".to_string())
                .parse()
                .expect("NUM_MESSAGES must be a number < 256");
            self.num_messages = Some(number_of_messages);
        }
        self.num_messages.expect("num messages must have been set")
    }
}
