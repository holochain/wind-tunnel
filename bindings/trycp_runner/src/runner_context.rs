use wind_tunnel_runner::prelude::UserValuesConstraint;

/// TryCP-specific context values for the [wind_tunnel_runner::prelude::RunnerContext].
#[derive(Default, Debug)]
pub struct TryCPRunnerContext {}

impl UserValuesConstraint for TryCPRunnerContext {}

impl TryCPRunnerContext {}
