use crate::ArcType;
use crate::UnytScenarioValues;
use crate::unyt_agent::UnytAgentExt;
use anyhow::anyhow;
use holochain_types::prelude::{ActionHashB64, GetStrategy};
use holochain_wind_tunnel_runner::prelude::*;
use rand::seq::IndexedRandom;
use rave_engine::types::{
    AcceptInput, CommitmentInput, CreateParkedSpendInput, PermissionSpace, RAVEExecuteInputs,
    TransactionDetails, UnitMap,
    entries::{
        AgreementDefInput, CodeTemplate, DataFetchInstruction, EARole, ExecutionEngine,
        ExecutorRules, InputRules, Instruction, ProvidedBy, RoleQualification, SmartAgreement,
    },
};
use serde_json::json;
use std::time::SystemTime;
use std::{collections::BTreeMap, thread, time::Duration};
use zfuel::{fraction::Fraction, fuel::ZFuel};

fn env_number_of_links_processed() -> usize {
    std::env::var("UNYT_NUMBER_OF_LINKS_TO_PROCESS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(10)
}

/// Smart agreements agent behaviour shared across Unyt scenarios.
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
    // Test 1: common check for all agents
    if !network_initialized {
        if ctx.is_network_initialized() {
            log::info!(
                "Network initialized for agent {}",
                ctx.get().cell_id().agent_pubkey()
            );
            let metric = ReportMetric::new("global_definition_propagation_time")
                .with_tag("arc", arc_type.as_tag())
                .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string())
                .with_field("value", session_started_at.elapsed().as_secs());
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

    // test 2: Accept incoming commitments and create outgoing commitments
    let actionable_transactions = match ctx.unyt_get_actionable_transactions() {
        Ok(txs) => txs,
        Err(err) => {
            log::warn!("Failed to get actionable transactions (transient DHT issue): {err}");
            thread::sleep(Duration::from_secs(1));
            return Ok(());
        }
    };

    // Accept incoming commitments
    if !actionable_transactions.commitment_actionable.is_empty() {
        log::info!(
            "Agent {} | accepting {} commitment transactions",
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

    // test 3: Accepting incoming RAVE transactions
    log::info!("Checking incoming RAVE transactions");
    let incoming_transactions = match ctx.unyt_get_incoming_raves() {
        Ok(txs) => txs,
        Err(err) => {
            log::warn!("Failed to get incoming RAVEs (transient DHT issue): {err}");
            Vec::new()
        }
    };

    // Measure sync lag for newly discovered RAVE transactions
    let now_us = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_micros();
    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();
    for tx in &incoming_transactions {
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
                .with_tag("tx_type", "rave")
                .with_tag("agent", agent_key.clone())
                .with_tag("arc", arc_type.as_tag())
                .with_field("value", lag_s),
        );
        ctx.get_mut()
            .scenario_values
            .seen_transactions_mut()
            .insert(tx.id.clone());
    }

    for transaction in incoming_transactions {
        log::info!("Collecting incoming transaction: {:?}", transaction);
        if let Err(err) = ctx.unyt_create_collect_from_rave(transaction.clone()) {
            log::warn!("Failed to collect from RAVE, transaction '{transaction:?}': {err}");
        }
    }

    // test 4
    // execute any smart agreement that is ready to be executed
    let number_of_links_processed = env_number_of_links_processed();
    log::info!("Getting requests to execute agreements");
    let requests = match ctx.unyt_get_requests_to_execute_agreements() {
        Ok(reqs) => reqs,
        Err(err) => {
            log::warn!("Failed to get requests to execute agreements (transient DHT issue): {err}");
            Vec::new()
        }
    };

    // Measure sync lag for newly discovered grouped-parked requests (zero-arc only)
    let now_us = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_micros();
    let agent_key = ctx.get().cell_id().agent_pubkey().to_string();
    for tx in &requests {
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
                .with_tag("tx_type", "grouped_parked")
                .with_tag("agent", agent_key.clone())
                .with_tag("arc", arc_type.as_tag())
                .with_field("value", lag_s),
        );
        ctx.get_mut()
            .scenario_values
            .seen_transactions_mut()
            .insert(tx.id.clone());
    }

    if let Ok(global_definition) = ctx.unyt_get_current_global_definition() {
        for request in requests {
            // select number of links and pass only UNYT_NUMBER_OF_LINKS_TO_PROCESS links
            if let TransactionDetails::GroupedParked {
                attached_transactions,
                ..
            } = request.details
            {
                let links: Vec<_> = attached_transactions
                    .into_iter()
                    .take(number_of_links_processed)
                    .collect();
                let ea_id = request.id;
                log::info!("Executing rave: {:?}", links);
                if let Err(err) = ctx.unyt_execute_rave(RAVEExecuteInputs {
                    ea_id: ea_id.into(),
                    executor_inputs: json!({}),
                    links: links.clone(),
                    global_definition: global_definition.id.clone().into(),
                    lane_definitions: Vec::new(),
                    strategy: GetStrategy::default(),
                }) {
                    log::warn!("Failed to execute RAVE with links '{links:?}': {err}");
                };
            }
        }
    } else {
        log::warn!("Failed to get global definition, skipping RAVE execution");
    }

    // test 5: Create commitments to build positive balance
    let ledger = match ctx.unyt_get_ledger() {
        Ok(l) => l,
        Err(err) => {
            log::warn!("Failed to get ledger (transient DHT issue): {err}");
            thread::sleep(Duration::from_secs(1));
            return Ok(());
        }
    };

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

    // Create commitments to other agents to build positive balance (when they accept)
    let spendable_for_commitments = (balance - fees + credit_limit.get_base_unyt())?;
    if spendable_for_commitments > ZFuel::zero() {
        ctx.collect_agents()?;
        let participating_agents = ctx.get().scenario_values.participating_agents().to_vec();

        if !participating_agents.is_empty() {
            let spendable = (spendable_for_commitments * Fraction::new(75, 100)?)?;
            let fraction = Fraction::new(participating_agents.len() as i64, 1)?;
            let amount_per_agent = (spendable / fraction)?;
            let amount = UnitMap::load(BTreeMap::from([("0".to_string(), amount_per_agent)]));

            for counterparty in participating_agents.iter().take(2) {
                // Only send to 2 agents to conserve balance
                if let Err(err) = ctx.unyt_create_commitment(CommitmentInput {
                    counterparty: counterparty.clone(),
                    amount: amount.clone(),
                    note: None,
                    lane_definitions: Vec::new(),
                }) {
                    log::warn!("Failed to create commitment for {counterparty}: {err}");
                }
            }
        }
    }

    // test 6: Create parked spends if we have positive balance
    if balance > fees {
        let spendable_amount = (balance - fees)?;
        let spendable_amount = (spendable_amount * Fraction::new(75, 100)?)?;

        if spendable_amount > ZFuel::zero() {
            ctx.collect_agents()?;
            let participating_agents = ctx.get().scenario_values.participating_agents().to_vec();

            if let Some(smart_agreement_hash) = generate_smart_agreement(ctx)?
                && !participating_agents.is_empty()
            {
                let fraction = Fraction::new(number_of_links_processed as i64, 1)?;
                let amount_per_link = (spendable_amount / fraction)?;
                let amount_per_link = (amount_per_link * Fraction::new(98, 100)?)?;
                let amount = UnitMap::load(BTreeMap::from([("0".to_string(), amount_per_link)]));

                log::info!(
                    "Agent {} | creating {} parked spends with balance {}",
                    ctx.get().cell_id().agent_pubkey(),
                    number_of_links_processed,
                    balance
                );

                for i in 0..number_of_links_processed {
                    let agent = &participating_agents[i % participating_agents.len()];
                    if let Err(err) = ctx.unyt_create_parked_spend(CreateParkedSpendInput {
                        ea_id: smart_agreement_hash.clone().into(),
                        executor: ctx
                            .get()
                            .scenario_values
                            .executor_pubkey()
                            .cloned()
                            .map(Into::into),
                        amount: amount.clone(),
                        spender_payload: json!({
                            "receiver": agent,
                            "pos": "...",
                        }),
                        ct_role_id: None,
                        lane_definitions: Vec::new(),
                    }) {
                        log::warn!("Failed to create parked spend for agent {agent}: {err}");
                    }
                }
            }
        }
    }

    thread::sleep(Duration::from_secs(1));

    Ok(())
}

fn generate_smart_agreement<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
) -> Result<Option<ActionHashB64>, anyhow::Error> {
    if let Some(smart_agreement_hash) = ctx.get().scenario_values.smart_agreement_hash().cloned() {
        log::trace!(
            "Smart agreement already created for agent {}",
            ctx.get().cell_id().agent_pubkey()
        );
        return Ok(Some(smart_agreement_hash));
    }
    // Choose a random executor?
    let executor_pubkey = match ctx
        .get()
        .scenario_values
        .participating_agents()
        .choose(&mut rand::rng())
    {
        Some(executor_pubkey) => executor_pubkey.clone(),
        None => return Ok(None),
    };
    let parked_link_spending_hash = ctx.unyt_create_code_template(CodeTemplate {
        version: semver::Version::new(0, 1, 0),
        title: "parked_link_spending".to_string(),
        execution_engine: ExecutionEngine::Rhai,
        execution_code: rmp_serde::encode::to_vec(
            r#"
                let unyt_allocation = [];
                for a in consumed_inputs.spender_allocations {
                    unyt_allocation.push(#{
                        "receiver": consumed_inputs.receiver[0].data,
                        "amount": a.data.amount,
                        "source": a.data.source
                    });
                }

                return #{
                    "unyt_allocation": unyt_allocation,
                    "computed_values": #{
                        "pos": consumed_inputs.pos[0].data,
                    }
                }
        "#,
        )?,
        agreement_definition_input: AgreementDefInput::new(json!({
            "type": "object",
            "properties": {
              "expected_roles": {
                "type": "array",
                "items": [
                  {
                    "const": {
                      "id": "spender",
                      "parked_link_type": "ParkedSpendCredit"
                    }
                  }
                ],
                "minItems": 1,
                "maxItems": 1,
                "uniqueItems": true
              }
            },
            "required": ["expected_roles"],
            "additionalProperties": false
          }        )),
        runtime_input_signature: json!({
          "type": "object",
          "properties": {
            "consumed_inputs": {
              "type": "object",
              "properties": {
                "spender_allocations": {
                  "type": "array",
                  "items": {
                    "type": "object",
                    "properties": {
                      "amount": { "type": "object", "additionalProperties": { "type": "string" } },
                      "source": { "type": "string" }
                    },
                    "required": ["amount", "source"]
                  }
                }
              }
            },
            "inputs": {
              "type": "object",
              "properties": {
                "receiver": { "type": "array", "items": { "type": "string" } },
                "pos": { "type": "array", "items": { "type": "string" } }
              }
            }
          },
          "required": ["consumed_inputs", "inputs"]
        }
        ),
        output_signature: json!({
          "type": "object",
          "properties": {
            "unyt_allocation": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "receiver": { "type": "string" },
                  "amount": { "type": "object", "additionalProperties": { "type": "string" } },
                  "source": { "type": "string" }
                },
                "required": ["receiver", "amount", "source"]
              }
            },
            "computed_values": {
              "type": "object",
              "properties": {
                "pos": { "type": "string" }
              }
            }
          },
          "required": ["unyt_allocation", "computed_values"]
        }
        ),
        one_time_run: false,
        aggregate_execution: true,
        tags: vec![],
        permissions: PermissionSpace::Default,
    })?;

    // creating the smart agreement for credit limit
    let agent_pubkey = ctx.get().cell_id().agent_pubkey().clone();
    let smart_agreement_hash = ctx.unyt_create_smart_agreement(SmartAgreement {
        title: format!("parked_link_spending for client {}", agent_pubkey),
        version: semver::Version::new(0, 1, 0),
        code_template_id: parked_link_spending_hash.into(),
        input_rules: InputRules(vec![
            DataFetchInstruction {
                name: "spender_allocations".to_string(),
                instruction: Instruction::ProvidedBy(ProvidedBy("spender".to_string())),
            },
            DataFetchInstruction {
                name: "receiver".to_string(),
                instruction: Instruction::ProvidedBy(ProvidedBy("spender".to_string())),
            },
            DataFetchInstruction {
                name: "pos".to_string(),
                instruction: Instruction::ProvidedBy(ProvidedBy("spender".to_string())),
            },
        ]),
        roles: vec![EARole {
            ct_role_id: "spender".to_string(),
            display_name: "Spender".to_string(),
            description: "The spender role".to_string(),
            qualification: RoleQualification::Authorized(vec![agent_pubkey.clone().into()]),
        }],
        executor_rules: ExecutorRules::AuthorizedExecutor(executor_pubkey.clone()),
        tags: vec![],
        permissions: PermissionSpace::Default,
    })?;
    ctx.get_mut()
        .scenario_values
        .set_executor_pubkey(executor_pubkey);
    ctx.get_mut()
        .scenario_values
        .set_smart_agreement_hash(smart_agreement_hash.clone());
    Ok(Some(smart_agreement_hash))
}
