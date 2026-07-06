use crate::lane::WHOT_UNIT_INDEX;
use crate::values::ScenarioValues;
use holochain_types::dna::{ActionHashB64, AgentPubKey, AgentPubKeyB64};
use holochain_types::prelude::{ActionHash, GetStrategy};
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::{
    CreateParkedLinkInput, CreateParkedSpendInput, ParkedData, ParkedLinkType, RAVEExecuteInputs,
    UnitMap,
};
use serde_json::json;
use std::thread;
use std::time::Duration;
use wind_tunnel_unyt_scenario::UnytScenarioValues as _;
use wind_tunnel_unyt_scenario::unyt_agent::UnytAgentExt;

/// Sepolia contract address the bridge agreement recognizes for HOT deposits.
/// https://sepolia.etherscan.io/address/0xe3e064e3c2eef66cb93da8d8114f5084e92f48d6
const DEPOSIT_CONTRACT_ADDRESS: &str = "0xe3e064e3c2eef66cb93da8d8114f5084e92f48d6";
/// HOT amount deposited per user each round.
const DEPOSIT_AMOUNT: &str = "100";

/// Bridge agent, fulfilling both the oracle and bridge agent functions.
///
/// Each round it walks the discovered users and, for each one, posts a
/// proof-of-deposit parked link (oracle step) then immediately processes it
/// into a deposit RAVE that credits the user with wHOT (bridge agent step).
/// Depositing one user per call keeps the aggregated proof payload within
/// Holochain's link tag size limit.
pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    // Lane must have been set up in agent setup.
    let Some(lane) = ctx
        .unyt_get_all_lane()?
        .into_iter()
        .find_map(|l| l.definition)
    else {
        anyhow::bail!("No lane found; agent setup must have failed");
    };
    let lane_hash: ActionHash = lane.definition_hash.into();

    let credit_limit_adjustment: ActionHash = lane.rave_agreements.credit_limit_adjustment.into();
    let Some(bridging_agreement) = lane.rave_agreements.bridging_agreement.clone() else {
        anyhow::bail!("Lane has no bridging agreement");
    };
    let bridging_agreement: ActionHash = bridging_agreement.into();

    // Discover the users to deposit for.
    ctx.collect_agents()?;
    let users = ctx.get().scenario_values.participating_agents().to_vec();
    if users.is_empty() {
        log::info!("[bridge_agent] no users discovered yet, waiting");
        thread::sleep(Duration::from_secs(2));
        return Ok(());
    }

    let reporter = ctx.runner_context().reporter();
    let self_key = ctx.get().cell_id().agent_pubkey().clone();
    let global_definition = ctx.unyt_get_current_global_definition()?.id;
    let lane_definitions = vec![lane_hash];

    // Oracle step: post a proof-of-deposit parked link for each user.
    let mut completed = 0u64;
    let mut failed = 0u64;
    let total_amount = UnitMap::from(vec![(WHOT_UNIT_INDEX, DEPOSIT_AMOUNT)]);
    for user in users {
        match deposit_for_user(
            ctx,
            &DepositInputs {
                credit_limit_adjustment: credit_limit_adjustment.clone(),
                bridging_agreement: bridging_agreement.clone(),
                global_definition: global_definition.clone(),
                lane_definitions: lane_definitions.clone(),
                self_key: self_key.clone(),
                total_amount: total_amount.clone(),
            },
            &user,
        ) {
            Ok(()) => completed += 1,
            Err(e) => {
                log::warn!("[bridge_agent] deposit failed for user {user}: {e:?}");
                failed += 1;
            }
        }
    }

    let values = &mut ctx.get_mut().scenario_values;
    values.bridge_parked_links_completed = values
        .bridge_parked_links_completed
        .saturating_add(completed);
    values.bridge_parked_links_failed = values.bridge_parked_links_failed.saturating_add(failed);
    let cumulative_completed = values.bridge_parked_links_completed;
    let cumulative_failed = values.bridge_parked_links_failed;
    reporter.add_custom(
        ReportMetric::new("bridge_parked_links_completed")
            .with_tag("agent", self_key.to_string())
            .with_field("value", cumulative_completed),
    );
    reporter.add_custom(
        ReportMetric::new("bridge_parked_links_failed")
            .with_tag("agent", self_key.to_string())
            .with_field("value", cumulative_failed),
    );
    log::info!("[bridge_agent] completed {completed} proof-of-deposit links");
    log::info!("[bridge_agent] {failed} proof-of-deposit links failed");

    Ok(())
}

/// Inputs for a single user's deposit round
struct DepositInputs {
    credit_limit_adjustment: ActionHash,
    bridging_agreement: ActionHash,
    global_definition: ActionHashB64,
    lane_definitions: Vec<ActionHash>,
    self_key: AgentPubKey,
    total_amount: UnitMap,
}

/// Runs the oracle then bridging-agent steps for one user, crediting them with wHOT.
fn deposit_for_user(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    inputs: &DepositInputs,
    user: &AgentPubKeyB64,
) -> anyhow::Result<()> {
    let DepositInputs {
        credit_limit_adjustment,
        bridging_agreement,
        global_definition,
        lane_definitions,
        self_key,
        total_amount,
    } = inputs;
    let proofs: Vec<serde_json::Value> = vec![json!({
        "method": "deposit",
        "contract_address": DEPOSIT_CONTRACT_ADDRESS,
        "amount": DEPOSIT_AMOUNT,
        "depositor_wallet_address": user
    })];

    ctx.unyt_create_parked_link(CreateParkedLinkInput {
        ea_id: credit_limit_adjustment.clone(),
        executor: Some(self_key.clone()),
        parked_link_type: ParkedLinkType::ParkedData((
            ParkedData {
                ct_role_id: "oracle".to_string(),
                amount: Some(total_amount.clone()),
                payload: json!({ "proof_of_deposit": proofs.clone() }),
            },
            true,
        )),
    })?;

    ctx.unyt_execute_rave(RAVEExecuteInputs {
        ea_id: credit_limit_adjustment.clone(),
        executor_inputs: serde_json::Value::Null,
        links: vec![],
        global_definition: global_definition.clone().into(),
        lane_definitions: lane_definitions.clone(),
        strategy: GetStrategy::Local,
    })?;

    ctx.unyt_create_parked_spend(CreateParkedSpendInput {
        ea_id: bridging_agreement.clone(),
        executor: Some(self_key.clone()),
        ct_role_id: Some("bridging_agent".to_string()),
        lane_definitions: lane_definitions.clone(),
        amount: total_amount.clone(),
        spender_payload: json!({
            "proof_of_deposit": proofs
        }),
    })?;

    ctx.unyt_execute_rave(RAVEExecuteInputs {
        ea_id: bridging_agreement.clone(),
        executor_inputs: json!({ "coupons": serde_json::Value::Object(serde_json::Map::new()) }),
        links: vec![],
        global_definition: global_definition.clone().into(),
        lane_definitions: lane_definitions.clone(),
        strategy: GetStrategy::Local,
    })?;
    Ok(())
}
