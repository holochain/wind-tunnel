use crate::ToSocketAddr;
use anyhow::Result;
use holochain_client::{AgentSigner, AppWebsocket, ConductorApiResult, ZomeCallTarget};
use holochain_conductor_api::{AppAuthenticationToken, AppInfo, NetworkInfo};
use holochain_types::app::{
    DisableCloneCellPayload, EnableCloneCellPayload, NetworkInfoRequestPayload,
};
use holochain_types::prelude::{CreateCloneCellPayload, ExternIO, FunctionName, Signal, ZomeName};
use holochain_zome_types::clone::ClonedCell;
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
        reporter: Arc<Reporter>,
    ) -> Result<Self> {
        AppWebsocket::connect(app_url.to_socket_addr()?, token, signer.clone())
            .await
            .map(|inner| Self { inner, reporter })
    }

    pub async fn on_signal<F>(&self, handler: F) -> Result<String>
    where
        F: Fn(Signal) + 'static + Sync + Send,
    {
        self.inner.on_signal(handler).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn app_info(&self) -> ConductorApiResult<Option<AppInfo>> {
        self.inner.app_info().await
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
    pub async fn network_info(
        &self,
        payload: NetworkInfoRequestPayload,
    ) -> ConductorApiResult<Vec<NetworkInfo>> {
        self.inner.network_info(payload).await
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
    match target {
        ZomeCallTarget::CellId(cell_id) => {
            operation_record.add_attr("agent", cell_id.agent_pubkey().to_string());
        }
        _ => {}
    }
}

impl Debug for AppWebsocketInstrumented {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppWebsocketInstrumented").finish()
    }
}
