use holo_hash::DnaHash;
use holochain_client::{
    AdminWebsocket, AgentPubKey, AppInfo, AppStatusFilter, AuthorizeSigningCredentialsPayload,
    ConductorApiResult, EnableAppResponse, InstallAppPayload, SigningCredentials,
};
use holochain_types::prelude::{CellId, DeleteCloneCellPayload, Record};
use holochain_zome_types::prelude::{DnaDef, GrantZomeCallCapabilityPayload};
use std::sync::Arc;

use crate::ToSocketAddr;
use anyhow::Result;
use holochain_conductor_api::StorageInfo;
use holochain_types::websocket::AllowedOrigins;
use wind_tunnel_instruments::Reporter;
use wind_tunnel_instruments_derive::wind_tunnel_instrument;

pub struct AdminWebsocketInstrumented {
    inner: AdminWebsocket,
    reporter: Arc<Reporter>,
}

impl AdminWebsocketInstrumented {
    pub async fn connect(admin_url: impl ToSocketAddr, reporter: Arc<Reporter>) -> Result<Self> {
        AdminWebsocket::connect(admin_url.to_socket_addr()?)
            .await
            .map(|inner| Self { inner, reporter })
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn generate_agent_pub_key(&mut self) -> ConductorApiResult<AgentPubKey> {
        self.inner.generate_agent_pub_key().await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn list_app_interfaces(&mut self) -> ConductorApiResult<Vec<u16>> {
        self.inner.list_app_interfaces().await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn attach_app_interface(
        &mut self,
        port: u16,
        allowed_origins: AllowedOrigins,
    ) -> ConductorApiResult<u16> {
        self.inner.attach_app_interface(port, allowed_origins).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn list_apps(
        &mut self,
        status_filter: Option<AppStatusFilter>,
    ) -> ConductorApiResult<Vec<AppInfo>> {
        self.inner.list_apps(status_filter).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn install_app(&mut self, payload: InstallAppPayload) -> ConductorApiResult<AppInfo> {
        self.inner.install_app(payload).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn uninstall_app(&mut self, installed_app_id: String) -> ConductorApiResult<()> {
        self.inner.uninstall_app(installed_app_id).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn enable_app(
        &mut self,
        installed_app_id: String,
    ) -> ConductorApiResult<EnableAppResponse> {
        self.inner.enable_app(installed_app_id).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn disable_app(&mut self, installed_app_id: String) -> ConductorApiResult<()> {
        self.inner.disable_app(installed_app_id).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")] // post = post_process_response
    pub async fn get_dna_definition(&mut self, dna_hash: DnaHash) -> ConductorApiResult<DnaDef> {
        self.inner.get_dna_definition(dna_hash).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn grant_zome_call_capability(
        &mut self,
        capability: GrantZomeCallCapabilityPayload,
    ) -> ConductorApiResult<()> {
        self.inner.grant_zome_call_capability(capability).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn delete_clone_cell(
        &mut self,
        payload: DeleteCloneCellPayload,
    ) -> ConductorApiResult<()> {
        self.inner.delete_clone_cell(payload).await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn storage_info(&mut self) -> ConductorApiResult<StorageInfo> {
        self.inner.storage_info().await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn dump_network_stats(&mut self) -> ConductorApiResult<String> {
        self.inner.dump_network_stats().await
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn graft_records(
        &mut self,
        cell_id: CellId,
        validate: bool,
        records: Vec<Record>,
    ) -> ConductorApiResult<()> {
        self.inner.graft_records(cell_id, validate, records).await
    }

    // This is really a wrapper function, because it will call `grant_zome_call_capability` but it will call
    // that on the `AdminWebsocket` rather than this `AdminWebsocketInstrumented` type. So we need to instrument
    // this and include the time taken to do the client side logic for setting up signing credentials.
    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn authorize_signing_credentials(
        &mut self,
        request: AuthorizeSigningCredentialsPayload,
    ) -> Result<SigningCredentials> {
        self.inner.authorize_signing_credentials(request).await
    }
}

impl std::fmt::Debug for AdminWebsocketInstrumented {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminWebsocketInstrumented").finish()
    }
}
