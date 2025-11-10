mod cli;
mod context;
mod definition;
mod executor;
mod holochain_binary;
mod init;
mod monitor;
mod progress;
mod run;
mod shutdown;
mod types;

pub use cli::parse_agent_behaviour;

pub mod prelude {
    pub use crate::cli::{ReporterOpt, WindTunnelScenarioCli};
    pub use crate::context::UserValuesConstraint;
    pub use crate::context::{AgentContext, RunnerContext};
    pub use crate::definition::{HookResult, ScenarioDefinitionBuilder};
    pub use crate::executor::Executor;
    pub use crate::holochain_binary::{
        holochain_build_info, holochain_path, WT_HOLOCHAIN_PATH_ENV,
    };
    pub use crate::init::init;
    pub use crate::run::run;
    pub use crate::types::WindTunnelResult;

    // Re-export of the `wind_tunnel_instruments` prelude. This is for convenience so that you can
    // access reporting tools from within scenarios.
    pub use wind_tunnel_instruments::prelude::*;
}
