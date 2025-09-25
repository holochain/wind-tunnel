use crate::{
    handle_scenario_setup::ScenarioValues,
    unyt_agent::{CreateParkedSpendInput, SAVEDExecuteInputs, UnytAgentExt},
};
use holochain_wind_tunnel_runner::prelude::*;
use rand::seq::SliceRandom;
use rave_engine::types::{
    entries::{
        AgreementDefInput, CodeTemplate, DataFetchInstruction, ExecutionEngine, ExecutorRules,
        InputRules, Instruction, RoleQualification, SmartAgreement,
    },
    ActionHashB64, TransactionDetails,
};
use rave_engine::types::{
    entries::{EARole, ProvidedBy},
    UnitMap,
};
use serde_json::json;
use std::{collections::BTreeMap, str::FromStr, thread, time::Duration};
use zfuel::{fraction::Fraction, fuel::ZFuel};

fn env_number_of_links_processed() -> usize {
    match std::env::var("NUMBER_OF_LINKS_TO_PROCESS")
        .unwrap_or("10".to_string())
        .parse::<usize>()
    {
        Ok(number) => number,
        Err(_) => 10,
    }
}

// pub fn agent_behaviour(
//     ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
// ) -> HookResult {
//     // check if agent is progenitor
//     if ctx.get().cell_id().agent_pubkey() == &ctx.runner_context().get().progenitor_agent_pubkey() {
//         return crate::behaviour::initiate_network::agent_behaviour(ctx);
//     } else {
//         // else continue with smart agreements behaviour
//         agent_behaviour_smart_agreements(ctx)
//     }
// }

pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();
    let session_started_at = ctx.get().scenario_values.session_start_time.unwrap();
    let network_initialized = ctx.get().scenario_values.network_initialized;
    // Test 1: common check for all agents
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

    // test 2: Accepting incoming transactions
    // check incoming SAVED transactions
    log::info!("Checking incoming transactions");
    let incoming_transactions = ctx.unyt_get_incoming_saveds()?;
    for transaction in incoming_transactions {
        log::info!("Collecting incoming transaction: {:?}", transaction);
        let _ = ctx.unyt_collect_from_saved(transaction);
    }

    //test 3
    // execute any smart agreement that is ready to be executed
    // todo: create an env variable to decide how many spends you want to create
    let number_of_links_processed = env_number_of_links_processed();
    log::info!("Getting requests to execute agreements");
    let requests = ctx.unyt_get_requests_to_execute_agreements()?;
    for request in requests {
        // select number of links and pass only NUMBER_OF_LINKS_PROCESSED links
        // Check the
        if let TransactionDetails::GroupedParked { transactions, .. } = request.details {
            let links = transactions;
            let ea_id = request.id;
            log::info!("Executing saved: {:?}", links);
            let _ = ctx.unyt_execute_saved(SAVEDExecuteInputs {
                ea_id: ea_id.into(),
                executor_inputs: json!({}),
                links,
                definition: None,
            });
        }
    }

    // test 3
    // get ledger and calculate how much you can spend in this round
    let ledger = ctx.unyt_get_ledger()?;
    let balance = ledger.balance.get_base_unyt();
    let fees = ledger.fees;
    let credit_limit = ctx.unyt_get_my_current_applied_credit_limit()?;
    let spendable_amount = ((balance - fees)? + credit_limit.get_base_unyt())?;
    // from the spend amount lets just use 75 % of it so that we have fees accounted for
    let spendable_amount = (spendable_amount * Fraction::new(75, 100)?)?;

    // test 4
    // collect agents and start transacting
    if spendable_amount > ZFuel::from_str("0")? {
        ctx.collect_agents()?;

        // get the smart agreement hash
        if let Some(smart_agreement_hash) = generate_smart_agreement(ctx)? {
            // create a parked link spending transaction
            // spend with those agents
            let participating_agents = ctx.get().scenario_values.participating_agents.clone();
            if participating_agents.is_empty() {
                log::warn!("No participating agents to spend with");
                return Ok(());
            }
            // split the spendable_amount into equal amounts for each of the number_of_links_processed transactions

            let fraction = Fraction::new(number_of_links_processed as i64, 1)?;
            // split the spendable_amount into equal amounts for participating agents
            let amount_per_agent = (spendable_amount / fraction)?;
            // expect 1% fees to be paid
            let amount_per_agent = (amount_per_agent * Fraction::new(98, 100)?)?;
            let amount = UnitMap::load(BTreeMap::from([("0".to_string(), amount_per_agent)]));

            for i in 0..number_of_links_processed {
                let agent = &participating_agents[i % participating_agents.len()];
                // create a parked link spending transaction
                let _ = ctx.unyt_create_parked_spend(CreateParkedSpendInput {
                    ea_id: smart_agreement_hash.clone().into(),
                    executor: ctx
                        .get()
                        .scenario_values
                        .executor_pubkey
                        .clone()
                        .expect("Executor pubkey not set")
                        .into(),
                    amount: amount.clone(),
                    spender_payload: json!({
                        "receiver": agent,
                        "pos": "...",
                    }),
                    service_network_definition: None,
                })?;
            }
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

fn generate_smart_agreement(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> Result<Option<ActionHashB64>, anyhow::Error> {
    match ctx.get().scenario_values.smart_agreement_hash.clone() {
        Some(smart_agreement_hash) => {
            log::trace!(
                "Smart agreement already created for agent {}",
                ctx.get().cell_id().agent_pubkey()
            );
            return Ok(Some(smart_agreement_hash));
        }
        None => {}
    }
    // Choose a random executor?
    let executor_pubkey = match ctx
        .get()
        .scenario_values
        .participating_agents
        .choose(&mut rand::thread_rng())
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
        )
        .unwrap(),
        agreement_definition_input: AgreementDefInput::new(json!({
            "type": "object",
            "properties": {
              "expected_roles": {
                "type": "array",
                "items": [
                  {
                    "const": {
                      "id": "spender",
                      "consumed_link": true
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
        executor_rules: ExecutorRules::AuthorizedExecutor(executor_pubkey.clone().into()),
        tags: vec![],
    })?;
    ctx.get_mut().scenario_values.executor_pubkey = Some(executor_pubkey.clone());
    ctx.get_mut().scenario_values.smart_agreement_hash = Some(smart_agreement_hash.clone());
    Ok(Some(smart_agreement_hash))
}
