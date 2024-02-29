mod admin;
mod app_websocket;

pub mod prelude {
    pub use crate::admin::AdminWebsocketInstrumented as AdminWebsocket;
    pub use crate::app_websocket::AppWebsocketInstrumented as AppWebsocket;
}
