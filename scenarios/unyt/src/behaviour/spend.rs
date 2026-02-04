use crate::{
    handle_scenario_setup::ScenarioValues,
    unyt_agent::{AcceptTx, SpendInput, UnytAgentExt},
};
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::UnitMap;
use std::{collections::BTreeMap, thread, time::Duration};
use zfuel::{fraction::Fraction, fuel::ZFuel};

pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();
    let session_started_at = ctx.get().scenario_values.session_start_time.unwrap();
    let network_initialized = ctx.get().scenario_values.network_initialized;

    let ledger = ctx.unyt_get_ledger()?;
    log::info!(
        "Agent {} | ledger: {:?}",
        ctx.get().cell_id().agent_pubkey(),
        ledger
    );

    // Test 1
    if !network_initialized {
        if ctx.is_network_initialized() {
            log::info!(
                "Network initialized for agent {}",
                ctx.get().cell_id().agent_pubkey()
            );
            reporter.add_custom(
                ReportMetric::new("global_definition_propagation_time")
                    .with_field("at", session_started_at.elapsed().as_secs())
                    .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string()),
            );
            ctx.get_mut().scenario_values.network_initialized = true;
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
    let actionable_transactions = ctx.unyt_get_actionable_transactions()?;
    // accept incoming invoices too?
    if !actionable_transactions.commitment_actionable.is_empty() {
        log::info!(
            "Agent {} | accepting {} transactions",
            ctx.get().cell_id().agent_pubkey(),
            actionable_transactions.commitment_actionable.len()
        );
    }
    for transaction in actionable_transactions.commitment_actionable {
        let _ = ctx.unyt_accept_transaction(AcceptTx {
            address: transaction.id.clone().into(),
            service_network_definition: None,
        });
    }

    // test 3
    // get ledger and calculate how much you can spend in this round
    let ledger = ctx.unyt_get_ledger()?;
    let balance = ledger.balance.get_base_unyt();
    let fees = ledger.fees_owed;
    let credit_limit = ctx.unyt_get_my_current_applied_credit_limit()?;
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
        let participating_agents = ctx.get().scenario_values.participating_agents.clone();
        // from the spend amount lets just use 75 % of it so that we have fees accounted for
        let spendable_amount = (spendable_amount * Fraction::new(75, 100)?)?;
        let fraction = Fraction::new(participating_agents.len() as i64, 1)?;
        // split the spendable_amount into equal amounts for participating agents
        let amount_per_agent = (spendable_amount / fraction)?;
        let amount = UnitMap::load(BTreeMap::from([("0".to_string(), amount_per_agent)]));
        for agent in participating_agents {
            let _ = ctx.unyt_create_spend(SpendInput {
                receiver: agent,
                amount: amount.clone(),
                note: None,
                service_network_definition: None,
            })?;
        }
    } else {
        log::warn!(
            "No spendable amount for agent {}, ledger balance: {}",
            ctx.get().cell_id().agent_pubkey(),
            balance,
        );
    }
    thread::sleep(Duration::from_secs(1));

    Ok(())
}
