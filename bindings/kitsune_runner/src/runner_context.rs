use wind_tunnel_runner::prelude::UserValuesConstraint;

/// Kitsune specific runner context values.
#[derive(Debug, Default)]
pub struct KitsuneRunnerContext;
impl UserValuesConstraint for KitsuneRunnerContext {}
