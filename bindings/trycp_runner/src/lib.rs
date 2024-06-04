mod cli;
mod common;
mod context;
mod definition;
mod runner_context;

pub mod prelude {
    pub use crate::cli::WindTunnelTryCPScenarioCli;
    pub use crate::common::{connect_trycp_client, disconnect_trycp_client, reset_trycp_remote};
    pub use crate::context::{DefaultScenarioValues, TryCPAgentContext};
    pub use crate::definition::TryCPScenarioDefinitionBuilder;
    pub use crate::runner_context::TryCPRunnerContext;

    /// Re-export of the `wind_tunnel_runner` prelude.
    ///
    /// This is for convenience so that you can depend on a single crate for the runner in your scenarios.
    pub use wind_tunnel_runner::prelude::*;
}
