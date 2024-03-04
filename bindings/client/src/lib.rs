mod admin;
mod app_agent_websocket;
mod app_websocket;

pub mod prelude {
    pub use crate::admin::AdminWebsocketInstrumented as AdminWebsocket;
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
