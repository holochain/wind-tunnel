use wind_tunnel_runner::prelude::UserValuesConstraint;

#[derive(Default, Debug)]
pub struct HolochainAgentContext {
    pub value: String, // TODO store useful things like the app client
}

impl UserValuesConstraint for HolochainAgentContext {}
