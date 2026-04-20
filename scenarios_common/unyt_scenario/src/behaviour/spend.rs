use super::common::record_sync_lag;
use crate::ArcType;
use crate::UnytScenarioValues;
use crate::unyt_agent::UnytAgentExt;
use anyhow::anyhow;
use holochain_types::prelude::ActionHashB64;
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::{
    AcceptInput, Actionable, CommitmentInput, Pagination, UnitMap, WatchStatus,
};
use std::str::FromStr;
use std::time::Instant;
use std::{collections::BTreeMap, thread, time::Duration};
use zfuel::{fraction::Fraction, fuel::ZFuel};

#[derive(Debug, Default)]
struct TransactionDetailRefreshOutcome {
    primary_transaction_total_calls: u64,
    primary_transaction_failed_calls: u64,
    related_transaction_total_calls: u64,
    related_transaction_failed_calls: u64,
}

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

    // test 2
    // Refresh calls triggered in the UI
    let ui_routine_started = Instant::now();

    // Call whoami
    if let Err(err) = ctx.unyt_whoami() {
        log::warn!("whoami failed: {err}");
    }

    // Call get_smart_agreement
    if let Some(smart_agreement_hash) = ctx.get().scenario_values.smart_agreement_hash().cloned()
        && let Err(err) = ctx.unyt_get_smart_agreement(smart_agreement_hash)
    {
        log::warn!("get_smart_agreement failed: {err}");
    }

    // Call check_agent_exists for each participating agent
    let participating_agents = ctx.get().scenario_values.participating_agents().to_vec();
    let mut check_agent_exists_total = 0_u64;
    let mut check_agent_exists_failed = 0_u64;
    let mut check_agent_exists_missing = 0_u64;
    let check_agent_exists_started = Instant::now();
    for agent in &participating_agents {
        check_agent_exists_total += 1;
        match ctx.unyt_check_agent_exists(agent.clone().into()) {
            Ok(true) => {}
            Ok(false) => {
                check_agent_exists_missing += 1;
                log::warn!("check_agent_exists reported missing agent: {agent}");
            }
            Err(err) => {
                check_agent_exists_failed += 1;
                log::warn!("check_agent_exists failed for {agent}: {err}");
            }
        }
    }
    if check_agent_exists_total > 0 {
        log::info!(
            "Agent {} | check_agent_exists total={}, failed={}, missing={}",
            ctx.get().cell_id().agent_pubkey(),
            check_agent_exists_total,
            check_agent_exists_failed,
            check_agent_exists_missing
        );
    }
    reporter.add_custom(
        ReportMetric::new("check_agent_exists_duration_s")
            .with_field("value", check_agent_exists_started.elapsed().as_secs_f64()),
    );
    reporter.add_custom(
        ReportMetric::new("check_agent_exists_total_calls")
            .with_field("value", check_agent_exists_total),
    );
    reporter.add_custom(
        ReportMetric::new("check_agent_exists_failed_calls")
            .with_field("value", check_agent_exists_failed),
    );
    reporter.add_custom(
        ReportMetric::new("check_agent_exists_missing_calls")
            .with_field("value", check_agent_exists_missing),
    );

    // Call get global units
    if let Err(err) = ctx.unyt_get_global_units_details() {
        log::warn!("get_global_units_details failed: {err}");
    }

    // Refresh actions lists
    let notification_links = match ctx.unyt_get_all_notification_links() {
        Ok(links) => Some(links),
        Err(err) => {
            log::warn!("get_all_notification_links failed: {err}");
            None
        }
    };

    let mut action_list_actionable: Option<Actionable> = None;
    if let Some(notification_links) = notification_links {
        let action_refresh_started = Instant::now();
        let refresh_results = ctx.unyt_action_list_refresh(notification_links);
        let mut failed_calls = 0_u64;
        if let Err(err) = &refresh_results.actionable_transactions {
            failed_calls = failed_calls.saturating_add(1);
            log::warn!("get_actionable_transactions failed during action list refresh: {err}");
        }
        if let Err(err) = &refresh_results.incoming_raves {
            failed_calls = failed_calls.saturating_add(1);
            log::warn!("get_incoming_raves failed during parallel list refresh: {err}");
        }
        if let Err(err) = &refresh_results.requests_to_execute_agreements {
            failed_calls = failed_calls.saturating_add(1);
            log::warn!(
                "get_requests_to_execute_agreements failed during action list refresh: {err}"
            );
        }
        if let Err(err) = &refresh_results.sorted_requests_to_spend {
            failed_calls = failed_calls.saturating_add(1);
            log::warn!("get_sorted_requests_to_spend failed during action list refresh: {err}");
        }

        action_list_actionable = refresh_results.actionable_transactions.ok().flatten();

        reporter.add_custom(
            ReportMetric::new("ui_action_list_refresh_failed_calls")
                .with_field("value", failed_calls),
        );
        reporter.add_custom(
            ReportMetric::new("ui_action_list_refresh_duration_s")
                .with_field("value", action_refresh_started.elapsed().as_secs_f64()),
        );
    }

    // test 3
    // check incoming transactions and accept them so that you can have more to spend
    let actionable_transactions = action_list_actionable.unwrap_or(Actionable {
        proposal_actionable: vec![],
        commitment_actionable: vec![],
        accept_actionable: vec![],
        reject_actionable: vec![],
    });

    // Refresh watched transactions list
    let watched_transactions = ctx.get().scenario_values.watched_transactions().clone();
    let watchlist_count = watched_transactions.len() as u64;
    if !watched_transactions.is_empty() {
        let detail_refresh_started = Instant::now();
        let mut detail_outcome = TransactionDetailRefreshOutcome::default();

        for transaction_hash in &watched_transactions {
            let detail_item_started = Instant::now();
            let outcome = run_transaction_detail_refresh(ctx, transaction_hash.clone());
            reporter.add_custom(
                ReportMetric::new("ui_transaction_detail_item_refresh_duration_s")
                    .with_field("value", detail_item_started.elapsed().as_secs_f64()),
            );
            reporter.add_custom(
                ReportMetric::new(
                    "ui_transaction_detail_item_refresh_primary_transaction_total_calls",
                )
                .with_field("value", outcome.primary_transaction_total_calls),
            );
            reporter.add_custom(
                ReportMetric::new(
                    "ui_transaction_detail_item_refresh_primary_transaction_failed_calls",
                )
                .with_field("value", outcome.primary_transaction_failed_calls),
            );
            reporter.add_custom(
                ReportMetric::new(
                    "ui_transaction_detail_item_refresh_related_transaction_total_calls",
                )
                .with_field("value", outcome.related_transaction_total_calls),
            );
            reporter.add_custom(
                ReportMetric::new(
                    "ui_transaction_detail_item_refresh_related_transaction_failed_calls",
                )
                .with_field("value", outcome.related_transaction_failed_calls),
            );
            detail_outcome.primary_transaction_total_calls +=
                outcome.primary_transaction_total_calls;
            detail_outcome.primary_transaction_failed_calls +=
                outcome.primary_transaction_failed_calls;
            detail_outcome.related_transaction_total_calls +=
                outcome.related_transaction_total_calls;
            detail_outcome.related_transaction_failed_calls +=
                outcome.related_transaction_failed_calls;
        }

        reporter.add_custom(
            ReportMetric::new("ui_transaction_detail_refresh_duration_s")
                .with_field("value", detail_refresh_started.elapsed().as_secs_f64()),
        );
        reporter.add_custom(
            ReportMetric::new("ui_transaction_detail_refresh_transactions_processed")
                .with_field("value", watchlist_count),
        );
        reporter.add_custom(
            ReportMetric::new("ui_transaction_detail_refresh_primary_transaction_total_calls")
                .with_field("value", detail_outcome.primary_transaction_total_calls),
        );
        reporter.add_custom(
            ReportMetric::new("ui_transaction_detail_refresh_primary_transaction_failed_calls")
                .with_field("value", detail_outcome.primary_transaction_failed_calls),
        );
        reporter.add_custom(
            ReportMetric::new("ui_transaction_detail_refresh_related_transaction_total_calls")
                .with_field("value", detail_outcome.related_transaction_total_calls),
        );
        reporter.add_custom(
            ReportMetric::new("ui_transaction_detail_refresh_related_transaction_failed_calls")
                .with_field("value", detail_outcome.related_transaction_failed_calls),
        );
    }

    // Refresh history list
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

    reporter.add_custom(
        ReportMetric::new("ui_routine_refresh_duration_s")
            .with_field("value", ui_routine_started.elapsed().as_secs_f64()),
    );
    reporter.add_custom(
        ReportMetric::new("ui_routine_refresh_watchlist_count")
            .with_field("value", watchlist_count),
    );

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

    // test 4
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

    // test 5
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

fn run_transaction_detail_refresh<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    transaction_hash: ActionHashB64,
) -> TransactionDetailRefreshOutcome {
    let mut outcome = TransactionDetailRefreshOutcome::default();

    outcome.primary_transaction_total_calls += 1;
    let transaction = match ctx.unyt_get_transaction(transaction_hash.clone()) {
        Ok(transaction) => Some(transaction),
        Err(err) => {
            outcome.primary_transaction_failed_calls += 1;
            log::warn!(
                "get_transaction failed during detail refresh for {transaction_hash}: {err}"
            );
            None
        }
    };

    outcome.primary_transaction_total_calls += 1;
    let state = match ctx.unyt_get_status(transaction_hash.clone()) {
        Ok(state) => Some(state),
        Err(err) => {
            outcome.primary_transaction_failed_calls += 1;
            log::warn!("get_status failed during detail refresh for {transaction_hash}: {err}");
            None
        }
    };

    let related_hash = state
        .as_ref()
        .and_then(first_related_transaction_hash)
        .or_else(|| {
            transaction
                .as_ref()
                .and_then(first_related_transaction_hash)
        });

    if let Some(related_hash) = related_hash {
        outcome.related_transaction_total_calls += 1;
        if let Err(err) = ctx.unyt_get_transaction(related_hash.clone()) {
            outcome.related_transaction_failed_calls += 1;
            log::warn!(
                "related get_transaction failed during detail refresh for {related_hash}: {err}"
            );
        }
    }

    outcome
}

fn first_related_transaction_hash<T: serde::Serialize>(value: &T) -> Option<ActionHashB64> {
    let value = serde_json::to_value(value).ok()?;
    let first_related = value
        .get("related_transaction")?
        .as_array()?
        .first()?
        .clone();
    let related_hash = first_related.get("id")?.as_str()?;
    ActionHashB64::from_str(related_hash).ok()
}
