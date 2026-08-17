use prelude::{PeerkitAgentContext, PeerkitRunnerContext};

mod bin_path;
mod cli;
mod common;
mod context;
mod definition;
mod runner_context;

pub mod prelude {
    pub use super::{
        bin_path::{WT_PEERKIT_PATH_ENV, peerkit_bin_path},
        common::{
            agent_id_for_behaviour, connect_to_agent, derive_identity, get_relay_dial_addrs, run,
            send_text, shutdown_node, start_node, take_received_messages,
        },
        context::PeerkitAgentContext,
        definition::PeerkitScenarioDefinitionBuilder,
        runner_context::PeerkitRunnerContext,
    };

    pub use wind_tunnel_runner::prelude::*;
}
