use crate::ToSocketAddr;
use crate::error::handle_api_err;
use anyhow::Result;
use holo_hash::DnaHash;
use holochain_client::{AgentSigner, AppWebsocket, CallZomeOptions, ZomeCallTarget};
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

    pub async fn on_signal<F>(&self, handler: F) -> Result<String>
    where
        F: Fn(Signal) + 'static + Sync + Send,
    {
        Ok(self.inner.on_signal(handler).await)
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn app_info(&self) -> anyhow::Result<Option<AppInfo>> {
        self.inner.app_info().await.map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn agent_info(
        &self,
        dna_hashes: Option<Vec<DnaHash>>,
    ) -> anyhow::Result<Vec<String>> {
        self.inner
            .agent_info(dna_hashes)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "app_", pre_hook = pre_call_zome)]
    pub async fn call_zome(
        &self,
        target: ZomeCallTarget,
        zome_name: impl Into<ZomeName> + Clone,
        fn_name: impl Into<FunctionName> + Clone,
        payload: ExternIO,
        options: CallZomeOptions,
    ) -> anyhow::Result<ExternIO> {
        self.inner
            .call_zome_with_options(target, zome_name.into(), fn_name.into(), payload, options)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn create_clone_cell(
        &self,
        payload: CreateCloneCellPayload,
    ) -> anyhow::Result<ClonedCell> {
        self.inner
            .create_clone_cell(payload)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn enable_clone_cell(
        &self,
        payload: EnableCloneCellPayload,
    ) -> anyhow::Result<ClonedCell> {
        self.inner
            .enable_clone_cell(payload)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn disable_clone_cell(&self, payload: DisableCloneCellPayload) -> anyhow::Result<()> {
        self.inner
            .disable_clone_cell(payload)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn dump_network_stats(&self) -> anyhow::Result<TransportStats> {
        self.inner
            .dump_network_stats()
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn dump_network_metrics(
        &self,
        dna_hash: Option<DnaHash>,
        include_dht_summary: bool,
    ) -> anyhow::Result<std::collections::HashMap<DnaHash, Kitsune2NetworkMetrics>> {
        self.inner
            .dump_network_metrics(dna_hash, include_dht_summary)
            .await
            .map_err(handle_api_err)
    }
}

fn pre_call_zome(
    operation_record: &mut OperationRecord,
    target: &ZomeCallTarget,
    zome_name: &(impl Into<ZomeName> + Clone),
    fn_name: &(impl Into<FunctionName> + Clone),
    _payload: &ExternIO,
    _options: &CallZomeOptions,
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
