mod context;
mod definition;
mod executor;
mod run;
mod types;
mod shutdown;
mod cli;

pub mod prelude {
    pub use crate::context::UserValuesConstraint;
    pub use crate::context::{Context, RunnerContext};
    pub use crate::definition::{HookResult, ScenarioDefinitionBuilder};
    pub use crate::run::run;
    pub use crate::types::WindTunnelResult;
}
