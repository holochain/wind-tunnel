use anyhow::Context;
use holochain_types::prelude::AgentPubKey;
use holochain_wind_tunnel_runner::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::ScenarioValues;

// DurableObject struct and related functionality
#[derive(Debug, Clone)]
pub struct DurableObject {
    pub base_url: String,
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
    pub fn new() -> Self {
        Self {
            base_url: "https://durable-object-tmp-storage.joel-ulahanna.workers.dev".to_string(),
            secret: "zo-el".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn post_progenitor_key(
        &self,
        run_id: &str,
        progenitor_key: &str,
    ) -> anyhow::Result<bool> {
        let post_data = PostData {
            run_id: run_id.to_string(),
            value: progenitor_key.to_string(),
            secret: self.secret.clone(),
        };

        log::info!(
            "Posting progenitor key to DurableObject: run_id={}, key={}",
            run_id,
            progenitor_key
        );

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

    pub fn get_progenitor_key(
        &self,
        ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    ) -> anyhow::Result<AgentPubKey> {
        if let Some(progenitor_agent_pubkey) = &ctx.get().scenario_values.progenitor_agent_pubkey {
            return Ok(progenitor_agent_pubkey.clone().into());
        }
        // Use the same run_id as used in setup_progenitor
        let run_id = ctx.runner_context().get_run_id().to_string();
        let url = format!("{}?run_id={}", self.base_url, run_id);
        // Get the progenitor key using the executor
        let progenitor_key_str = ctx
            .runner_context()
            .executor()
            .execute_in_place(async move {
                loop {
                    log::debug!(
                        "Attempting to get progenitor key from DurableObject: run_id={}",
                        run_id
                    );

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

                        log::debug!(
                            "Successfully retrieved progenitor key: {}",
                            get_response.value
                        );
                        return Ok(get_response.value);
                    } else if response.status() == 404 {
                        log::info!("Progenitor key not yet available, retrying in 2 seconds...");
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        continue;
                    } else {
                        anyhow::bail!("GET request failed with status: {}", response.status());
                    }
                }
            })
            .context("Failed to fetch progenitor key from DurableObject")?;

        // Parse the string back to AgentPubKey using try_from
        let progenitor_pubkey: AgentPubKey = AgentPubKey::try_from(progenitor_key_str)
            .context("Failed to parse progenitor key from DurableObject")?;

        ctx.get_mut().scenario_values.progenitor_agent_pubkey =
            Some(progenitor_pubkey.clone().into());

        log::debug!("Fetched progenitor agent pubkey: {:?}", progenitor_pubkey);
        Ok(progenitor_pubkey)
    }
}
