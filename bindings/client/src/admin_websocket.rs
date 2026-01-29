use holo_hash::{ActionHash, DnaHash};
use holochain_client::{
    AdminWebsocket, AgentPubKey, AppInfo, AppStatusFilter, AuthorizeSigningCredentialsPayload,
    EnableAppResponse, InstallAppPayload, SigningCredentials,
};
use holochain_types::prelude::{CellId, DeleteCloneCellPayload, Record};
use holochain_zome_types::prelude::{DnaDef, GrantZomeCallCapabilityPayload};
use kitsune2_api::ApiTransportStats;
use std::sync::Arc;

use crate::ToSocketAddr;
use crate::error::handle_api_err;
use anyhow::Result;
use holochain_conductor_api::{
    AppAuthenticationTokenIssued, AppInterfaceInfo, IssueAppAuthenticationTokenPayload, StorageInfo,
};
use holochain_types::app::InstalledAppId;
use holochain_types::websocket::AllowedOrigins;
use wind_tunnel_instruments::Reporter;
use wind_tunnel_instruments_derive::wind_tunnel_instrument;

pub struct AdminWebsocketInstrumented {
    inner: AdminWebsocket,
    reporter: Arc<Reporter>,
}

impl AdminWebsocketInstrumented {
    pub async fn connect(
        admin_url: impl ToSocketAddr,
        origin: Option<String>,
        reporter: Arc<Reporter>,
    ) -> Result<Self> {
        let addr = admin_url.to_socket_addr()?;
        Ok(AdminWebsocket::connect(addr, origin)
            .await
            .map(|inner| Self { inner, reporter })?)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn generate_agent_pub_key(&self) -> anyhow::Result<AgentPubKey> {
        self.inner
            .generate_agent_pub_key()
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn list_app_interfaces(&self) -> anyhow::Result<Vec<AppInterfaceInfo>> {
        self.inner
            .list_app_interfaces()
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn attach_app_interface(
        &self,
        port: u16,
        danger_bind_addr: Option<String>,
        allowed_origins: AllowedOrigins,
        installed_app_id: Option<InstalledAppId>,
    ) -> anyhow::Result<u16> {
        self.inner
            .attach_app_interface(port, danger_bind_addr, allowed_origins, installed_app_id)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn list_apps(
        &self,
        status_filter: Option<AppStatusFilter>,
    ) -> anyhow::Result<Vec<AppInfo>> {
        self.inner
            .list_apps(status_filter)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn install_app(&self, payload: InstallAppPayload) -> anyhow::Result<AppInfo> {
        self.inner
            .install_app(payload)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn uninstall_app(&self, installed_app_id: String) -> anyhow::Result<()> {
        self.inner
            .uninstall_app(installed_app_id, false)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn enable_app(&self, installed_app_id: String) -> anyhow::Result<EnableAppResponse> {
        self.inner
            .enable_app(installed_app_id)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn disable_app(&self, installed_app_id: String) -> anyhow::Result<()> {
        self.inner
            .disable_app(installed_app_id)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")] // post = post_process_response
    pub async fn get_dna_definition(&self, cell_id: CellId) -> anyhow::Result<DnaDef> {
        self.inner
            .get_dna_definition(cell_id)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn grant_zome_call_capability(
        &self,
        capability: GrantZomeCallCapabilityPayload,
    ) -> anyhow::Result<ActionHash> {
        self.inner
            .grant_zome_call_capability(capability)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn delete_clone_cell(&self, payload: DeleteCloneCellPayload) -> anyhow::Result<()> {
        self.inner
            .delete_clone_cell(payload)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn storage_info(&self) -> anyhow::Result<StorageInfo> {
        self.inner.storage_info().await.map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn dump_network_stats(&self) -> anyhow::Result<ApiTransportStats> {
        self.inner
            .dump_network_stats()
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn graft_records(
        &self,
        cell_id: CellId,
        validate: bool,
        records: Vec<Record>,
    ) -> anyhow::Result<()> {
        self.inner
            .graft_records(cell_id, validate, records)
            .await
            .map_err(handle_api_err)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn agent_info(
        &self,
        dna_hashes: Option<Vec<DnaHash>>,
    ) -> anyhow::Result<Vec<String>> {
        self.inner
            .agent_info(dna_hashes)
            .await
            .map_err(handle_api_err)
    }

    // This is really a wrapper function, because it will call `grant_zome_call_capability` but it will call
    // that on the `AdminWebsocket` rather than this `AdminWebsocketInstrumented` type. So we need to instrument
    // this and include the time taken to do the client side logic for setting up signing credentials.
    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn authorize_signing_credentials(
        &self,
        request: AuthorizeSigningCredentialsPayload,
    ) -> anyhow::Result<SigningCredentials> {
        Ok(self
            .inner
            .authorize_signing_credentials(request)
            .await
            .map_err(handle_api_err)?)
    }

    #[wind_tunnel_instrument(prefix = "admin_")]
    pub async fn issue_app_auth_token(
        &self,
        payload: IssueAppAuthenticationTokenPayload,
    ) -> anyhow::Result<AppAuthenticationTokenIssued> {
        self.inner
            .issue_app_auth_token(payload)
            .await
            .map_err(handle_api_err)
    }
}

impl std::fmt::Debug for AdminWebsocketInstrumented {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminWebsocketInstrumented").finish()
    }
}
