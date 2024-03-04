use holochain_client_instrumented::prelude::AdminWebsocket;
use wind_tunnel_runner::prelude::UserValuesConstraint;

#[derive(Default, Debug)]
pub struct HolochainRunnerContext {
    pub app_port: Option<u16>,
}

impl UserValuesConstraint for HolochainRunnerContext {}
