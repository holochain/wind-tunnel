mod common;

mod context;
mod holochain_sandbox;
mod macros;
mod runner_context;

pub mod prelude {
    /// Common operations for Holochain scenarios.
    ///
    /// This is a good place to start if you are getting started writing scenarios.
    pub use crate::common::*;

    pub use crate::context::HolochainAgentContext;
    pub use crate::holochain_sandbox::HolochainSandbox;
    pub use crate::runner_context::HolochainRunnerContext;

    /// Re-export of the `wind_tunnel_runner` prelude.
    ///
    /// This is for convenience so that you can depend on a single crate for the runner in your scenarios.
    pub use wind_tunnel_runner::prelude::*;

    /// Re-export of the instrumented client for convenience.
    pub use holochain_client_instrumented::prelude::*;
}
