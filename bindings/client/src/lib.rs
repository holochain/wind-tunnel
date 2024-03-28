use std::net::{SocketAddr, ToSocketAddrs};

mod admin_websocket;
mod app_agent_websocket;
mod app_websocket;

pub mod prelude {
    pub use crate::admin_websocket::AdminWebsocketInstrumented as AdminWebsocket;
    pub use crate::app_agent_websocket::AppAgentWebsocketInstrumented as AppAgentWebsocket;
    pub use crate::app_websocket::AppWebsocketInstrumented as AppWebsocket;

    // Types defined in other crates should be fetched directly, but types defined in the client
    // need to be re-exported here to avoid confusion from depending on this client wrapper and
    // the original client crate
    pub use holochain_client::{
        AgentSigner, AuthorizeSigningCredentialsPayload, ClientAgentSigner, EnableAppResponse,
        SigningCredentials,
    };
}

/// Conversions to a socket address to allow for example `ws://localhost:1234` to be used
/// where a socket address is needed.
pub trait ToSocketAddr {
    fn to_socket_addr(&self) -> anyhow::Result<SocketAddr>;
}

impl ToSocketAddr for &str {
    fn to_socket_addr(&self) -> anyhow::Result<SocketAddr> {
        let url: url::Url = (*self)
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to convert to URL: {:?}", e))?;

        Ok((
            url.host()
                .ok_or_else(|| anyhow::anyhow!("Missing host in URL"))?
                .to_string(),
            url.port()
                .ok_or_else(|| anyhow::anyhow!("Missing port in URL"))?,
        )
            .to_socket_addrs()
            .map_err(|e| anyhow::anyhow!("Failed to resolve host: {:?}", e))?
            .next()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve host"))?)
    }
}

impl ToSocketAddr for String {
    fn to_socket_addr(&self) -> anyhow::Result<SocketAddr> {
        self.as_str().to_socket_addr()
    }
}
