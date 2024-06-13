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

    pub use trycp_client_instrumented::prelude::*;

    /// Re-export of the `wind_tunnel_runner` prelude.
    ///
    /// This is for convenience so that you can depend on a single crate for the runner in your scenarios.
    pub use wind_tunnel_runner::prelude::*;

    /// Re-export some of the `holochain_wind_tunnel_runner`.
    ///
    /// This is really a runner for a separate purpose but some of its functionality is useful for
    /// the TryCP runner. It doesn't make sense to include both in scenarios, so this is a way to
    /// make functionality available without coping it.
    pub use holochain_wind_tunnel_runner::scenario_happ_path;

    /// Re-export types from the Holochain crates that shouldn't need to be imported into every scenario
    pub use holochain_conductor_api::{CellInfo, IssueAppAuthenticationTokenPayload};
}
