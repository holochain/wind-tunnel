use crate::{domino_agent::DominoAgentExt, handle_scenario_setup::ScenarioValues};
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::{
    entries::{
        AgreementDefInput, CodeTemplate, DataFetchInstruction, EARole, ExecutionEngine,
        ExecutorRules, GlobalDefinition, InputRules, Instruction, ProvidedBy, RoleQualification,
        SmartAgreement, SystemSAVEDAgreements, TransactionFeeCompute,
    },
    ActionHashB64, Timestamp,
};
use serde_json::{json, Value};
use std::{str::FromStr, thread, time::Duration};
use zfuel::fuel::ZFuel;

/// This behavior is where the progenitor is meant to initialize the network
/// after which it will stay idle for now
pub fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    // check if network is initialized, if not initialize it
    if !ctx.is_network_initialized() {
        log::info!(
            "Progenitor agent {} initializing network",
            ctx.get().cell_id().agent_pubkey()
        );
        // create system code templates
        let (credit_limit_smart_agreement, fee_transfer_smart_agreement) = create_agreements(ctx)?;
        //  set global configuration
        let timestamp = Timestamp::now();
        let days = 30; // if test run longger than this days this will need to be updated
        let expiration_date = (timestamp + Duration::from_secs(days * 24 * 60 * 60))?;
        ctx.domino_initialize_global_definition(GlobalDefinition {
            effective_start_date: timestamp,
            expiration_date,
            system_saved_agreements: SystemSAVEDAgreements {
                compute_credit_limit: credit_limit_smart_agreement.into(),
                compute_transaction_fee: TransactionFeeCompute {
                    agreement: fee_transfer_smart_agreement.into(),
                    fee_trigger: ZFuel::from_str("100").unwrap(),
                    fee_percentage: 1,
                },
            },
            additional_special_agents: vec![],
            additional_saved_agreements: vec![],
        })?;
        log::info!("Network should be initialized now");
    } else {
        // else just pause since there is nothing else for this agent to do,
        // since the network is initialized
        thread::sleep(Duration::from_secs(10));
    }
    Ok(())
}

fn create_agreements(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> Result<(ActionHashB64, ActionHashB64), anyhow::Error> {
    let credit_limit_hash = ctx.domino_create_code_template(CodeTemplate {
        version: semver::Version::new(0, 1, 0),
        title: "__system_credit_limit_computation".to_string(),
        execution_engine: ExecutionEngine::Rhai,
        execution_code: rmp_serde::encode::to_vec(
            r#"
                return  #{
                    "credit_limit":  #{
                        "agent": inputs.claiming_agent_pubkey.data,
                        "amount": inputs.credit_limit.data
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
                "claiming_agent_pubkey": { "type": "string" },
                "credit_limit": { "type": "string" }
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
              "properties": {
                "agent": { "type": "string" },
                "amount": { "type": "string" }
              },
              "required": ["agent", "amount"]
            }
          },
          "required": ["credit_limit"]
        }
        ),
        tags: vec![],
    })?;
    // creating the smart agreement for credit limit
    let credit_limit_smart_agreement = ctx.domino_create_smart_agreement(SmartAgreement {
        title: "credit check v0.1.0".to_string(),
        version: semver::Version::new(0, 1, 0),
        code_template_id: credit_limit_hash.into(),
        input_rules: InputRules(vec![
            DataFetchInstruction {
                name: "claiming_agent_pubkey".to_string(),
                instruction: Instruction::ExecutorProvided,
            },
            DataFetchInstruction {
                name: "credit_limit".to_string(),
                instruction: Instruction::Fixed(Value::String("10000".to_string())),
            },
        ]),
        roles: vec![],
        executor_rules: ExecutorRules::Any,
        one_time_run: false,
        aggregate_execution: false,
        tags: vec![],
    })?;

    let fee_transfer_hash = ctx.domino_create_code_template(CodeTemplate {
        version: semver::Version::new(0, 1, 0),
        title: "__system_transaction_fee_collection".to_string(),
        execution_engine: ExecutionEngine::Rhai,
        execution_code: rmp_serde::encode::to_vec(
            r#"
                let total_amount = 0;
                let merged_allocations = [];
                        
                for a in consumed_inputs.allocation {
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
                "allocation": {
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
        tags: vec![],
    })?;
    let fee_transfer_smart_agreement = ctx.domino_create_smart_agreement(SmartAgreement {
        title: "collect fee v0.1".to_string(),
        version: semver::Version::new(0, 1, 0),
        code_template_id: fee_transfer_hash.into(),
        input_rules: InputRules(vec![
            DataFetchInstruction {
                name: "allocation".to_string(),
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
        one_time_run: false,
        aggregate_execution: true,
        tags: vec![],
    })?;
    Ok((credit_limit_smart_agreement, fee_transfer_smart_agreement))
}
