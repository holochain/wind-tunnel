use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use anyhow::Result;
use holochain_client::{AgentSigner, AppAgentWebsocket, AppWebsocket, ConductorApiResult, ZomeCallTarget};
use holochain_types::app::InstalledAppId;
use std::sync::Arc;
use holochain_types::prelude::{ExternIO, FunctionName, ZomeName};
use wind_tunnel_instruments::Reporter;
use wind_tunnel_instruments_derive::wind_tunnel_instrument;
use crate::app_websocket::AppWebsocketInstrumented;

#[derive(Clone)]
pub struct AppAgentWebsocketInstrumented {
    /// A wrapper around the original [AppAgentWebsocket], which is NOT instrumented but we instrument it here.
    inner: AppAgentWebsocket,
    /// A wrapper around the original [AppWebsocket], which is instrumented. This is here so that we can Deref
    /// the [AppAgentWebsocketInstrumented] type and still have an instrumented [AppWebsocket].
    inner_instrumented: AppWebsocketInstrumented,
    reporter: Arc<Reporter>,
}

impl AppAgentWebsocketInstrumented {
    pub async fn connect(
        url: String,
        app_id: InstalledAppId,
        signer: Arc<Box<dyn AgentSigner + Send + Sync>>,
        reporter: Arc<Reporter>,
    ) -> Result<Self> {
        let app_ws = AppWebsocket::connect(url).await?;
        let inner = AppAgentWebsocket::from_existing(app_ws.clone(), app_id.clone(), signer.clone()).await?;
        let inner_instrumented = AppWebsocketInstrumented::from_existing(app_ws, reporter.clone()).await?;

        Ok(Self { inner, inner_instrumented, reporter })
    }

    pub async fn from_existing(
        app_ws: AppWebsocketInstrumented,
        app_id: InstalledAppId,
        signer: Arc<Box<dyn AgentSigner + Send + Sync>>
    ) -> Result<Self> {
        let inner_instrumented = app_ws.clone();
        AppAgentWebsocket::from_existing(app_ws.inner, app_id, signer)
            .await
            .map(|inner| Self { inner, inner_instrumented, reporter: app_ws.reporter })
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn call_zome(
        &mut self,
        target: ZomeCallTarget,
        zome_name: ZomeName,
        fn_name: FunctionName,
        payload: ExternIO,
    ) -> ConductorApiResult<ExternIO> {
        self.inner.call_zome(target, zome_name, fn_name, payload).await
    }
}

impl Deref for AppAgentWebsocketInstrumented {
    type Target = AppWebsocketInstrumented;

    fn deref(&self) -> &Self::Target {
        &self.inner_instrumented
    }
}

impl DerefMut for AppAgentWebsocketInstrumented {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner_instrumented
    }
}

impl Debug for AppAgentWebsocketInstrumented {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppAgentWebsocketInstrumented")
            .finish()
    }
}
