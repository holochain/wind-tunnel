use super::common::record_sync_lag;
use crate::ArcType;
use crate::UnytScenarioValues;
use crate::unyt_agent::UnytAgentExt;
use anyhow::anyhow;
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::{AcceptInput, CommitmentInput, Pagination, UnitMap, WatchStatus};
use std::{collections::BTreeMap, thread, time::Duration};
use zfuel::{fraction::Fraction, fuel::ZFuel};

/// Spend agent behaviour shared across Unyt scenarios.
///
/// Metrics are tagged with an `arc` key, indicating zero or full arc.
pub fn agent_behaviour<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    arc_type: ArcType,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();
    let session_started_at = ctx
        .get()
        .scenario_values
        .session_start_time()
        .ok_or(anyhow!("`session_started_at` not set"))?;
    let network_initialized = ctx.get().scenario_values.network_initialized();

    // Test 1
    // Agents need to await retrieval of the global definition before transacting.
    if !network_initialized {
        if ctx.is_network_initialized() {
            log::info!(
                "[agent {}] {arc_type}-arc network initialized",
                ctx.agent_index()
            );
            let metric = ReportMetric::new("global_definition_propagation_time")
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string())
                .with_tag("arc", arc_type.as_tag())
                .with_field("value", session_started_at.elapsed().as_secs());
            reporter.add_custom(metric);
            ctx.get_mut().scenario_values.set_network_initialized(true);
        } else {
            log::info!(
                "[agent {}] network not initialized, waiting",
                ctx.agent_index()
            );
            thread::sleep(Duration::from_secs(2));
            return Ok(());
        }
    }

    // Test 2
    // Agent ready to transact.
    // check incoming transactions and accept them so that you can have more to spend
    let actionable_transactions = match ctx.unyt_get_actionable_transactions() {
        Ok(txs) => txs,
        Err(err) => {
            log::warn!(
                "[agent {}] failed to get actionable transactions (transient DHT issue): {err}",
                ctx.agent_index()
            );
            thread::sleep(Duration::from_secs(1));
            return Ok(());
        }
    };
    // Measure sync lag for newly discovered commitment transactions
    record_sync_lag(
        ctx,
        &arc_type,
        &actionable_transactions.commitment_actionable,
        "commitment",
    );

    log::info!(
        "[agent {}] {} incoming commitments",
        ctx.agent_index(),
        actionable_transactions.commitment_actionable.len()
    );
    for transaction in actionable_transactions.commitment_actionable {
        if let Err(err) = ctx.unyt_create_accept(AcceptInput {
            commitment: transaction.id.clone(),
            note: None,
        }) {
            log::warn!(
                "[agent {}] failed to accept commitment {}: {err}",
                ctx.agent_index(),
                transaction.id
            );
        } else {
            log::info!(
                "[agent {}] accepted commitment {}",
                ctx.agent_index(),
                transaction.id
            );
        }
    }

    // Test 3
    // get ledger and calculate how much you can spend in this round
    let ledger = match ctx.unyt_get_ledger() {
        Ok(l) => l,
        Err(err) => {
            log::warn!(
                "[agent {}] failed to get ledger (transient DHT issue): {err}",
                ctx.agent_index()
            );
            thread::sleep(Duration::from_secs(1));
            return Ok(());
        }
    };
    let balance = ledger.balance.get_base_unyt();
    let fees = ledger.fees_owed;
    let credit_limit = match ctx.unyt_get_my_current_applied_credit_limit() {
        Ok(cl) => cl,
        Err(err) => {
            log::warn!(
                "[agent {}] failed to get credit limit (transient DHT issue): {err}",
                ctx.agent_index()
            );
            thread::sleep(Duration::from_secs(1));
            return Ok(());
        }
    };
    let spendable_amount = (balance - fees + credit_limit.get_base_unyt())?;
    log::info!(
        "[agent {}] balance: {}, fees: {}, credit_limit: {}, spendable: {}",
        ctx.agent_index(),
        balance,
        fees,
        credit_limit.get_base_unyt(),
        spendable_amount
    );

    // Test 4
    // collect agents and start transacting
    if spendable_amount > ZFuel::zero() {
        ctx.collect_agents()?;

        // Create payment commitments (negative amount) to other agents — 2-step: create + accept.
        // Committing a negative amount means spending/making a payment.
        // Accepting a negative commitment credits the acceptor immediately. A commitment with a
        // positive amount would require a 3rd step of creating a receipt of the accept.
        let participating_agents = ctx.get().scenario_values.participating_agents().to_vec();
        if participating_agents.is_empty() {
            log::warn!(
                "[agent {}] no participating agents to spend with",
                ctx.agent_index()
            );
            return Ok(());
        }
        // from the spend amount lets just use 25 % of it so that we have fees accounted for
        let spendable_amount = (spendable_amount * Fraction::new(25, 100)?)?;
        let fraction = Fraction::new(participating_agents.len() as i64, 1)?;
        // split the spendable_amount into equal amounts for participating agents
        let amount_per_agent = (spendable_amount / fraction)?;
        // Payments must be a negative amount.
        let amount_per_agent = (ZFuel::zero() - amount_per_agent)?;
        let amount = UnitMap::load(BTreeMap::from([("0".to_string(), amount_per_agent)]));
        log::info!(
            "[agent {}] sending {} to {} agents",
            ctx.agent_index(),
            amount_per_agent,
            participating_agents.len()
        );
        for counterparty in participating_agents {
            match ctx.unyt_create_commitment(CommitmentInput {
                counterparty: counterparty.clone(),
                amount: amount.clone(),
                note: None,
                lane_definitions: Vec::new(),
            }) {
                Ok(tx_id) => {
                    log::info!(
                        "[agent {}] sent {} to {}",
                        ctx.agent_index(),
                        amount_per_agent,
                        counterparty
                    );
                    ctx.get_mut()
                        .scenario_values
                        .watched_transactions_mut()
                        .push(tx_id);
                }
                Err(err) => {
                    log::warn!(
                        "[agent {}] failed to create commitment for {counterparty}: {err}",
                        ctx.agent_index()
                    );
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
                    "[agent {}] get_history returned {} items",
                    ctx.agent_index(),
                    history.items.len()
                );
            }
            Err(err) => {
                log::warn!("[agent {}] failed to get history: {err}", ctx.agent_index());
            }
        }
    } else {
        log::warn!(
            "[agent {}] no spendable amount, balance: {}, fees: {}, credit_limit: {}",
            ctx.agent_index(),
            balance,
            fees,
            credit_limit.get_base_unyt(),
        );
    }

    // test 6
    // poll get_status for watched transactions (mirrors the UI "watch list" feature);
    // remove transactions once they reach WatchStatus::Completed
    let watched = ctx.get().scenario_values.watched_transactions().clone();
    if !watched.is_empty() {
        log::info!(
            "[agent {}] polling get_status for {} watched transactions",
            ctx.agent_index(),
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
                    log::warn!(
                        "[agent {}] failed to get_status for {tx_id}: {err}",
                        ctx.agent_index()
                    );
                }
            }
        }
        if !completed.is_empty() {
            log::info!(
                "[agent {}] {} watched transactions completed",
                ctx.agent_index(),
                completed.len()
            );
            ctx.get_mut()
                .scenario_values
                .watched_transactions_mut()
                .retain(|tx| !completed.contains(tx));
        }
    }

    thread::sleep(Duration::from_secs(5));

    Ok(())
}
