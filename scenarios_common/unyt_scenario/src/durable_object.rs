//! HTTP client for the Cloudflare Durable Object temporary storage.
//!
//! The [`DurableObject`] client lets the progenitor agent publish its
//! public key so that other agents (potentially running on different
//! machines) can retrieve it before installing the hApp.

use crate::UnytScenarioValues;
use anyhow::Context;
use holochain_types::prelude::AgentPubKey;
use holochain_wind_tunnel_runner::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

/// HTTP client for sharing the progenitor key via a Durable Object.
///
/// The progenitor agent posts its public key after generation; every
/// other agent polls until the key becomes available, then uses it to
/// configure DNA properties before installing the hApp.
#[derive(Debug, Clone)]
pub struct DurableObject {
    /// Root URL of the Durable Object worker endpoint.
    pub base_url: String,
    /// Shared secret sent with POST requests for authentication.
    pub secret: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct PostData {
    run_id: String,
    value: String,
    secret: String,
}

#[derive(Deserialize)]
struct PostResponse {
    success: bool,
}

#[derive(Deserialize)]
struct GetResponse {
    value: String,
}

impl DurableObject {
    /// Creates a new client with default endpoint and timeouts.
    pub fn new() -> Self {
        let base_url = std::env::var("UNYT_DURABLE_OBJECTS_URL")
            .ok()
            .map(|var| var.trim().to_string())
            .filter(|var| !var.is_empty())
            .expect("UNYT_DURABLE_OBJECTS_URL needs to be set for this scenario to run");
        let secret = std::env::var("UNYT_DURABLE_OBJECTS_SECRET")
            .ok()
            .map(|var| var.trim().to_string())
            .filter(|var| !var.is_empty())
            .expect("UNYT_DURABLE_OBJECTS_SECRET needs to be set for this scenario to run");

        Self {
            base_url,
            secret,
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .read_timeout(Duration::from_secs(30))
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to build the reqwest Client"),
        }
    }

    /// Posts a value to the Durable Object under `key`.
    ///
    /// `key` is used verbatim as the Durable Object's storage key, so
    /// callers namespace per run and role (e.g. `"{run_id}:bridge"`).
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response
    /// cannot be parsed.
    async fn post(&self, key: &str, value: &str) -> anyhow::Result<bool> {
        let post_data = PostData {
            run_id: key.to_string(),
            value: value.to_string(),
            secret: self.secret.clone(),
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .json(&post_data)
            .send()
            .await
            .context("Failed to send POST request to DurableObject")?;

        if !response.status().is_success() {
            anyhow::bail!("POST request failed with status: {}", response.status());
        }

        let post_response: PostResponse = response
            .json()
            .await
            .context("Failed to parse POST response")?;

        log::info!("POST response: success={}", post_response.success);
        Ok(post_response.success)
    }

    /// Fetches the value stored under `key`, blocking the current agent.
    ///
    /// Polls the endpoint every 2 seconds until the value is available
    /// or a 120-second timeout expires.
    ///
    /// # Errors
    ///
    /// Returns an error if the value is not available within the
    /// timeout, or the HTTP request/parsing fails.
    fn fetch_blocking<SV: UnytScenarioValues>(
        &self,
        ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
        key: &str,
    ) -> anyhow::Result<String> {
        let url = format!("{}?run_id={}", self.base_url, key);
        ctx.runner_context()
            .executor()
            .execute_in_place(async {
                timeout(Duration::from_secs(120), async {
                    loop {
                        log::debug!("Attempting to get value from DurableObject: key={}", key);

                        let response = self
                            .client
                            .get(&url)
                            .send()
                            .await
                            .context("Failed to send GET request to DurableObject")?;

                        if response.status().is_success() {
                            let get_response: GetResponse = response
                                .json()
                                .await
                                .context("Failed to parse GET response")?;

                            log::debug!("Successfully retrieved value: {}", get_response.value);
                            return Ok(get_response.value);
                        } else if response.status() == 404 {
                            log::info!("Value not yet available, retrying in 2 seconds...");
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            continue;
                        } else {
                            anyhow::bail!("GET request failed with status: {}", response.status());
                        }
                    }
                })
                .await?
            })
            .context("Failed to fetch value from DurableObject")
    }

    /// Posts the progenitor key so other agents can retrieve it via
    /// [`Self::get_progenitor_key`]. Called once by the progenitor.
    pub async fn post_progenitor_key(
        &self,
        run_id: &str,
        progenitor_key: &str,
    ) -> anyhow::Result<bool> {
        log::info!(
            "Posting progenitor key to DurableObject: run_id={run_id}, key={progenitor_key}"
        );
        self.post(run_id, progenitor_key).await
    }

    /// Posts the bridge agent's key so the progenitor can authorize it
    /// as oracle and bridging_agent when building the lane's agreements.
    /// Called once by the bridge agent.
    pub async fn post_bridge_agent_key(
        &self,
        run_id: &str,
        bridge_agent_key: &str,
    ) -> anyhow::Result<bool> {
        log::info!(
            "Posting bridge agent key to DurableObject: run_id={run_id}, key={bridge_agent_key}"
        );
        self.post(&format!("{run_id}:bridge_agent"), bridge_agent_key)
            .await
    }

    /// Fetches the progenitor key from the Durable Object, caching it in
    /// [`UnytScenarioValues`] so subsequent calls return immediately.
    pub fn get_progenitor_key<SV: UnytScenarioValues>(
        &self,
        ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    ) -> anyhow::Result<AgentPubKey> {
        if let Some(progenitor_agent_pubkey) = ctx.get().scenario_values.progenitor_agent_pubkey() {
            return Ok(progenitor_agent_pubkey.clone().into());
        }
        let run_id = ctx.runner_context().get_run_id().to_string();
        let progenitor_key_str = self.fetch_blocking(ctx, &run_id)?;
        let progenitor_pubkey: AgentPubKey = AgentPubKey::try_from(progenitor_key_str)
            .context("Failed to parse progenitor key from DurableObject")?;

        ctx.get_mut()
            .scenario_values
            .set_progenitor_agent_pubkey(progenitor_pubkey.clone().into());

        log::debug!("Fetched progenitor agent pubkey: {:?}", progenitor_pubkey);
        Ok(progenitor_pubkey)
    }

    /// Fetches the bridge agent's key from the Durable Object. Called by
    /// the progenitor while setting up the lane's agreements.
    pub fn get_bridge_agent_key<SV: UnytScenarioValues>(
        &self,
        ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    ) -> anyhow::Result<AgentPubKey> {
        let run_id = ctx.runner_context().get_run_id().to_string();
        let bridge_agent_key_str = self.fetch_blocking(ctx, &format!("{run_id}:bridge_agent"))?;
        AgentPubKey::try_from(bridge_agent_key_str)
            .context("Failed to parse bridge agent key from DurableObject")
    }

    /// Posts the swap agent's key so swappers can address commitments to it.
    /// Called once by the swap agent.
    pub async fn post_swap_agent_key(
        &self,
        run_id: &str,
        swap_agent_key: &str,
    ) -> anyhow::Result<bool> {
        log::info!(
            "Posting swap agent key to DurableObject: run_id={run_id}, key={swap_agent_key}"
        );
        self.post(&format!("{run_id}:swap_agent"), swap_agent_key)
            .await
    }

    /// Fetches the swap agent's key from the Durable Object. Called by
    /// swappers to set the commitment counterparty.
    pub fn get_swap_agent_key<SV: UnytScenarioValues>(
        &self,
        ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    ) -> anyhow::Result<AgentPubKey> {
        let run_id = ctx.runner_context().get_run_id().to_string();
        let swap_agent_key_str = self.fetch_blocking(ctx, &format!("{run_id}:swap_agent"))?;
        AgentPubKey::try_from(swap_agent_key_str)
            .context("Failed to parse swap agent key from DurableObject")
    }
}

impl Default for DurableObject {
    fn default() -> Self {
        Self::new()
    }
}
