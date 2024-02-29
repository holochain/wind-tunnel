use holochain_client::{AppWebsocket, ConductorApiResult};
use anyhow::Result;
use holochain_conductor_api::{AppInfo, ClonedCell, NetworkInfo, ZomeCall};
use holochain_types::app::{DisableCloneCellPayload, EnableCloneCellPayload, NetworkInfoRequestPayload};
use holochain_types::prelude::{CreateCloneCellPayload, InstalledAppId};
use holochain_zome_types::ExternIO;
use wind_tunnel_instruments_derive::wind_tunnel_instrument;

pub struct AppWebsocketInstrumented(AppWebsocket);

impl AppWebsocketInstrumented {
    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn connect(app_url: String) -> Result<Self> {
        AppWebsocket::connect(app_url).await.map(Self)
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn app_info(
        &mut self,
        app_id: InstalledAppId,
    ) -> ConductorApiResult<Option<AppInfo>> {
        self.0.app_info(app_id).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn call_zome(&mut self, msg: ZomeCall) -> ConductorApiResult<ExternIO> {
        self.0.call_zome(msg).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn create_clone_cell(
        &mut self,
        payload: CreateCloneCellPayload,
    ) -> ConductorApiResult<ClonedCell> {
        self.0.create_clone_cell(payload).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn enable_clone_cell(
        &mut self,
        payload: EnableCloneCellPayload,
    ) -> ConductorApiResult<ClonedCell> {
        self.0.enable_clone_cell(payload).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn disable_clone_cell(
        &mut self,
        payload: DisableCloneCellPayload,
    ) -> ConductorApiResult<()> {
        self.0.disable_clone_cell(payload).await
    }

    #[wind_tunnel_instrument(prefix = "app_")]
    pub async fn network_info(&mut self, payload: NetworkInfoRequestPayload) -> ConductorApiResult<Vec<NetworkInfo>> {
        self.0.network_info(payload).await
    }
}
