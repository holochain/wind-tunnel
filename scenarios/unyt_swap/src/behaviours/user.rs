use crate::lane::{HF_UNIT_INDEX, WHOT_UNIT_INDEX};
use crate::values::ScenarioValues;
use holochain_types::dna::hash_type::Agent;
use holochain_types::dna::{ActionHash, HoloHashB64};
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::{Actionable, CommitmentInput, ReceiptInput, TransactionType, UnitMap};
use std::time::Instant;
use wind_tunnel_unyt_scenario::ArcType;
use wind_tunnel_unyt_scenario::durable_object::DurableObject;
use wind_tunnel_unyt_scenario::unyt_agent::UnytAgentExt;
use zfuel::fuel::ZFuel;

/// The depositor that receives bridged HOT as wHOT by collecting deposit RAVEs.
pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    arc_type: ArcType,
) -> HookResult {
    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();
    let reporter = ctx.runner_context().reporter();

    // Collect any incoming deposit RAVEs (HOT -> wHOT credit).
    let incoming_raves = ctx.unyt_get_incoming_raves()?;
    log::info!(
        "[{arc_type}-arc] Agent {agent_key} | {} incoming raves",
        incoming_raves.len()
    );
    let mut collected = 0u64;
    for rave in incoming_raves {
        match ctx.unyt_create_collect_from_rave(rave) {
            Ok(hash) => {
                collected += 1;
                log::info!("[{arc_type}-arc] Agent {agent_key} | collected deposit RAVE: {hash}");
            }
            Err(err) => {
                log::warn!(
                    "[{arc_type}-arc] Agent {agent_key} | failed to collect from RAVE: {err}"
                )
            }
        }
    }
    let cumulative = {
        let values = &mut ctx.get_mut().scenario_values;
        values.deposit_raves_collected = values.deposit_raves_collected.saturating_add(collected);
        values.deposit_raves_collected
    };
    reporter.add_custom(
        ReportMetric::new("deposit_raves_collected")
            .with_tag("agent", agent_key.clone())
            .with_tag("arc", arc_type.as_tag())
            .with_field("value", cumulative),
    );

    // Finalize accepted commitments: a positive receipt (receiving HF) requires
    // this third step after the swap agent accepts.
    let actionable = ctx
        .unyt_get_actionable_transactions()
        .unwrap_or_else(|err| {
            log::warn!(
                "[{arc_type}-arc] Agent {agent_key} | get_actionable_transactions failed: {err}"
            );
            Actionable {
                proposal_actionable: vec![],
                commitment_actionable: vec![],
                accept_actionable: vec![],
                reject_actionable: vec![],
            }
        });
    let mut receipts = 0u64;
    for accept in actionable.accept_actionable {
        // Correlate this accept back to the commitment it settles (the first entry in
        // the accept's history) so we can measure the swap's end-to-end completion time.
        let commitment_id = accept
            .history
            .iter()
            .find(|tx| tx.tx_type == TransactionType::Commitment)
            .map(|tx| tx.id.clone());

        match ctx.unyt_create_receipt_for_accept(ReceiptInput {
            hash: accept.id.clone(),
            note: None,
        }) {
            Ok(_) => {
                receipts += 1;
                // If we created the matching commitment, record how long the whole
                // wHOT -> HF swap took, from commitment to finalized receipt.
                if let Some(started) = commitment_id.and_then(|id| {
                    ctx.get_mut()
                        .scenario_values
                        .commitment_started_at
                        .remove(&id)
                }) {
                    reporter.add_custom(
                        ReportMetric::new("swap_completion_duration_s")
                            .with_tag("agent", agent_key.clone())
                            .with_tag("arc", arc_type.as_tag())
                            .with_field("value", started.elapsed().as_secs_f64()),
                    );
                }
            }
            Err(err) => log::warn!(
                "[{arc_type}-arc] Agent {agent_key} | failed to create receipt for {}: {err}",
                accept.id
            ),
        }
    }
    let cumulative = {
        let values = &mut ctx.get_mut().scenario_values;
        values.swap_receipts_created = values.swap_receipts_created.saturating_add(receipts);
        values.swap_receipts_created
    };
    reporter.add_custom(
        ReportMetric::new("swap_receipts_created")
            .with_tag("agent", agent_key.clone())
            .with_tag("arc", arc_type.as_tag())
            .with_field("value", cumulative),
    );

    // Swap part of the behavior: only swap the wHOT this agent actually holds.
    let ledger = ctx.unyt_get_ledger()?;
    let whot_balance = ledger.balance.get_safe(&WHOT_UNIT_INDEX.to_string());
    log::info!("[{arc_type}-arc] Agent {agent_key} | swappable wHOT balance: {whot_balance}");
    if whot_balance <= ZFuel::zero() {
        log::info!("[{arc_type}-arc] Agent {agent_key} | no wHOT to swap yet, waiting");
        return Ok(());
    }

    // Get the lane definition to use its hash in the commitment
    let Some(lane) = ctx
        .unyt_get_all_lane()?
        .into_iter()
        .find_map(|l| l.definition)
    else {
        anyhow::bail!("No lane found; agent setup must have failed");
    };
    let lane_hash: ActionHash = lane.definition_hash.clone().into();

    // The swap agent is the commitment counterparty; fetch its key once and cache it.
    let swap_agent = match ctx.get().scenario_values.swap_agent_key.clone() {
        Some(key) => key,
        None => {
            let key: HoloHashB64<Agent> = DurableObject::new().get_swap_agent_key(ctx)?.into();
            ctx.get_mut().scenario_values.swap_agent_key = Some(key.clone());
            key
        }
    };

    // Swap step: commit to pay wHOT and receive HF
    let spent_whot = (ZFuel::zero() - whot_balance)?;
    let amount = UnitMap::from(vec![
        (WHOT_UNIT_INDEX, spent_whot.to_string().as_ref()),
        (HF_UNIT_INDEX, whot_balance.to_string().as_ref()),
    ]);
    match ctx.unyt_create_commitment(CommitmentInput {
        counterparty: swap_agent,
        amount,
        note: None,
        lane_definitions: vec![lane_hash],
    }) {
        Ok(hash) => {
            log::info!(
                "[{arc_type}-arc] Agent {agent_key} | committed {whot_balance} wHOT -> HF: {hash}"
            );
            let cumulative = {
                let values = &mut ctx.get_mut().scenario_values;
                // Start the swap timer; stopped when its accept is finalized above.
                values.commitment_started_at.insert(hash, Instant::now());
                values.swap_commitments_created = values.swap_commitments_created.saturating_add(1);
                values.swap_commitments_created
            };
            reporter.add_custom(
                ReportMetric::new("swap_commitments_created")
                    .with_tag("agent", agent_key.clone())
                    .with_tag("arc", arc_type.as_tag())
                    .with_field("value", cumulative),
            );
        }
        Err(err) => {
            log::warn!("[{arc_type}-arc] Agent {agent_key} | failed to create commitment: {err}")
        }
    }

    Ok(())
}
