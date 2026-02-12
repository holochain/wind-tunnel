use crate::{ScenarioValues, unyt_agent::UnytAgentExt};
use holochain_types::prelude::{ActionHashB64, Timestamp};
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::{
    PermissionSpace, UnitIndexMap,
    entries::{
        AddressBook, AgreementDefInput, CodeTemplate, CommonRAVEAgreements, CommonSpecialAgents,
        DataFetchInstruction, EARole, ExecutionEngine, ExecutorRules, GlobalDefinition, InputRules,
        Instruction, LaneDefinition, ProvidedBy, RoleQualification, SmartAgreement,
        SystemRAVEAgreements, TransactionFeeCompute,
    },
};
use serde_json::json;
use std::{thread, time::Duration};
use zfuel::fuel::ZFuel;

/// This behavior is where the progenitor is meant to initialize the network
/// after which it will stay idle for now
pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    // check if network is initialized, if not initialize it
    if !ctx.is_network_initialized() {
        let progenitor_key = ctx.get().cell_id().agent_pubkey().clone();
        log::info!("Progenitor agent {} initializing network", &progenitor_key);
        // create system code templates
        let (credit_limit_smart_agreement, fee_transfer_smart_agreement) = create_agreements(ctx)?;
        //  set global configuration
        let timestamp = Timestamp::now();
        let days = 30; // if test run longger than this days this will need to be updated
        let expiration_date = (timestamp + Duration::from_secs(days * 24 * 60 * 60))?;
        ctx.unyt_initialize_global_definition(GlobalDefinition {
            lane_def: LaneDefinition {
                effective_start_date: timestamp,
                expiration_date,
                special_agents: CommonSpecialAgents {
                    bridging_agent: AddressBook {
                        pub_key: progenitor_key.into(),
                        address_book_data: json!({}),
                    },
                    ops_accounts: vec![],
                    service_infrastructure_account: None,
                },
                rave_agreements: CommonRAVEAgreements {
                    bridging_agreement: None,
                    credit_limit_adjustment: credit_limit_smart_agreement.clone(),
                    proof_of_service: fee_transfer_smart_agreement.clone(),
                },
                additional_special_agents: vec![],
                additional_rave_agreements: vec![],
                service_units: UnitIndexMap::new(),
            },
            system_rave_agreements: SystemRAVEAgreements {
                compute_credit_limit: credit_limit_smart_agreement,
                compute_transaction_fee: TransactionFeeCompute {
                    agreement: fee_transfer_smart_agreement,
                    fee_trigger: ZFuel::new_with_default_precision(100),
                    fee_percentage: 0,
                },
            },
        })?;
        log::info!("Network should be initialized now");
    } else {
        // else just pause since there is nothing else for this agent to do,
        // since the network is initialized
        let ledger = ctx.unyt_get_ledger()?;
        log::info!(
            "Progenitor {} | Ledger: {:?}",
            ctx.get().cell_id().agent_pubkey(),
            ledger
        );
        log::info!("Network is already initialized, pausing for 10 seconds...");
        thread::sleep(Duration::from_secs(20));
    }
    Ok(())
}

fn create_agreements(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> Result<(ActionHashB64, ActionHashB64), anyhow::Error> {
    let credit_limit_hash = ctx.unyt_create_code_template(CodeTemplate {
        version: semver::Version::new(0, 1, 0),
        title: "__system_credit_limit_computation".to_string(),
        execution_engine: ExecutionEngine::Rhai,
        execution_code: rmp_serde::encode::to_vec(
            r#"
                return  #{
                    "output": #{
                        "credit_limit":  inputs.credit_limit.data
                    }
                };  
        "#,
        )
        .unwrap(),
        agreement_definition_input: AgreementDefInput::new(json!({
          "type": "object",
          "properties": {
            "expected_roles": {
              "type": "array",
              "items": [],
              "minItems": 0,
              "maxItems": 0,
              "uniqueItems": true
            }
          },
          "required": ["expected_roles"],
          "additionalProperties": false
        }
        )),
        runtime_input_signature: json!({
          "type": "object",
          "properties": {
            "inputs": {
              "type": "object",
              "properties": {
                "credit_limit": { "type": "object", "additionalProperties": { "type": "string" } }
              }
            }
          },
          "required": ["inputs"]
        }
        ),
        output_signature: json!({
          "type": "object",
          "properties": {
            "credit_limit": {
              "type": "object",
              "additionalProperties": { "type": "string" }
            }
          },
          "required": ["credit_limit"]
        }
        ),
        one_time_run: false,
        aggregate_execution: false,
        tags: vec![],
        permissions: PermissionSpace::Default,
    })?;
    // creating the smart agreement for credit limit
    let credit_limit_smart_agreement = ctx.unyt_create_smart_agreement(SmartAgreement {
        title: "credit check".to_string(),
        version: semver::Version::new(0, 1, 0),
        code_template_id: credit_limit_hash.into(),
        input_rules: InputRules(vec![DataFetchInstruction {
            name: "credit_limit".to_string(),
            instruction: Instruction::Fixed(json!({
              "0": "1000000",
              "1": "1000000"
            })),
        }]),
        roles: vec![],
        executor_rules: ExecutorRules::Any,
        tags: vec![],
        permissions: PermissionSpace::Default,
    })?;

    let fee_transfer_hash = ctx.unyt_create_code_template(CodeTemplate {
        version: semver::Version::new(0, 1, 0),
        title: "__system_transaction_fee_collection".to_string(),
        execution_engine: ExecutionEngine::Rhai,
        execution_code: rmp_serde::encode::to_vec(
            r#"
                let total_amount = 0;
                let merged_allocations = [];
                        
                for a in consumed_inputs.spender_allocations {
                    merged_allocations.push(#{
                        "receiver": inputs.receiver.data,
                        "amount": a.data.amount,
                        "source": a.data.source
                    });
                    total_amount += parse_float(a.data.amount["0"]);
                }

                return #{
                "unyt_allocation": merged_allocations,
                "computed_values": #{
                    "total_amount": total_amount,
                }
                };
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
        }
        )),
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
                "receiver": { "type": "string" }
              }
            }
          },
          "required": ["consumed_inputs", "inputs"]
        }),
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
                "required": ["amount", "receiver", "source"]
              }
            },
            "computed_values": {
              "type": "object",
              "properties": {
                "total_amount": { "type": "string" }
              },
              "required": ["total_amount"]
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
    let fee_transfer_smart_agreement = ctx.unyt_create_smart_agreement(SmartAgreement {
        title: "collect fee v0.1".to_string(),
        version: semver::Version::new(0, 1, 0),
        code_template_id: fee_transfer_hash.into(),
        input_rules: InputRules(vec![
            DataFetchInstruction {
                name: "spender_allocations".to_string(),
                instruction: Instruction::ProvidedBy(ProvidedBy("spender".to_string())),
            },
            DataFetchInstruction {
                name: "receiver".to_string(),
                instruction: Instruction::ExecutorProvided,
            },
        ]),
        roles: vec![EARole {
            ct_role_id: "spender".to_string(),
            display_name: "Spender".to_string(),
            description: "Spender".to_string(),
            qualification: RoleQualification::Any,
        }],
        executor_rules: ExecutorRules::AuthorizedExecutor(
            ctx.get().cell_id().agent_pubkey().clone().into(),
        ),
        tags: vec![],
        permissions: PermissionSpace::Default,
    })?;
    Ok((credit_limit_smart_agreement, fee_transfer_smart_agreement))
}
