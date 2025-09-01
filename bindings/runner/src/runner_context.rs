use wind_tunnel_runner::prelude::UserValuesConstraint;

/// Holochain-specific context values for the [wind_tunnel_runner::prelude::RunnerContext].
#[derive(Default, Debug)]
pub struct HolochainRunnerContext {
    pub(crate) app_ws_url: Option<String>,
}

impl UserValuesConstraint for HolochainRunnerContext {}

impl HolochainRunnerContext {
    /// Get the `app_ws_url` that was configured during setup.
    pub fn app_ws_url(&self) -> String {
        self.app_ws_url.clone().expect(
            "app_ws_url is not set, did you forget to call `configure_app_port` in your setup?",
        )
    }
}
