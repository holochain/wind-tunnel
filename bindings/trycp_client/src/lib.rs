use holochain_client::AgentSigner;
use holochain_conductor_api::ZomeCall;
use holochain_types::prelude::{FunctionName, ZomeName};
use holochain_zome_types::cell::CellId;
use holochain_zome_types::prelude::ZomeCallUnsigned;
use serde::de::DeserializeOwned;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use trycp_api::MessageResponse;
use trycp_client::{Request, SignalRecv, TrycpClient};
use wind_tunnel_instruments::{OperationRecord, Reporter};
use wind_tunnel_instruments_derive::wind_tunnel_instrument;

pub mod prelude {
    pub use super::TryCPClientInstrumented as TryCPClient;

    pub use holochain_client::AuthorizeSigningCredentialsPayload;
}

#[derive(Clone)]
pub struct TryCPClientInstrumented {
    trycp_client: Arc<TrycpClient>,
    signal_recv: Arc<tokio::sync::Mutex<SignalRecv>>,
    signer: Arc<dyn AgentSigner + Send + Sync>,
    reporter: Arc<Reporter>,
    timeout: Duration,
}

impl std::fmt::Debug for TryCPClientInstrumented {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TryCPClient").finish()
    }
}

mod control_impl {
    use super::*;
    use trycp_api::{DownloadLogsResponse, TryCpServerResponse};
    use trycp_client::Signal;

    impl TryCPClientInstrumented {
        pub async fn connect<R>(
            request: R,
            signer: Arc<dyn AgentSigner + Send + Sync>,
            reporter: Arc<Reporter>,
        ) -> io::Result<Self>
        where
            R: IntoClientRequest + Unpin,
        {
            let (trycp_client, signal_recv) = TrycpClient::connect(request).await?;
            Ok(Self {
                trycp_client: Arc::new(trycp_client),
                signal_recv: Arc::new(tokio::sync::Mutex::new(signal_recv)),
                signer,
                reporter,
                timeout: Duration::from_secs(30),
            })
        }

        pub async fn recv_signal(&self) -> Option<Signal> {
            let mut recv = self.signal_recv.lock().await;
            recv.recv().await
        }

        /// Given a DNA file, stores the DNA and returns the path at which it is stored.
        ///
        /// Stored in the data path under `id`, with content `content`.
        /// Returns the path at which the DNA is stored.
        #[wind_tunnel_instrument(prefix = "trycp_")]
        pub async fn save_dna(
            &self,
            id: String,
            content: Vec<u8>,
            timeout: Option<Duration>,
        ) -> io::Result<String> {
            let response = self
                .trycp_client
                .request(
                    Request::SaveDna { id, content },
                    timeout.unwrap_or(self.timeout),
                )
                .await?;

            read_response(response)
        }

        /// Given a DNA URL, ensures that the DNA is downloaded and returns the path at which it is stored.
        #[wind_tunnel_instrument(prefix = "trycp_")]
        pub async fn download_dna(
            &self,
            url: String,
            timeout: Option<Duration>,
        ) -> io::Result<String> {
            let response = self
                .trycp_client
                .request(
                    Request::DownloadDna { url },
                    timeout.unwrap_or(self.timeout),
                )
                .await?;

            read_response(response)
        }

        /// Set up a player.
        ///
        /// Provide a player `id` and serialized YAML content for the conductor in `partial_config`.
        #[wind_tunnel_instrument(prefix = "trycp_")]
        pub async fn configure_player(
            &self,
            id: String,
            partial_config: String,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let response = self
                .trycp_client
                .request(
                    Request::ConfigurePlayer { id, partial_config },
                    timeout.unwrap_or(self.timeout),
                )
                .await?;

            check_empty_response(response)
        }

        /// Start a conductor.
        ///
        /// Provide a conductor `id` and optionally a `log_level` such as "info" or "warn".
        #[wind_tunnel_instrument(prefix = "trycp_")]
        pub async fn startup(
            &self,
            id: String,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let log_level = std::env::var("TRYCP_RUST_LOG").unwrap_or("warn".to_string());

            let response = self
                .trycp_client
                .request(
                    Request::Startup { id, log_level: Some(log_level) },
                    timeout.unwrap_or(self.timeout),
                )
                .await?;

            check_empty_response(response)
        }

        /// Start a conductor.
        ///
        /// Provide a conductor `id` and optionally a `signal` such as "SIGTERM", "SIGKILL" or "SIGINT".
        /// No other signals are supported.
        #[wind_tunnel_instrument(prefix = "trycp_")]
        pub async fn shutdown(
            &self,
            id: String,
            signal: Option<String>,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let response = self
                .trycp_client
                .request(
                    Request::Shutdown { id, signal },
                    timeout.unwrap_or(self.timeout),
                )
                .await?;

            check_empty_response(response)
        }

        /// Shuts down all running conductors.
        #[wind_tunnel_instrument(prefix = "trycp_")]
        pub async fn reset(&self, timeout: Option<Duration>) -> io::Result<()> {
            let response = self
                .trycp_client
                .request(Request::Reset, timeout.unwrap_or(self.timeout))
                .await?;

            check_empty_response(response)
        }

        /// Hook up an app interface.
        ///
        /// Provide a `token` that has been issued using the admin interface and the `port` to
        /// connect to.
        #[wind_tunnel_instrument(prefix = "trycp_")]
        pub async fn connect_app_interface(
            &self,
            token: Vec<u8>,
            port: u16,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let response = self
                .trycp_client
                .request(
                    Request::ConnectAppInterface { token, port },
                    timeout.unwrap_or(self.timeout),
                )
                .await?;

            check_empty_response(response)
        }

        /// Disconnect an app interface.
        ///
        /// Provide the `port` to identify the app interface to disconnect.
        #[wind_tunnel_instrument(prefix = "trycp_")]
        pub async fn disconnect_app_interface(
            &self,
            port: u16,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let response = self
                .trycp_client
                .request(
                    Request::DisconnectAppInterface { port },
                    timeout.unwrap_or(self.timeout),
                )
                .await?;

            check_empty_response(response)
        }

        #[wind_tunnel_instrument(prefix = "trycp_")]
        pub async fn download_logs(&self, id: String, timeout: Option<Duration>) -> io::Result<DownloadLogsResponse> {
            let response = self
                .trycp_client
                .request(Request::DownloadLogs { id }, timeout.unwrap_or(self.timeout))
                .await?;

            match read_response::<TryCpServerResponse>(response) {
                Ok(TryCpServerResponse::DownloadLogs(response)) => Ok(response),
                _ => Err(io::Error::other("Unexpected response")),
            }
        }
    }
}

mod admin_impl {
    use holo_hash::{AgentPubKey, DnaHash};
    use holochain_client::{
        AuthorizeSigningCredentialsPayload, EnableAppResponse, SigningCredentials,
    };
    use holochain_conductor_api::{
        AdminRequest, AdminResponse, AppAuthenticationToken, AppAuthenticationTokenIssued, AppInfo,
        AppInterfaceInfo, AppStatusFilter, FullStateDump, IssueAppAuthenticationTokenPayload,
        StorageInfo,
    };
    use holochain_serialized_bytes::encode;
    use holochain_types::app::{DeleteCloneCellPayload, InstallAppPayload, InstalledAppId};
    use holochain_types::prelude::{GrantedFunctions, Record};
    use holochain_types::websocket::AllowedOrigins;
    use holochain_zome_types::capability::GrantZomeCallCapabilityPayload;
    use holochain_zome_types::cell::CellId;
    use holochain_zome_types::dna_def::DnaDef;
    use kitsune_p2p_types::agent_info::AgentInfoSigned;

    use super::*;

    impl TryCPClientInstrumented {
        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn get_dna_definition(
            &self,
            id: String,
            dna_hash: DnaHash,
            timeout: Option<Duration>,
        ) -> io::Result<DnaDef> {
            let response = self
                .call_admin(
                    id,
                    AdminRequest::GetDnaDefinition(Box::new(dna_hash)),
                    timeout,
                )
                .await?;

            match response {
                AdminResponse::DnaDefinitionReturned(dna_def) => Ok(dna_def),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn install_app(
            &self,
            id: String,
            payload: InstallAppPayload,
            timeout: Option<Duration>,
        ) -> io::Result<AppInfo> {
            let response = self
                .call_admin(
                    id.clone(),
                    AdminRequest::InstallApp(Box::new(payload)),
                    timeout,
                )
                .await?;

            match response {
                AdminResponse::AppInstalled(app) => Ok(app),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn uninstall_app(
            &self,
            id: String,
            installed_app_id: String,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let response = self
                .call_admin(id, AdminRequest::UninstallApp { installed_app_id }, timeout)
                .await?;

            match response {
                AdminResponse::AppUninstalled => Ok(()),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn list_dnas(
            &self,
            id: String,
            timeout: Option<Duration>,
        ) -> io::Result<Vec<DnaHash>> {
            let response = self.call_admin(id, AdminRequest::ListDnas, timeout).await?;

            match response {
                AdminResponse::DnasListed(dnas) => Ok(dnas),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn generate_agent_pub_key(
            &self,
            id: String,
            timeout: Option<Duration>,
        ) -> io::Result<AgentPubKey> {
            let response = self
                .call_admin(id, AdminRequest::GenerateAgentPubKey, timeout)
                .await?;

            match response {
                AdminResponse::AgentPubKeyGenerated(agent_pub_key) => Ok(agent_pub_key),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn list_cell_ids(
            &self,
            id: String,
            timeout: Option<Duration>,
        ) -> io::Result<Vec<CellId>> {
            let response = self
                .call_admin(id, AdminRequest::ListCellIds, timeout)
                .await?;

            match response {
                AdminResponse::CellIdsListed(ids) => Ok(ids),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn list_apps(
            &self,
            id: String,
            status_filter: Option<AppStatusFilter>,
            timeout: Option<Duration>,
        ) -> io::Result<Vec<AppInfo>> {
            let response = self
                .call_admin(id, AdminRequest::ListApps { status_filter }, timeout)
                .await?;

            match response {
                AdminResponse::AppsListed(apps) => Ok(apps),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn enable_app(
            &self,
            id: String,
            installed_app_id: String,
            timeout: Option<Duration>,
        ) -> io::Result<EnableAppResponse> {
            let response = self
                .call_admin(id, AdminRequest::EnableApp { installed_app_id }, timeout)
                .await?;

            match response {
                AdminResponse::AppEnabled { app, errors } => Ok(EnableAppResponse { app, errors }),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn disable_app(
            &self,
            id: String,
            installed_app_id: String,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let response = self
                .call_admin(id, AdminRequest::DisableApp { installed_app_id }, timeout)
                .await?;

            match response {
                AdminResponse::AppDisabled => Ok(()),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn attach_app_interface(
            &self,
            id: String,
            port: Option<u16>,
            allowed_origins: AllowedOrigins,
            installed_app_id: Option<InstalledAppId>,
            timeout: Option<Duration>,
        ) -> io::Result<u16> {
            let response = self
                .call_admin(
                    id,
                    AdminRequest::AttachAppInterface {
                        port,
                        allowed_origins,
                        installed_app_id,
                    },
                    timeout,
                )
                .await?;

            match response {
                AdminResponse::AppInterfaceAttached { port } => Ok(port),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn list_app_interfaces(
            &self,
            id: String,
            timeout: Option<Duration>,
        ) -> io::Result<Vec<AppInterfaceInfo>> {
            let response = self
                .call_admin(id, AdminRequest::ListAppInterfaces, timeout)
                .await?;

            match response {
                AdminResponse::AppInterfacesListed(info) => Ok(info),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn dump_state(
            &self,
            id: String,
            cell_id: CellId,
            timeout: Option<Duration>,
        ) -> io::Result<String> {
            let response = self
                .call_admin(
                    id,
                    AdminRequest::DumpState {
                        cell_id: Box::new(cell_id),
                    },
                    timeout,
                )
                .await?;

            match response {
                AdminResponse::StateDumped(s) => Ok(s),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn dump_conductor_state(
            &self,
            id: String,
            timeout: Option<Duration>,
        ) -> io::Result<String> {
            let response = self
                .call_admin(id, AdminRequest::DumpConductorState, timeout)
                .await?;

            match response {
                AdminResponse::ConductorStateDumped(s) => Ok(s),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn dump_full_state(
            &self,
            id: String,
            cell_id: CellId,
            dht_ops_cursor: Option<u64>,
            timeout: Option<Duration>,
        ) -> io::Result<FullStateDump> {
            let response = self
                .call_admin(
                    id,
                    AdminRequest::DumpFullState {
                        cell_id: Box::new(cell_id),
                        dht_ops_cursor,
                    },
                    timeout,
                )
                .await?;

            match response {
                AdminResponse::FullStateDumped(fds) => Ok(fds),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn dump_network_metrics(
            &self,
            id: String,
            dna_hash: Option<DnaHash>,
            timeout: Option<Duration>,
        ) -> io::Result<String> {
            let response = self
                .call_admin(id, AdminRequest::DumpNetworkMetrics { dna_hash }, timeout)
                .await?;

            match response {
                AdminResponse::NetworkMetricsDumped(metrics) => Ok(metrics),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn dump_network_stats(
            &self,
            id: String,
            timeout: Option<Duration>,
        ) -> io::Result<String> {
            let response = self
                .call_admin(id, AdminRequest::DumpNetworkStats, timeout)
                .await?;

            match response {
                AdminResponse::NetworkStatsDumped(stats) => Ok(stats),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn add_agent_info(
            &self,
            id: String,
            agent_infos: Vec<AgentInfoSigned>,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let response = self
                .call_admin(id, AdminRequest::AddAgentInfo { agent_infos }, timeout)
                .await?;

            match response {
                AdminResponse::AgentInfoAdded => Ok(()),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn agent_info(
            &self,
            id: String,
            cell_id: Option<CellId>,
            timeout: Option<Duration>,
        ) -> io::Result<Vec<AgentInfoSigned>> {
            let response = self
                .call_admin(id, AdminRequest::AgentInfo { cell_id }, timeout)
                .await?;

            match response {
                AdminResponse::AgentInfo(agents) => Ok(agents),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn graft_records(
            &self,
            id: String,
            cell_id: CellId,
            validate: bool,
            records: Vec<Record>,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let response = self
                .call_admin(
                    id,
                    AdminRequest::GraftRecords {
                        cell_id,
                        validate,
                        records,
                    },
                    timeout,
                )
                .await?;

            match response {
                AdminResponse::RecordsGrafted => Ok(()),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn grant_zome_call_capability(
            &self,
            id: String,
            capability: GrantZomeCallCapabilityPayload,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let response = self
                .call_admin(
                    id,
                    AdminRequest::GrantZomeCallCapability(Box::new(capability)),
                    timeout,
                )
                .await?;

            match response {
                AdminResponse::ZomeCallCapabilityGranted => Ok(()),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn delete_clone_cell(
            &self,
            id: String,
            payload: DeleteCloneCellPayload,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let response = self
                .call_admin(
                    id,
                    AdminRequest::DeleteCloneCell(Box::new(payload)),
                    timeout,
                )
                .await?;

            match response {
                AdminResponse::CloneCellDeleted => Ok(()),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn storage_info(
            &self,
            id: String,
            timeout: Option<Duration>,
        ) -> io::Result<StorageInfo> {
            let response = self
                .call_admin(id, AdminRequest::StorageInfo, timeout)
                .await?;

            match response {
                AdminResponse::StorageInfo(info) => Ok(info),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn issue_app_auth_token(
            &self,
            id: String,
            payload: IssueAppAuthenticationTokenPayload,
            timeout: Option<Duration>,
        ) -> io::Result<AppAuthenticationTokenIssued> {
            let response = self
                .call_admin(
                    id,
                    AdminRequest::IssueAppAuthenticationToken(payload),
                    timeout,
                )
                .await?;

            match response {
                AdminResponse::AppAuthenticationTokenIssued(token) => Ok(token),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn revoke_app_auth_token(
            &self,
            id: String,
            token: AppAuthenticationToken,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let response = self
                .call_admin(
                    id,
                    AdminRequest::RevokeAppAuthenticationToken(token),
                    timeout,
                )
                .await?;

            match response {
                AdminResponse::AppAuthenticationTokenRevoked => Ok(()),
                _ => Err(io::Error::other("Unexpected admin response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_admin_")]
        pub async fn authorize_signing_credentials(
            &self,
            id: String,
            request: AuthorizeSigningCredentialsPayload,
            timeout: Option<Duration>,
        ) -> io::Result<SigningCredentials> {
            use holochain_zome_types::capability::{ZomeCallCapGrant, CAP_SECRET_BYTES};
            use rand::{rngs::OsRng, RngCore};
            use std::collections::BTreeSet;

            let mut csprng = OsRng;
            let keypair = ed25519_dalek::SigningKey::generate(&mut csprng);
            let public_key = keypair.verifying_key();
            let signing_agent_key = AgentPubKey::from_raw_32(public_key.as_bytes().to_vec());

            let mut cap_secret = [0; CAP_SECRET_BYTES];
            csprng.fill_bytes(&mut cap_secret);

            self.call_admin(
                id,
                AdminRequest::GrantZomeCallCapability(Box::new(GrantZomeCallCapabilityPayload {
                    cell_id: request.cell_id,
                    cap_grant: ZomeCallCapGrant {
                        tag: "zome-call-signing-key".to_string(),
                        access: holochain_zome_types::capability::CapAccess::Assigned {
                            secret: cap_secret.into(),
                            assignees: BTreeSet::from([signing_agent_key.clone()]),
                        },
                        functions: request.functions.unwrap_or(GrantedFunctions::All),
                    },
                })),
                timeout,
            )
            .await?;

            Ok(SigningCredentials {
                signing_agent_key,
                keypair,
                cap_secret: cap_secret.into(),
            })
        }

        async fn call_admin(
            &self,
            id: String,
            request: AdminRequest,
            timeout: Option<Duration>,
        ) -> io::Result<AdminResponse> {
            let message = encode(&request).map_err(io::Error::other)?;

            let response = self
                .trycp_client
                .request(
                    Request::CallAdminInterface { id, message },
                    timeout.unwrap_or(self.timeout),
                )
                .await?;

            match read_response(response) {
                Ok(AdminResponse::Error(e)) => {
                    // Explicitly map the error response to an error to prevent crashes when
                    // checking the expected response types.
                    Err(io::Error::other(format!("{e:?}")))
                }
                other => other,
            }
        }
    }
}

mod app_impl {
    use super::*;
    use holochain_conductor_api::{AppInfo, AppRequest, AppResponse, NetworkInfo};
    use holochain_nonce::fresh_nonce;
    use holochain_serialized_bytes::encode;
    use holochain_types::app::{
        CreateCloneCellPayload, DisableCloneCellPayload, EnableCloneCellPayload,
        NetworkInfoRequestPayload,
    };
    use holochain_types::prelude::{ExternIO, FunctionName, Timestamp, ZomeName};
    use holochain_zome_types::clone::ClonedCell;
    use holochain_zome_types::prelude::{CellId, ZomeCallUnsigned};

    impl TryCPClientInstrumented {
        #[wind_tunnel_instrument(prefix = "trycp_app_")]
        pub async fn app_info(
            &self,
            port: u16,
            timeout: Option<Duration>,
        ) -> io::Result<Option<AppInfo>> {
            let response = self.call_app(port, AppRequest::AppInfo, timeout).await?;

            match response {
                AppResponse::AppInfo(info) => Ok(info),
                _ => Err(io::Error::other("Unexpected app response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_app_", pre_hook = pre_call_zome)]
        pub async fn call_zome<I>(
            &self,
            port: u16,
            cell_id: CellId,
            zome_name: impl Into<ZomeName> + Clone,
            fn_name: impl Into<FunctionName> + Clone,
            payload: I,
            timeout: Option<Duration>,
        ) -> io::Result<ExternIO>
        where
            I: serde::Serialize + std::fmt::Debug,
        {
            let (nonce, expires_at) = fresh_nonce(Timestamp::now()).map_err(io::Error::other)?;

            let zome_call_unsigned = ZomeCallUnsigned {
                provenance: self
                    .signer
                    .get_provenance(&cell_id)
                    .ok_or(io::Error::other("Provenance not found".to_string()))?,
                cap_secret: self.signer.get_cap_secret(&cell_id),
                cell_id: cell_id.clone(),
                zome_name: zome_name.into(),
                fn_name: fn_name.into(),
                payload: ExternIO::encode(payload).map_err(io::Error::other)?,
                expires_at,
                nonce,
            };

            let signed_zome_call = sign_zome_call(zome_call_unsigned, self.signer.clone()).await?;

            let app_request = AppRequest::CallZome(Box::new(signed_zome_call));
            let response = self.call_app(port, app_request, timeout).await?;

            match response {
                AppResponse::ZomeCalled(result) => Ok(*result),
                _ => unreachable!("Unexpected response {:?}", response),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_app_")]
        pub async fn create_clone_cell(
            &self,
            port: u16,
            payload: CreateCloneCellPayload,
            timeout: Option<Duration>,
        ) -> io::Result<ClonedCell> {
            let response = self
                .call_app(
                    port,
                    AppRequest::CreateCloneCell(Box::new(payload)),
                    timeout,
                )
                .await?;

            match response {
                AppResponse::CloneCellCreated(cell) => Ok(cell),
                _ => Err(io::Error::other("Unexpected app response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_app_")]
        pub async fn enable_clone_cell(
            &self,
            port: u16,
            payload: EnableCloneCellPayload,
            timeout: Option<Duration>,
        ) -> io::Result<ClonedCell> {
            let response = self
                .call_app(
                    port,
                    AppRequest::EnableCloneCell(Box::new(payload)),
                    timeout,
                )
                .await?;

            match response {
                AppResponse::CloneCellEnabled(cell) => Ok(cell),
                _ => Err(io::Error::other("Unexpected app response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_app_")]
        pub async fn disable_clone_cell(
            &self,
            port: u16,
            payload: DisableCloneCellPayload,
            timeout: Option<Duration>,
        ) -> io::Result<()> {
            let response = self
                .call_app(
                    port,
                    AppRequest::DisableCloneCell(Box::new(payload)),
                    timeout,
                )
                .await?;

            match response {
                AppResponse::CloneCellDisabled => Ok(()),
                _ => Err(io::Error::other("Unexpected app response")),
            }
        }

        #[wind_tunnel_instrument(prefix = "trycp_app_")]
        pub async fn network_info(
            &self,
            port: u16,
            payload: NetworkInfoRequestPayload,
            timeout: Option<Duration>,
        ) -> io::Result<Vec<NetworkInfo>> {
            let response = self
                .call_app(port, AppRequest::NetworkInfo(Box::new(payload)), timeout)
                .await?;

            match response {
                AppResponse::NetworkInfo(info) => Ok(info),
                _ => Err(io::Error::other("Unexpected app response")),
            }
        }

        async fn call_app(
            &self,
            port: u16,
            request: AppRequest,
            timeout: Option<Duration>,
        ) -> io::Result<AppResponse> {
            let message = encode(&request).map_err(io::Error::other)?;

            let response = self
                .trycp_client
                .request(
                    Request::CallAppInterface { port, message },
                    timeout.unwrap_or(self.timeout),
                )
                .await?;

            match read_response(response) {
                Ok(AppResponse::Error(r)) => {
                    // Explicitly map the error response to an error to prevent crashes when
                    // checking the expected response types.
                    Err(io::Error::other(format!("{r:?}")))
                }
                other => other,
            }
        }
    }
}

fn read_response<R: DeserializeOwned>(response: MessageResponse) -> io::Result<R> {
    match response {
        MessageResponse::Null => {
            panic!("Unexpected null response");
        }
        MessageResponse::Bytes(v) => rmp_serde::from_slice(&v).map_err(io::Error::other),
    }
}

fn check_empty_response(response: MessageResponse) -> io::Result<()> {
    match response {
        MessageResponse::Null => Ok(()),
        MessageResponse::Bytes(v) => {
            panic!("Unexpected bytes response: {:?}", v);
        }
    }
}

fn pre_call_zome<I>(
    operation_record: &mut OperationRecord,
    _port: &u16,
    _cell_id: &CellId,
    zome_name: &(impl Into<ZomeName> + Clone),
    fn_name: &(impl Into<FunctionName> + Clone),
    _payload: &I,
    _timeout: &Option<Duration>,
) where
    I: serde::Serialize + std::fmt::Debug,
{
    let zome_name: ZomeName = zome_name.clone().into();
    let fn_name: FunctionName = fn_name.clone().into();
    operation_record.add_attr("zome_name", zome_name.0.to_string());
    operation_record.add_attr("fn_name", fn_name.0.to_string());
}

pub(crate) async fn sign_zome_call(
    zome_call_unsigned: ZomeCallUnsigned,
    signer: Arc<dyn AgentSigner + Send + Sync>,
) -> io::Result<ZomeCall> {
    let pub_key = zome_call_unsigned.provenance.clone();

    let data_to_sign = zome_call_unsigned.data_to_sign().map_err(|e| {
        io::Error::other(format!(
            "Failed to get data to sign from unsigned zome call: {e:?}"
        ))
    })?;

    let signature = signer
        .sign(&zome_call_unsigned.cell_id, pub_key, data_to_sign)
        .await
        .map_err(io::Error::other)?;

    Ok(ZomeCall {
        cell_id: zome_call_unsigned.cell_id,
        zome_name: zome_call_unsigned.zome_name,
        fn_name: zome_call_unsigned.fn_name,
        payload: zome_call_unsigned.payload,
        cap_secret: zome_call_unsigned.cap_secret,
        provenance: zome_call_unsigned.provenance,
        nonce: zome_call_unsigned.nonce,
        expires_at: zome_call_unsigned.expires_at,
        signature,
    })
}
