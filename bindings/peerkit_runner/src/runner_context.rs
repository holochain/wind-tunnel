use wind_tunnel_runner::prelude::UserValuesConstraint;

/// Peerkit specific runner context values. Currently empty.
#[derive(Debug, Default)]
pub struct PeerkitRunnerContext {}

impl UserValuesConstraint for PeerkitRunnerContext {}
