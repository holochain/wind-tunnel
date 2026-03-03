use crate::{ScenarioValues, durable_object::DurableObject};
use holochain_types::prelude::*;
use holochain_wind_tunnel_runner::prelude::{self as wind_tunnel_prelude, *};
use rave_engine::types::{
    AcceptInput, Actionable, CommitmentInput, CreateParkedSpendInput, History,
    InitializeGlobalDefinition, Ledger, Pagination, PermissionSpace, RAVEExecuteInputs,
    Transaction, UnitMap,
    entries::{
        AgreementDefInput, CodeTemplateExt, ExecutionEngine, GlobalDefinitionExt, RAVE,
        SmartAgreement, SmartAgreementExt, code_template::CodeTemplate,
    },
};

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
        config: InitializeGlobalDefinition,
    ) -> Result<ActionHash, anyhow::Error>;
    fn unyt_create_commitment(
        &mut self,
        commitment: CommitmentInput,
    ) -> Result<ActionHashB64, anyhow::Error>;
    fn unyt_get_actionable_transactions(&mut self) -> Result<Actionable, anyhow::Error>;
    fn unyt_create_accept(
        &mut self,
        accept_input: AcceptInput,
    ) -> Result<ActionHashB64, anyhow::Error>;
    fn unyt_get_ledger(&mut self) -> Result<Ledger, anyhow::Error>;
    fn unyt_get_my_current_applied_credit_limit(&mut self) -> Result<UnitMap, anyhow::Error>;
    fn unyt_get_history(&mut self, pagination: Pagination) -> Result<History, anyhow::Error>;
    fn unyt_get_incoming_raves(&mut self) -> Result<Vec<Transaction>, anyhow::Error>;
    fn unyt_create_collect_from_rave(
        &mut self,
        tx: Transaction,
    ) -> Result<ActionHashB64, anyhow::Error>;
    fn unyt_create_parked_spend(
        &mut self,
        park: CreateParkedSpendInput,
    ) -> Result<ActionHashB64, anyhow::Error>;
    fn unyt_execute_rave(
        &mut self,
        inputs: RAVEExecuteInputs,
    ) -> Result<(RAVE, ActionHash), anyhow::Error>;
    fn unyt_get_requests_to_execute_agreements(
        &mut self,
    ) -> Result<Vec<Transaction>, anyhow::Error>;
}

impl UnytAgentExt for AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>> {
    fn unyt_init(&mut self) -> HookResult {
        self.call_zome_alliance::<_, InitCallbackResult>("init", ())?;
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
                    .is_ok_and(|agreements| !agreements.is_empty())
                })
        } else {
            false
        }
    }

    fn collect_agents(&mut self) -> Result<(), anyhow::Error> {
        const MAX_NUMBER_OF_AGENTS_NEEDED: usize = 50;
        if self.get().scenario_values.participating_agents.len() < MAX_NUMBER_OF_AGENTS_NEEDED {
            let code_templates = self.unyt_get_code_templates_lib()?;
            // collect unity authors of the code templates
            let mut unique_agents = code_templates
                .iter()
                .map(|template| template.author.clone())
                .collect::<Vec<_>>();
            unique_agents.sort();
            unique_agents.dedup();

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
        config: InitializeGlobalDefinition,
    ) -> Result<ActionHash, anyhow::Error> {
        self.call_zome_alliance("initialize_global_definition", config)
    }

    fn unyt_create_commitment(
        &mut self,
        commitment: CommitmentInput,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_commitment", commitment)
    }

    fn unyt_get_actionable_transactions(&mut self) -> Result<Actionable, anyhow::Error> {
        self.call_zome_alliance("get_actionable_transactions", ())
    }

    fn unyt_create_accept(
        &mut self,
        accept_input: AcceptInput,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_accept", accept_input)
    }

    fn unyt_get_ledger(&mut self) -> Result<Ledger, anyhow::Error> {
        self.call_zome_alliance("get_ledger", ())
    }

    fn unyt_get_my_current_applied_credit_limit(&mut self) -> Result<UnitMap, anyhow::Error> {
        self.call_zome_alliance("get_my_current_applied_credit_limit", ())
    }

    fn unyt_get_history(&mut self, pagination: Pagination) -> Result<History, anyhow::Error> {
        self.call_zome_alliance("get_history", pagination)
    }

    fn unyt_get_incoming_raves(&mut self) -> Result<Vec<Transaction>, anyhow::Error> {
        self.call_zome_alliance("get_incoming_raves", ())
    }

    fn unyt_create_collect_from_rave(
        &mut self,
        tx: Transaction,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_collect_from_rave", tx)
    }

    fn unyt_create_parked_spend(
        &mut self,
        park: CreateParkedSpendInput,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_parked_spend", park)
    }

    fn unyt_execute_rave(
        &mut self,
        inputs: RAVEExecuteInputs,
    ) -> Result<(RAVE, ActionHash), anyhow::Error> {
        self.call_zome_alliance("execute_rave", inputs)
    }

    fn unyt_get_requests_to_execute_agreements(
        &mut self,
    ) -> Result<Vec<Transaction>, anyhow::Error> {
        self.call_zome_alliance("get_requests_to_execute_agreements", ())
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
