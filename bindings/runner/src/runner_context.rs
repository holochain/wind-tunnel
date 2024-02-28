use wind_tunnel_runner::prelude::UserValuesConstraint;

#[derive(Default, Debug)]
pub struct HolochainRunnerContext {
    pub value: usize, // TODO store useful things like the admin client
}

impl UserValuesConstraint for HolochainRunnerContext {}
