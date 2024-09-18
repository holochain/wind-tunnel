use holochain_client::ConductorApiError;
use wind_tunnel_core::prelude::AgentBailError;

/// Handle a Conductor API error, returning an `anyhow::Error`.
///
/// If the error is a websocket closed error, this function will panic. There is currently no way to
/// reconnect websockets so once the connection drops, the scenario won't recover. It is better to
/// treat the error as fatal amd stop than keep logging errors until the scenario finishes.
pub fn handle_api_err(err: ConductorApiError) -> anyhow::Error {
    match err {
        // Handle websocket closed errors by shutting down the process, as this is a fatal error
        // for this agent.
        ConductorApiError::WebsocketError(holochain_websocket::WebsocketError::Close(_)) => {
            AgentBailError::default().into()
        }
        _ => anyhow::anyhow!("Conductor API error: {:?}", err),
    }
}
