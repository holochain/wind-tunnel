use holochain_client::ConductorApiError;
use std::io::ErrorKind;

pub fn handle_api_err(err: ConductorApiError) -> anyhow::Error {
    match err {
        // Handle websocket closed errors by shutting down the process, as this is a fatal error
        ConductorApiError::WebsocketError(e)
            if e.kind() == ErrorKind::Other && e.to_string() == "ConnectionClosed" =>
        {
            panic!("Conductor API connection closed unexpectedly: {:?}", e)
        }
        _ => anyhow::anyhow!("Conductor API error: {:?}", err),
    }
}
