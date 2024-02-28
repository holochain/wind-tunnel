use holochain_client_instrumented::prelude::AdminWebsocket;
use wind_tunnel_runner::prelude::UserValuesConstraint;

#[derive(Default, Debug)]
pub struct HolochainRunnerContext {
    pub admin_client: Option<AdminWebsocket>,
    pub value: usize, // TODO store useful things like the admin client
}

impl UserValuesConstraint for HolochainRunnerContext {}

impl HolochainRunnerContext {
    pub fn set_admin_client(&mut self, admin_client: AdminWebsocket) {
        self.admin_client = Some(admin_client);
    }

    pub fn admin_client(&mut self) -> &mut AdminWebsocket {
        self.admin_client.as_mut().expect("Admin client not set")
    }
}
