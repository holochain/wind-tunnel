use crate::ToSocketAddr;
use anyhow::Result;
use holo_hash::DnaHash;
use holochain_client::{
    AgentSigner, AppWebsocket, ConductorApiResult, WebsocketConfig, ZomeCallTarget,
};
use holochain_conductor_api::{AppAuthenticationToken, AppInfo};
use holochain_types::app::{DisableCloneCellPayload, EnableCloneCellPayload};
use holochain_types::prelude::{
    CreateCloneCellPayload, ExternIO, FunctionName, Kitsune2NetworkMetrics, Signal, ZomeName,
};
use holochain_zome_types::clone::ClonedCell;
use kitsune2_api::TransportStats;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use wind_tunnel_instruments::{OperationRecord, Reporter};
use wind_tunnel_instruments_derive::wind_tunnel_instrument;

#[derive(Clone)]
pub struct AppWebsocketInstrumented {
    pub(crate) inner: AppWebsocket,
    pub(crate) reporter: Arc<Reporter>,
}

impl AppWebsocketInstrumented {
    pub async fn connect(
        app_url: impl ToSocketAddr,
        token: AppAuthenticationToken,
        signer: Arc<dyn AgentSigner + Send + Sync>,
        origin: Option<String>,
        reporter: Arc<Reporter>,
    ) -> Result<Self> {
        Ok(
            AppWebsocket::connect(app_url.to_socket_addr()?, token, signer.clone(), origin)
                .await
                .map(|inner| Self { inner, reporter })?,
        )
    }

    /// Connect to a Conductor API app websocket with a custom [`WebsocketConfig`].
    pub async fn connect_with_config(
        app_url: impl ToSocketAddr,
        websocket_config: Arc<WebsocketConfig>,
        token: AppAuthenticationToken,
        signer: Arc<dyn AgentSigner + Send + Sync>,
        origin: Option<String>,
        reporter: Arc<Reporter>,
    ) -> Result<Self> {
        Ok(AppWebsocket::connect_with_config(
            app_url.to_socket_addr()?,
            websocket_config,
            token,
            signer.clone(),
            origin,
        )
        .await
        .map(|inner| Self { inner, reporter })?)
    }

    pub async fn on_signal<F>(&self, handler: F) -> Result<String>
    where
        F: Fn(Signal) + 'static + Sync + Send,
    {
        Ok(self.inner.on_signal(handler).await)
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn app_info(&self) -> ConductorApiResult<Option<AppInfo>> {
        self.inner.app_info().await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn agent_info(
        &self,
        dna_hashes: Option<Vec<DnaHash>>,
    ) -> ConductorApiResult<Vec<String>> {
        self.inner.agent_info(dna_hashes).await
    }

    #[wind_tunnel_instrument(prefix = "app_", pre_hook = pre_call_zome)]
    pub async fn call_zome(
        &self,
        target: ZomeCallTarget,
        zome_name: impl Into<ZomeName> + Clone,
        fn_name: impl Into<FunctionName> + Clone,
        payload: ExternIO,
    ) -> ConductorApiResult<ExternIO> {
        self.inner
            .call_zome(target, zome_name.into(), fn_name.into(), payload)
            .await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn create_clone_cell(
        &self,
        payload: CreateCloneCellPayload,
    ) -> ConductorApiResult<ClonedCell> {
        self.inner.create_clone_cell(payload).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn enable_clone_cell(
        &self,
        payload: EnableCloneCellPayload,
    ) -> ConductorApiResult<ClonedCell> {
        self.inner.enable_clone_cell(payload).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn disable_clone_cell(
        &self,
        payload: DisableCloneCellPayload,
    ) -> ConductorApiResult<()> {
        self.inner.disable_clone_cell(payload).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn dump_network_stats(&self) -> ConductorApiResult<TransportStats> {
        self.inner.dump_network_stats().await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn dump_network_metrics(
        &self,
        dna_hash: Option<DnaHash>,
        include_dht_summary: bool,
    ) -> ConductorApiResult<std::collections::HashMap<DnaHash, Kitsune2NetworkMetrics>> {
        self.inner
            .dump_network_metrics(dna_hash, include_dht_summary)
            .await
    }
}

fn pre_call_zome(
    operation_record: &mut OperationRecord,
    target: &ZomeCallTarget,
    zome_name: &(impl Into<ZomeName> + Clone),
    fn_name: &(impl Into<FunctionName> + Clone),
    _payload: &ExternIO,
) {
    let zome_name: ZomeName = zome_name.clone().into();
    let fn_name: FunctionName = fn_name.clone().into();
    operation_record.add_attr("zome_name", zome_name.0.to_string());
    operation_record.add_attr("fn_name", fn_name.0.to_string());
    if let ZomeCallTarget::CellId(cell_id) = target {
        operation_record.add_attr("agent", cell_id.agent_pubkey().to_string());
    }
}

impl Debug for AppWebsocketInstrumented {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppWebsocketInstrumented").finish()
    }
}
