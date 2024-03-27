mod cli;
mod context;
mod definition;
mod executor;
mod init;
mod monitor;
mod progress;
mod run;
mod shutdown;
mod types;

pub mod prelude {
    pub use crate::cli::{ReporterOpt, WindTunnelScenarioCli};
    pub use crate::context::UserValuesConstraint;
    pub use crate::context::{AgentContext, RunnerContext};
    pub use crate::definition::{HookResult, ScenarioDefinitionBuilder};
    pub use crate::executor::Executor;
    pub use crate::init::init;
    pub use crate::run::run;
    pub use crate::types::WindTunnelResult;

    // Re-export of the `wind_tunnel_instruments` prelude. This is for convenience so that you can
    // access reporting tools from within scenarios.
    pub use wind_tunnel_instruments::prelude::*;
}
