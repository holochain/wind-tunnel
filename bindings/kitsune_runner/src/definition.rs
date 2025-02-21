use crate::cli::WindTunnelKitsuneScenarioCli;
use clap::Parser;
use wind_tunnel_runner::prelude::{ScenarioDefinitionBuilder, UserValuesConstraint};

pub struct KitsuneScenarioDefinitionBuilder<RV: UserValuesConstraint, AV: UserValuesConstraint> {
    inner: ScenarioDefinitionBuilder<RV, AV>,
}

impl<RV: UserValuesConstraint, AV: UserValuesConstraint> KitsuneScenarioDefinitionBuilder<RV, AV> {
    /// See [ScenarioDefinitionBuilder::new_with_init].
    ///
    /// This function uses [WindTunnelKitsuneScenarioCli] instead of [wind_tunnel_runner::prelude::WindTunnelScenarioCli].
    pub fn new_with_init(name: &str) -> anyhow::Result<Self> {
        env_logger::init();
        let cli = WindTunnelKitsuneScenarioCli::parse();
        Ok(Self {
            inner: ScenarioDefinitionBuilder::new(name, cli.try_into()?),
        })
    }

    /// Once the Kitsune customisations have been made, use this function to switch back to
    /// configuring default properties for the scenario.
    pub fn into_std(self) -> ScenarioDefinitionBuilder<RV, AV> {
        self.inner
    }
}
