use crate::handle_scenario_setup::ScenarioValues;
use holochain_types::prelude::*;
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::entries::{
    code_template::CodeTemplate, AgreementDefInput, CodeTemplateExt, ExecutionEngine,
    GlobalDefinition, GlobalDefinitionExt, SmartAgreement, SmartAgreementExt,
};

pub trait DominoAgentExt {
    fn domino_init(&mut self) -> HookResult;
    fn is_network_initialized(&mut self) -> bool;
    fn domino_create_flag_template(&mut self) -> Result<ActionHashB64, anyhow::Error>;
    fn domino_get_current_global_definition(
        &mut self,
    ) -> Result<GlobalDefinitionExt, anyhow::Error>;
    fn domino_get_smart_agreements_for_code_template(
        &mut self,
        code_template_hash: ActionHash,
    ) -> Result<Vec<SmartAgreementExt>, anyhow::Error>;
    fn domino_create_code_template(
        &mut self,
        code_template: CodeTemplate,
    ) -> Result<ActionHashB64, anyhow::Error>;
    fn domino_create_smart_agreement(
        &mut self,
        smart_agreement: SmartAgreement,
    ) -> Result<ActionHashB64, anyhow::Error>;
    fn domino_get_code_templates_lib(&mut self) -> Result<Vec<CodeTemplateExt>, anyhow::Error>;
    fn domino_initialize_global_definition(
        &mut self,
        config: GlobalDefinition,
    ) -> Result<ActionHash, anyhow::Error>;
}

impl DominoAgentExt
    for AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>
{
    fn domino_init(&mut self) -> HookResult {
        let _ = self.call_zome_alliance::<_, String>("init", ())?;
        Ok(())
    }

    fn is_network_initialized(&mut self) -> bool {
        if let Err(_) = self.domino_get_current_global_definition() {
            return false;
        }
        // check if there are any code templates in the lib
        if let Ok(code_templates) = self.domino_get_code_templates_lib() {
            if code_templates.is_empty() {
                return false;
            }
            // check if any titles in code templates start with "__system_credit_limit_computation" if not return false
            let found = code_templates.iter().find(|template| {
                template
                    .title
                    .starts_with("__system_credit_limit_computation")
            });
            match found {
                Some(code_template) => {
                    // check if the code template has a smart agreement
                    if let Err(_) = self.domino_get_smart_agreements_for_code_template(
                        code_template.id.clone().into(),
                    ) {
                        return false;
                    } else {
                        true
                    }
                }
                None => return false,
            }
        } else {
            return false;
        }
    }
    fn domino_create_flag_template(&mut self) -> Result<ActionHashB64, anyhow::Error> {
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
            tags: vec![],
        };
        self.call_zome_alliance("create_code_template", code_template)
    }
    fn domino_get_current_global_definition(
        &mut self,
    ) -> Result<GlobalDefinitionExt, anyhow::Error> {
        self.call_zome_alliance("get_current_global_definition", ())
    }

    fn domino_get_smart_agreements_for_code_template(
        &mut self,
        code_template_hash: ActionHash,
    ) -> Result<Vec<SmartAgreementExt>, anyhow::Error> {
        self.call_zome_alliance("get_smart_agreements_for_code_template", code_template_hash)
    }

    fn domino_create_code_template(
        &mut self,
        code_template: CodeTemplate,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_code_template", code_template)
    }

    fn domino_create_smart_agreement(
        &mut self,
        smart_agreement: SmartAgreement,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_smart_agreement", smart_agreement)
    }

    fn domino_get_code_templates_lib(&mut self) -> Result<Vec<CodeTemplateExt>, anyhow::Error> {
        self.call_zome_alliance("get_code_templates_lib", ())
    }

    fn domino_initialize_global_definition(
        &mut self,
        config: GlobalDefinition,
    ) -> Result<ActionHash, anyhow::Error> {
        self.call_zome_alliance("initialize_global_definition", config)
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
        call_zome(self, "transactor", fn_name, payload)
    }
}
