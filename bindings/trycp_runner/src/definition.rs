use crate::cli::WindTunnelTryCPScenarioCli;
use clap::Parser;
use wind_tunnel_runner::prelude::{ScenarioDefinitionBuilder, UserValuesConstraint};

pub struct TryCPScenarioDefinitionBuilder<RV: UserValuesConstraint, V: UserValuesConstraint> {
    inner: ScenarioDefinitionBuilder<RV, V>,
}

impl<RV: UserValuesConstraint, V: UserValuesConstraint> TryCPScenarioDefinitionBuilder<RV, V> {
    /// See [ScenarioDefinitionBuilder::new_with_init].
    ///
    /// This function uses [WindTunnelTryCPScenarioCli] instead of [wind_tunnel_runner::prelude::WindTunnelScenarioCli].
    pub fn new_with_init(name: &str) -> anyhow::Result<Self> {
        env_logger::init();
        let cli = WindTunnelTryCPScenarioCli::parse();
        Ok(Self {
            inner: ScenarioDefinitionBuilder::new(name, cli.try_into()?),
        })
    }

    /// Once the TryCP customisations have been made, use this function to switch back to
    /// configuring default properties for the scenario.
    pub fn into_std(self) -> ScenarioDefinitionBuilder<RV, V> {
        // These environment variables are common to TryCP tests. Always capture them and just let
        // scenarios add any that are custom.
        self.inner
            .add_capture_env("CONDUCTOR_CONFIG")
            .add_capture_env("MIN_PEERS")
    }
}
