mod cli;
mod context;
mod definition;
mod executor;
mod run;
mod shutdown;
mod types;

pub mod prelude {
    pub use crate::context::UserValuesConstraint;
    pub use crate::context::{AgentContext, RunnerContext};
    pub use crate::definition::{HookResult, ScenarioDefinitionBuilder};
    pub use crate::run::run;
    pub use crate::types::WindTunnelResult;
}
