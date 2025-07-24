use crate::{
    domino_agent::{AcceptTx, DominoAgentExt, SpendInput},
    handle_scenario_setup::ScenarioValues,
};
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::Units;
use std::{collections::BTreeMap, str::FromStr, thread, time::Duration};
use zfuel::{fraction::Fraction, fuel::ZFuel};

pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();
    let session_started_at = ctx.get().scenario_values.session_start_time.unwrap();
    let network_initialized = ctx.get().scenario_values.network_initialized;
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
    let actionable_transactions = ctx.domino_get_actionable_transactions()?;
    // accept incoming invoices too?
    for transaction in actionable_transactions.spend_actionable {
        let _ = ctx.domino_accept_transaction(AcceptTx {
            address: transaction.id.clone().into(),
            service_network_definition: None,
        });
    }

    // todo: get ledger and calculate how much you can spend in this round

    // test 3
    // get ledger and calculate how much you can spend in this round
    let ledger = ctx.domino_get_ledger()?;
    let balance = ledger.balance.get_base_unyt();
    let fees = ledger.fees;
    let credit_limit = ctx.domino_get_my_current_applied_credit_limit()?;
    let spendable_amount = ((balance - fees)? + credit_limit)?;

    // test 4
    // collect agents and start transacting
    if spendable_amount > ZFuel::from_str("0")? {
        const MAX_NUMBER_OF_AGENTS_NEEDED: usize = 10;
        if ctx.get().scenario_values.participating_agents.len() < MAX_NUMBER_OF_AGENTS_NEEDED {
            let code_templates = ctx.domino_get_code_templates_lib()?;
            // collecte unity authors of the code templates
            let mut unique_agents = code_templates
                .iter()
                .map(|template| template.author.clone())
                .collect::<Vec<_>>();

            // remove yourself from the list
            unique_agents
                .retain(|agent| agent != &ctx.get().cell_id().agent_pubkey().clone().into());
            // remove progenitor from the list
            unique_agents.retain(|agent| {
                agent != &ctx.runner_context().get().progenitor_agent_pubkey().into()
            });
            ctx.get_mut().scenario_values.participating_agents = unique_agents
                .into_iter()
                .map(|agent| agent.into())
                .collect();
        }

        // spend with those agents
        let participating_agents = ctx.get().scenario_values.participating_agents.clone();
        // from the spend amount lets just use 75 % of it so that we have fees accounted for
        let spendable_amount = (spendable_amount * Fraction::new(75, 100)?)?;
        let fraction = Fraction::new(participating_agents.len() as i64, 1)?;
        // todo: split the spendable_amount into equal amounts for participating agents
        let amount_per_agent = (spendable_amount / fraction)?;
        let amount = Units::load(BTreeMap::from([("0".to_string(), amount_per_agent)]));
        for agent in participating_agents {
            let _ = ctx.domino_create_spend(SpendInput {
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
    // thread::sleep(Duration::from_secs(1));

    Ok(())
}
