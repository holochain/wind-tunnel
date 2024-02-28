#[cfg(feature = "holochain_0_2")]
use holochain_client_0_2::{AdminWebsocket, AgentPubKey, ConductorApiResult};

#[cfg(feature = "holochain_0_3")]
use holochain_client_0_3::{AdminWebsocket, AgentPubKey, ConductorApiResult};

use anyhow::Result;
use wind_tunnel_instruments_derive::wind_tunnel_instrument;

pub struct AdminWebsocketInstrumented(AdminWebsocket);

impl AdminWebsocketInstrumented {
    pub async fn connect(admin_url: String) -> Result<Self> {
        AdminWebsocket::connect(admin_url).await.map(Self)
    }

    pub fn close(&mut self) {
        self.0.close();
    }

    #[wind_tunnel_instrument]
    pub async fn generate_agent_pub_key(&mut self) -> ConductorApiResult<AgentPubKey> {
        self.0.generate_agent_pub_key().await
    }
}

impl std::fmt::Debug for AdminWebsocketInstrumented {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminWebsocketInstrumented").finish()
    }
}
