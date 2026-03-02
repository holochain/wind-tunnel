use crate::ArcType;
use crate::UnytScenarioValues;
use crate::unyt_agent::UnytAgentExt;
use anyhow::anyhow;
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::{AcceptInput, CommitmentInput, Pagination, UnitMap, WatchStatus};
use std::time::SystemTime;
use std::{collections::BTreeMap, thread, time::Duration};
use zfuel::{fraction::Fraction, fuel::ZFuel};

/// Spend agent behaviour shared across Unyt scenarios.
///
/// When `arc_type` is `Some`, the `global_definition_propagation_time`
/// metric is tagged with an `arc` key (e.g. `"zero"` for 0-arc agents).
pub fn agent_behaviour<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    arc_type: Option<ArcType>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();
    let session_started_at = ctx
        .get()
        .scenario_values
        .session_start_time()
        .ok_or(anyhow!("`session_started_at` not set"))?;
    let network_initialized = ctx.get().scenario_values.network_initialized();

    // Test 1
    if !network_initialized {
        if ctx.is_network_initialized() {
            log::info!(
                "Network initialized for agent {}",
                ctx.get().cell_id().agent_pubkey()
            );
            let mut metric = ReportMetric::new("global_definition_propagation_time")
                .with_field("at", session_started_at.elapsed().as_secs())
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string());
            if let Some(tag) = arc_type {
                metric = metric.with_tag("arc", tag.as_tag());
            }
            reporter.add_custom(metric);
            ctx.get_mut().scenario_values.set_network_initialized(true);
        } else {
            // if the network is not initialized do not proceed with further testing without waiting for it to be initialized
            log::info!(
                "Network not initialized for agent {}, waiting for it to be initialized",
                ctx.get().cell_id().agent_pubkey()
            );
            thread::sleep(Duration::from_secs(2));
            return Ok(());
        }
    }

    // test 2
    // check incoming transactions and accept them so that you can have more to spend
    let actionable_transactions = match ctx.unyt_get_actionable_transactions() {
        Ok(txs) => txs,
        Err(err) => {
            log::warn!("Failed to get actionable transactions (transient DHT issue): {err}");
            thread::sleep(Duration::from_secs(1));
            return Ok(());
        }
    };
    // Measure sync lag for newly discovered commitment transactions (zero-arc only)
    if let Some(tag) = arc_type {
        let now_us = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_micros();
        let agent_key = ctx.get().cell_id().agent_pubkey().to_string();
        for tx in &actionable_transactions.commitment_actionable {
            if ctx
                .get()
                .scenario_values
                .seen_transactions()
                .contains(&tx.id)
            {
                continue;
            }
            let published_at_us = tx.timestamp.as_micros() as u128;
            let lag_s = now_us.saturating_sub(published_at_us) as f64 / 1e6;
            reporter.add_custom(
                ReportMetric::new("sync_lag")
                    .with_tag("agent", agent_key.clone())
                    .with_tag("arc", tag.as_tag())
                    .with_tag("tx_type", "commitment")
                    .with_field("value", lag_s),
            );
            ctx.get_mut()
                .scenario_values
                .seen_transactions_mut()
                .insert(tx.id.clone());
        }
    }

    // accept incoming invoices too?
    if !actionable_transactions.commitment_actionable.is_empty() {
        log::info!(
            "Agent {} | accepting {} transactions",
            ctx.get().cell_id().agent_pubkey(),
            actionable_transactions.commitment_actionable.len()
        );
    }
    for transaction in actionable_transactions.commitment_actionable {
        if let Err(err) = ctx.unyt_create_accept(AcceptInput {
            commitment: transaction.id.clone(),
            note: None,
        }) {
            log::warn!("Failed to accept transaction '{transaction:?}': {err}");
        };
    }

    // test 3
    // get ledger and calculate how much you can spend in this round
    let ledger = match ctx.unyt_get_ledger() {
        Ok(l) => l,
        Err(err) => {
            log::warn!("Failed to get ledger (transient DHT issue): {err}");
            thread::sleep(Duration::from_secs(1));
            return Ok(());
        }
    };
    log::info!(
        "Agent {} | ledger: {:?}",
        ctx.get().cell_id().agent_pubkey(),
        ledger
    );
    let balance = ledger.balance.get_base_unyt();
    let fees = ledger.fees_owed;
    let credit_limit = match ctx.unyt_get_my_current_applied_credit_limit() {
        Ok(cl) => cl,
        Err(err) => {
            log::warn!("Failed to get credit limit (transient DHT issue): {err}");
            thread::sleep(Duration::from_secs(1));
            return Ok(());
        }
    };
    let spendable_amount = (balance - fees + credit_limit.get_base_unyt())?;

    // test 4
    // collect agents and start transacting
    if spendable_amount > ZFuel::zero() {
        log::info!(
            "Agent {} | spendable amount: {}",
            ctx.get().cell_id().agent_pubkey(),
            spendable_amount
        );
        ctx.collect_agents()?;

        // spend with those agents
        let participating_agents = ctx.get().scenario_values.participating_agents().to_vec();
        if participating_agents.is_empty() {
            log::warn!("No participating agents to spend with");
            return Ok(());
        }
        // from the spend amount lets just use 75 % of it so that we have fees accounted for
        let spendable_amount = (spendable_amount * Fraction::new(75, 100)?)?;
        let fraction = Fraction::new(participating_agents.len() as i64, 1)?;
        // split the spendable_amount into equal amounts for participating agents
        let amount_per_agent = (spendable_amount / fraction)?;
        let amount = UnitMap::load(BTreeMap::from([("0".to_string(), amount_per_agent)]));
        for counterparty in participating_agents {
            match ctx.unyt_create_commitment(CommitmentInput {
                counterparty: counterparty.clone(),
                amount: amount.clone(),
                note: None,
                lane_definitions: Vec::new(),
            }) {
                Ok(tx_id) => {
                    ctx.get_mut()
                        .scenario_values
                        .watched_transactions_mut()
                        .push(tx_id);
                }
                Err(err) => {
                    log::warn!("Failed to create commitment for {counterparty}: {err}");
                }
            }
        }

        // test 5
        // call get_history to confirm that the latest created transactions are returned
        // (mirrors the UI calling get_history after a create)
        match ctx.unyt_get_history(Pagination {
            high_boundary: None,
            per_page: 10,
        }) {
            Ok(history) => {
                log::info!(
                    "Agent {} | get_history returned {} items",
                    ctx.get().cell_id().agent_pubkey(),
                    history.items.len()
                );
            }
            Err(err) => {
                log::warn!("Failed to get history: {err}");
            }
        }
    } else {
        log::warn!(
            "No spendable amount for agent {}, ledger balance: {}",
            ctx.get().cell_id().agent_pubkey(),
            balance,
        );
    }

    // test 6
    // poll get_status for watched transactions (mirrors the UI "watch list" feature);
    // remove transactions once they reach WatchStatus::Completed
    let watched = ctx.get().scenario_values.watched_transactions().clone();
    if !watched.is_empty() {
        log::info!(
            "Agent {} | polling get_status for {} watched transactions",
            ctx.get().cell_id().agent_pubkey(),
            watched.len()
        );
        let mut completed = Vec::new();
        for tx_id in &watched {
            match ctx.unyt_get_status(tx_id.clone()) {
                Ok(state) => {
                    if state.status == WatchStatus::Completed {
                        completed.push(tx_id.clone());
                    }
                }
                Err(err) => {
                    log::warn!("Failed to get_status for {tx_id}: {err}");
                }
            }
        }
        if !completed.is_empty() {
            log::info!(
                "Agent {} | {} watched transactions completed",
                ctx.get().cell_id().agent_pubkey(),
                completed.len()
            );
            ctx.get_mut()
                .scenario_values
                .watched_transactions_mut()
                .retain(|tx| !completed.contains(tx));
        }
    }

    thread::sleep(Duration::from_secs(1));

    Ok(())
}
