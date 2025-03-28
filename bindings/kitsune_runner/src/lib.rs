use prelude::{KitsuneAgentContext, KitsuneRunnerContext};

mod cli;
mod common;
mod context;
mod definition;
mod runner_context;

pub mod prelude {
    pub use super::{
        common::{chatter_id, create_chatter, join_chatter_network, run, say},
        context::KitsuneAgentContext,
        definition::KitsuneScenarioDefinitionBuilder,
        runner_context::KitsuneRunnerContext,
    };

    pub use wind_tunnel_runner::prelude::*;
}
