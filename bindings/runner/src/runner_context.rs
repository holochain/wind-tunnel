use wind_tunnel_runner::prelude::UserValuesConstraint;

use crate::prelude::HolochainRunner;

/// Holochain-specific context values for the [wind_tunnel_runner::prelude::RunnerContext].
#[derive(Default, Debug)]
pub struct HolochainRunnerContext {
    pub(crate) holochain_runner: Option<HolochainRunner>,
}

impl UserValuesConstraint for HolochainRunnerContext {}
