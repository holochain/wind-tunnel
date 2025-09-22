use wind_tunnel_runner::prelude::UserValuesConstraint;

/// Holochain-specific context values for the [wind_tunnel_runner::prelude::RunnerContext].
#[derive(Default, Debug)]
pub struct HolochainRunnerContext {}

impl UserValuesConstraint for HolochainRunnerContext {}
