use anyhow::Result;
use holochain_client::{AppWebsocket, ConductorApiResult};
use holochain_conductor_api::{AppInfo, ClonedCell, NetworkInfo, ZomeCall};
use holochain_types::app::{
    DisableCloneCellPayload, EnableCloneCellPayload, NetworkInfoRequestPayload,
};
use holochain_types::prelude::{CreateCloneCellPayload, InstalledAppId};
use holochain_zome_types::ExternIO;
use std::sync::Arc;
use wind_tunnel_instruments::{OperationRecord, Reporter};
use wind_tunnel_instruments_derive::wind_tunnel_instrument;

#[derive(Clone)]
pub struct AppWebsocketInstrumented {
    pub(crate) inner: AppWebsocket,
    pub(crate) reporter: Arc<Reporter>,
}

impl AppWebsocketInstrumented {
    pub async fn connect(app_url: String, reporter: Arc<Reporter>) -> Result<Self> {
        AppWebsocket::connect(app_url)
            .await
            .map(|inner| Self { inner, reporter })
    }

    pub async fn from_existing(app_ws: AppWebsocket, reporter: Arc<Reporter>) -> Result<Self> {
        Ok(Self {
            inner: app_ws,
            reporter,
        })
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn app_info(
        &mut self,
        app_id: InstalledAppId,
    ) -> ConductorApiResult<Option<AppInfo>> {
        self.inner.app_info(app_id).await
    }

    #[wind_tunnel_instrument(prefix = "app_", pre_hook = pre_call_zome)]
    pub async fn call_zome(&mut self, msg: ZomeCall) -> ConductorApiResult<ExternIO> {
        self.inner.call_zome(msg).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn create_clone_cell(
        &mut self,
        payload: CreateCloneCellPayload,
    ) -> ConductorApiResult<ClonedCell> {
        self.inner.create_clone_cell(payload).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn enable_clone_cell(
        &mut self,
        payload: EnableCloneCellPayload,
    ) -> ConductorApiResult<ClonedCell> {
        self.inner.enable_clone_cell(payload).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn disable_clone_cell(
        &mut self,
        payload: DisableCloneCellPayload,
    ) -> ConductorApiResult<()> {
        self.inner.disable_clone_cell(payload).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn network_info(
        &mut self,
        payload: NetworkInfoRequestPayload,
    ) -> ConductorApiResult<Vec<NetworkInfo>> {
        self.inner.network_info(payload).await
    }
}

fn pre_call_zome(operation_record: &mut OperationRecord, msg: &ZomeCall) {
    operation_record.add_attr("zome_name", msg.zome_name.0.to_string());
    operation_record.add_attr("fn_name", msg.fn_name.0.to_string());
}
