use crate::{durable_object::DurableObject, handle_scenario_setup::ScenarioValues};
use holochain_serialized_bytes::prelude::*;
use holochain_types::prelude::*;
use holochain_wind_tunnel_runner::prelude::{self as wind_tunnel_prelude, *};
use rave_engine::types::{
    Actionable, Completed, Ledger, PermissionSpace, Transaction, UnitMap,
    entries::{
        AgreementDefInput, CodeTemplateExt, ExecutionEngine, GlobalDefinition, GlobalDefinitionExt,
        RAVE, SmartAgreement, SmartAgreementExt, code_template::CodeTemplate,
    },
};
use serde_json::Value;

// todo: move to rave_engine
#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
pub struct SpendInput {
    pub receiver: AgentPubKeyB64,
    pub amount: UnitMap,
    pub note: Option<String>,
    pub service_network_definition: Option<ActionHash>,
}

#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
pub struct AcceptTx {
    pub address: ActionHash,
    pub service_network_definition: Option<ActionHash>,
}
#[derive(Serialize, Deserialize, Debug, Clone, SerializedBytes)]
pub struct CreateParkedSpendInput {
    pub ea_id: ActionHash,
    pub executor: AgentPubKey,
    pub amount: UnitMap,
    pub spender_payload: Value,
    pub service_network_definition: Option<ActionHash>,
}
#[derive(Serialize, Deserialize, Debug, Clone, SerializedBytes)]
pub struct SAVEDExecuteInputs {
    pub executor_inputs: Value,
    pub ea_id: ActionHash,
    // if empty, we assume that the executor will provide the links
    #[serde(default)]
    pub links: Vec<Transaction>,
    #[serde(default)]
    pub definition: Option<ActionHash>,
}

pub trait UnytAgentExt {
    fn unyt_init(&mut self) -> HookResult;
    fn is_network_initialized(&mut self) -> bool;
    fn collect_agents(&mut self) -> Result<(), anyhow::Error>;
    fn unyt_create_flag_template(&mut self) -> Result<ActionHashB64, anyhow::Error>;
    fn unyt_get_current_global_definition(&mut self) -> Result<GlobalDefinitionExt, anyhow::Error>;
    fn unyt_get_smart_agreements_for_code_template(
        &mut self,
        code_template_hash: ActionHash,
    ) -> Result<Vec<SmartAgreementExt>, anyhow::Error>;
    fn unyt_create_code_template(
        &mut self,
        code_template: CodeTemplate,
    ) -> Result<ActionHashB64, anyhow::Error>;
    fn unyt_create_smart_agreement(
        &mut self,
        smart_agreement: SmartAgreement,
    ) -> Result<ActionHashB64, anyhow::Error>;
    fn unyt_get_code_templates_lib(&mut self) -> Result<Vec<CodeTemplateExt>, anyhow::Error>;
    fn unyt_initialize_global_definition(
        &mut self,
        config: GlobalDefinition,
    ) -> Result<ActionHash, anyhow::Error>;
    fn unyt_create_spend(&mut self, transaction: SpendInput) -> Result<Transaction, anyhow::Error>;
    fn unyt_get_actionable_transactions(&mut self) -> Result<Actionable, anyhow::Error>;
    fn unyt_accept_transaction(&mut self, tx: AcceptTx) -> Result<Transaction, anyhow::Error>;
    fn unyt_get_ledger(&mut self) -> Result<Ledger, anyhow::Error>;
    fn unyt_get_my_current_applied_credit_limit(&mut self) -> Result<UnitMap, anyhow::Error>;
    fn unyt_get_completed_transactions(&mut self) -> Result<Completed, anyhow::Error>;
    fn unyt_get_incoming_saveds(&mut self) -> Result<Vec<Transaction>, anyhow::Error>;
    fn unyt_collect_from_saved(&mut self, tx: Transaction) -> Result<Transaction, anyhow::Error>;
    fn unyt_create_parked_spend(
        &mut self,
        park: CreateParkedSpendInput,
    ) -> Result<(), anyhow::Error>;
    fn unyt_execute_saved(
        &mut self,
        inputs: SAVEDExecuteInputs,
    ) -> Result<(RAVE, ActionHash), anyhow::Error>;
    fn unyt_get_requests_to_execute_agreements(
        &mut self,
    ) -> Result<Vec<Transaction>, anyhow::Error>;
    fn unyt_get_parked_spend(&mut self) -> Result<Vec<Transaction>, anyhow::Error>;
    fn unyt_get_all_my_executed_saveds(&mut self) -> Result<Vec<Transaction>, anyhow::Error>;
}

impl UnytAgentExt for AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>> {
    fn unyt_init(&mut self) -> HookResult {
        let _ = self.call_zome_alliance::<_, String>("init", ())?;
        Ok(())
    }

    fn is_network_initialized(&mut self) -> bool {
        if self.unyt_get_current_global_definition().is_err() {
            return false;
        }
        // check if there are any code templates in the lib
        if let Ok(code_templates) = self.unyt_get_code_templates_lib() {
            if code_templates.is_empty() {
                return false;
            }
            // check if any titles in code templates start with "__system_credit_limit_computation" if not return false
            code_templates
                .iter()
                .find(|template| {
                    template
                        .title
                        .starts_with("__system_credit_limit_computation")
                })
                .is_some_and(|code_template| {
                    // check if the code template has a smart agreement
                    self.unyt_get_smart_agreements_for_code_template(
                        code_template.id.clone().into(),
                    )
                    .is_ok()
                })
        } else {
            false
        }
    }
    fn collect_agents(&mut self) -> Result<(), anyhow::Error> {
        const MAX_NUMBER_OF_AGENTS_NEEDED: usize = 50;
        if self.get().scenario_values.participating_agents.len() < MAX_NUMBER_OF_AGENTS_NEEDED {
            let code_templates = self.unyt_get_code_templates_lib()?;
            // collecte unity authors of the code templates
            let mut unique_agents = code_templates
                .iter()
                .map(|template| template.author.clone())
                .collect::<Vec<_>>();

            // remove yourself from the list
            let self_key: AgentPubKeyB64 = self.get().cell_id().agent_pubkey().clone().into();
            unique_agents.retain(|agent| agent != &self_key);
            // remove progenitor from the list
            if let Ok(progenitor_key) = DurableObject::new().get_progenitor_key(self) {
                let progenitor_b64: AgentPubKeyB64 = progenitor_key.into();
                unique_agents.retain(|agent| agent != &progenitor_b64);
            }
            self.get_mut().scenario_values.participating_agents = unique_agents;
        }
        Ok(())
    }

    fn unyt_create_flag_template(&mut self) -> Result<ActionHashB64, anyhow::Error> {
        let code_template = CodeTemplate {
            version: semver::Version::new(0, 1, 0),
            title: "my flag".to_string(),
            execution_engine: ExecutionEngine::Rhai,
            execution_code: vec![],
            agreement_definition_input: AgreementDefInput::new(serde_json::json!({})),
            runtime_input_signature: serde_json::json!({
              "type": "object",
              "properties": {
                "inputs": {
                  "type": "object",
                  "properties": { }
                }
              },
              "required": ["inputs"]
            }),
            output_signature: serde_json::json!({
              "type": "object",
              "properties": { },
              "required": []
            }),
            aggregate_execution: false,
            one_time_run: false,
            tags: vec![],
            permissions: PermissionSpace::Default,
        };
        self.call_zome_alliance("create_code_template", code_template)
    }
    fn unyt_get_current_global_definition(&mut self) -> Result<GlobalDefinitionExt, anyhow::Error> {
        self.call_zome_alliance("get_current_global_definition", ())
    }

    fn unyt_get_smart_agreements_for_code_template(
        &mut self,
        code_template_hash: ActionHash,
    ) -> Result<Vec<SmartAgreementExt>, anyhow::Error> {
        self.call_zome_alliance("get_smart_agreements_for_code_template", code_template_hash)
    }

    fn unyt_create_code_template(
        &mut self,
        code_template: CodeTemplate,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_code_template", code_template)
    }

    fn unyt_create_smart_agreement(
        &mut self,
        smart_agreement: SmartAgreement,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_smart_agreement", smart_agreement)
    }

    fn unyt_get_code_templates_lib(&mut self) -> Result<Vec<CodeTemplateExt>, anyhow::Error> {
        self.call_zome_alliance("get_code_templates_lib", ())
    }

    fn unyt_initialize_global_definition(
        &mut self,
        config: GlobalDefinition,
    ) -> Result<ActionHash, anyhow::Error> {
        self.call_zome_alliance("initialize_global_definition", config)
    }

    fn unyt_create_spend(&mut self, transaction: SpendInput) -> Result<Transaction, anyhow::Error> {
        self.call_zome_alliance("create_spend", transaction)
    }

    fn unyt_get_actionable_transactions(&mut self) -> Result<Actionable, anyhow::Error> {
        self.call_zome_alliance("get_actionable_transactions", ())
    }

    fn unyt_accept_transaction(&mut self, tx: AcceptTx) -> Result<Transaction, anyhow::Error> {
        self.call_zome_alliance("accept_transaction", tx)
    }

    fn unyt_get_ledger(&mut self) -> Result<Ledger, anyhow::Error> {
        self.call_zome_alliance("get_ledger", ())
    }

    fn unyt_get_my_current_applied_credit_limit(&mut self) -> Result<UnitMap, anyhow::Error> {
        self.call_zome_alliance("get_my_current_applied_credit_limit", ())
    }

    fn unyt_get_completed_transactions(&mut self) -> Result<Completed, anyhow::Error> {
        self.call_zome_alliance("get_completed_transactions", ())
    }

    fn unyt_get_incoming_saveds(&mut self) -> Result<Vec<Transaction>, anyhow::Error> {
        self.call_zome_alliance("get_incoming_saveds", ())
    }

    fn unyt_collect_from_saved(&mut self, tx: Transaction) -> Result<Transaction, anyhow::Error> {
        self.call_zome_alliance("collect_from_saved", tx)
    }

    fn unyt_create_parked_spend(
        &mut self,
        park: CreateParkedSpendInput,
    ) -> Result<(), anyhow::Error> {
        self.call_zome_alliance("create_parked_spend", park)
    }

    fn unyt_execute_saved(
        &mut self,
        inputs: SAVEDExecuteInputs,
    ) -> Result<(RAVE, ActionHash), anyhow::Error> {
        self.call_zome_alliance("execute_saved", inputs)
    }

    fn unyt_get_requests_to_execute_agreements(
        &mut self,
    ) -> Result<Vec<Transaction>, anyhow::Error> {
        self.call_zome_alliance("get_requests_to_execute_agreements", ())
    }

    fn unyt_get_parked_spend(&mut self) -> Result<Vec<Transaction>, anyhow::Error> {
        self.call_zome_alliance("get_parked_spend", ())
    }

    fn unyt_get_all_my_executed_saveds(&mut self) -> Result<Vec<Transaction>, anyhow::Error> {
        self.call_zome_alliance("get_all_my_executed_saveds", ())
    }
}

// Helper trait for the zome calling
trait ZomeTransactorExt {
    fn call_zome_alliance<I, O>(&mut self, fn_name: &str, payload: I) -> anyhow::Result<O>
    where
        O: std::fmt::Debug + serde::de::DeserializeOwned,
        I: serde::Serialize + std::fmt::Debug;
}

impl ZomeTransactorExt
    for AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>
{
    fn call_zome_alliance<I, O>(&mut self, fn_name: &str, payload: I) -> anyhow::Result<O>
    where
        O: std::fmt::Debug + serde::de::DeserializeOwned,
        I: serde::Serialize + std::fmt::Debug,
    {
        let reporter = self.runner_context().reporter();
        let operation_record = wind_tunnel_prelude::OperationRecord::new(fn_name.to_string());
        let result = call_zome(self, "transactor", fn_name, payload);
        wind_tunnel_prelude::report_operation(reporter.clone(), operation_record, &result);
        result
    }
}
