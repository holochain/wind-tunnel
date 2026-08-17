use crate::cli::WindTunnelPeerkitScenarioCli;
use clap::Parser;
use wind_tunnel_runner::prelude::{ScenarioDefinitionBuilder, UserValuesConstraint};

pub struct PeerkitScenarioDefinitionBuilder<RV: UserValuesConstraint, AV: UserValuesConstraint> {
    inner: ScenarioDefinitionBuilder<RV, AV>,
}

impl<RV: UserValuesConstraint, AV: UserValuesConstraint> PeerkitScenarioDefinitionBuilder<RV, AV> {
    /// See [ScenarioDefinitionBuilder::new_with_init].
    ///
    /// This function uses [WindTunnelPeerkitScenarioCli] instead of
    /// [wind_tunnel_runner::prelude::WindTunnelScenarioCli].
    pub fn new_with_init(name: &str) -> anyhow::Result<Self> {
        env_logger::init();
        let cli = WindTunnelPeerkitScenarioCli::parse();
        Ok(Self {
            inner: ScenarioDefinitionBuilder::new(name, cli.try_into()?),
        })
    }

    /// Once the Peerkit customisations have been made, use this function to
    /// switch back to configuring default properties for the scenario.
    pub fn into_std(self) -> ScenarioDefinitionBuilder<RV, AV> {
        self.inner
    }
}
