use holo_hash::DnaHash;
use holochain_client::{
    AdminWebsocket, AgentPubKey, AppInfo, AppStatusFilter, ConductorApiResult, EnableAppResponse,
    InstallAppPayload,
};
use holochain_types::prelude::{DeleteCloneCellPayload, Record};
use holochain_zome_types::prelude::{DnaDef, GrantZomeCallCapabilityPayload};

use anyhow::Result;
use holochain_conductor_api::StorageInfo;
use holochain_zome_types::CellId;
use wind_tunnel_instruments_derive::wind_tunnel_instrument;

pub struct AdminWebsocketInstrumented(AdminWebsocket);

impl AdminWebsocketInstrumented {
    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn connect(admin_url: String) -> Result<Self> {
        AdminWebsocket::connect(admin_url).await.map(Self)
    }

    pub fn close(&mut self) {
        self.0.close();
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn generate_agent_pub_key(&mut self) -> ConductorApiResult<AgentPubKey> {
        self.0.generate_agent_pub_key().await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn list_app_interfaces(&mut self) -> ConductorApiResult<Vec<u16>> {
        self.0.list_app_interfaces().await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn attach_app_interface(&mut self, port: u16) -> ConductorApiResult<u16> {
        self.0.attach_app_interface(port).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn list_apps(
        &mut self,
        status_filter: Option<AppStatusFilter>,
    ) -> ConductorApiResult<Vec<AppInfo>> {
        self.0.list_apps(status_filter).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn install_app(&mut self, payload: InstallAppPayload) -> ConductorApiResult<AppInfo> {
        self.0.install_app(payload).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn uninstall_app(&mut self, installed_app_id: String) -> ConductorApiResult<()> {
        self.0.uninstall_app(installed_app_id).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn enable_app(
        &mut self,
        installed_app_id: String,
    ) -> ConductorApiResult<EnableAppResponse> {
        self.0.enable_app(installed_app_id).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn disable_app(&mut self, installed_app_id: String) -> ConductorApiResult<()> {
        self.0.disable_app(installed_app_id).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")] // post = post_process_response
    pub async fn get_dna_definition(&mut self, dna_hash: DnaHash) -> ConductorApiResult<DnaDef> {
        self.0.get_dna_definition(dna_hash).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn grant_zome_call_capability(
        &mut self,
        capability: GrantZomeCallCapabilityPayload,
    ) -> ConductorApiResult<()> {
        self.0.grant_zome_call_capability(capability).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn delete_clone_cell(
        &mut self,
        payload: DeleteCloneCellPayload,
    ) -> ConductorApiResult<()> {
        self.0.delete_clone_cell(payload).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn storage_info(&mut self) -> ConductorApiResult<StorageInfo> {
        self.0.storage_info().await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn dump_network_stats(&mut self) -> ConductorApiResult<String> {
        self.0.dump_network_stats().await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn graft_records(
        &mut self,
        cell_id: CellId,
        validate: bool,
        records: Vec<Record>,
    ) -> ConductorApiResult<()>
    {
        self.0.graft_records(cell_id, validate, records).await
    }
}

impl std::fmt::Debug for AdminWebsocketInstrumented {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminWebsocketInstrumented").finish()
    }
}
