//! Extension trait providing typed wrappers around Unyt zome calls.
//!
//! [`UnytAgentExt`] is implemented for every `AgentContext` whose
//! scenario values satisfy [`UnytScenarioValues`], giving each agent
//! convenient methods for interacting with the Unyt transactor zome.

use crate::UnytScenarioValues;
use holochain_types::prelude::*;
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::{
    AcceptInput, Actionable, BridgingAgentInitiateDepositInput, CommitmentInput,
    CreateParkedLinkInput, CreateParkedSpendInput, History, InitializeGlobalDefinition, LaneExt,
    LaneInit, Ledger, NotificationLinks, Pagination, PermissionSpace, RAVEExecuteInputs, State,
    Transaction, UnitDefinitionExt, UnitMap, ZomeFnInput,
    entries::{
        AgreementDefInput, CodeTemplateExt, CommitmentToProposalInput, CounterProposalInput,
        ExecutionEngine, GlobalDefinitionExt, ProposalInput, RAVE, ReceiptInput, ReclaimInput,
        RejectInput, SmartAgreement, SmartAgreementExt, code_template::CodeTemplate,
    },
};

#[derive(Debug)]
pub struct UnytActionListRefreshResults {
    pub actionable_transactions: anyhow::Result<Option<Actionable>>,
    pub incoming_raves: anyhow::Result<Vec<Transaction>>,
    pub requests_to_execute_agreements: anyhow::Result<Vec<Transaction>>,
    pub sorted_requests_to_spend: anyhow::Result<Vec<Transaction>>,
}

/// Typed helpers for calling the Unyt transactor zome.
///
/// Every method wraps a single zome call on the "alliance" role's
/// `transactor` coordinator zome. Metrics are automatically reported
/// for each call.
pub trait UnytAgentExt {
    /// Calls `init` on the transactor zome.
    fn unyt_init(&mut self) -> HookResult;

    /// Checks whether the Unyt network has been initialized.
    fn is_network_initialized(&mut self) -> bool;

    /// Discovers participating agents from code template authors.
    fn collect_agents(&mut self) -> Result<(), anyhow::Error>;

    /// Creates a minimal "flag" code template.
    fn unyt_create_flag_template(&mut self) -> Result<ActionHashB64, anyhow::Error>;

    /// Retrieves the current global definition.
    fn unyt_get_current_global_definition(&mut self) -> Result<GlobalDefinitionExt, anyhow::Error>;

    /// Lists smart agreements linked to a code template.
    fn unyt_get_smart_agreements_for_code_template(
        &mut self,
        code_template_hash: ActionHash,
    ) -> Result<Vec<SmartAgreementExt>, anyhow::Error>;

    /// Creates a new code template entry.
    fn unyt_create_code_template(
        &mut self,
        code_template: CodeTemplate,
    ) -> Result<ActionHashB64, anyhow::Error>;

    /// Creates a new smart agreement entry.
    fn unyt_create_smart_agreement(
        &mut self,
        smart_agreement: SmartAgreement,
    ) -> Result<ActionHashB64, anyhow::Error>;

    /// Fetches all code templates from the library.
    fn unyt_get_code_templates_lib(&mut self) -> Result<Vec<CodeTemplateExt>, anyhow::Error>;

    /// Initializes the global definition for the network.
    fn unyt_initialize_global_definition(
        &mut self,
        config: InitializeGlobalDefinition,
    ) -> Result<ActionHash, anyhow::Error>;

    /// Creates a new commitment entry.
    fn unyt_create_commitment(
        &mut self,
        commitment: CommitmentInput,
    ) -> Result<ActionHashB64, anyhow::Error>;

    /// Retrieves all actionable transactions for this agent.
    fn unyt_get_actionable_transactions(&mut self) -> Result<Actionable, anyhow::Error>;

    /// Accepts an incoming transaction.
    fn unyt_create_accept(
        &mut self,
        accept_input: AcceptInput,
    ) -> Result<ActionHashB64, anyhow::Error>;

    /// Retrieves this agent's ledger.
    fn unyt_get_ledger(&mut self) -> Result<Ledger, anyhow::Error>;

    /// Returns the agent's current applied credit limit.
    fn unyt_get_my_current_applied_credit_limit(&mut self) -> Result<UnitMap, anyhow::Error>;

    /// Fetches paginated transaction history.
    fn unyt_get_history(&mut self, pagination: Pagination) -> Result<History, anyhow::Error>;

    fn unyt_get_status(&mut self, hash: ActionHashB64) -> Result<State, anyhow::Error>;

    fn unyt_get_transaction(&mut self, hash: ActionHashB64) -> Result<Transaction, anyhow::Error>;

    fn unyt_whoami(&mut self) -> Result<AgentPubKey, anyhow::Error>;

    fn unyt_get_smart_agreement(
        &mut self,
        hash: ActionHashB64,
    ) -> Result<SmartAgreementExt, anyhow::Error>;

    fn unyt_get_all_notification_links(&mut self) -> Result<NotificationLinks, anyhow::Error>;

    fn unyt_check_agent_exists(&mut self, agent: AgentPubKey) -> Result<bool, anyhow::Error>;

    fn unyt_get_global_units_details(&mut self) -> Result<Vec<UnitDefinitionExt>, anyhow::Error>;

    fn unyt_get_sorted_requests_to_spend(&mut self) -> Result<Vec<Transaction>, anyhow::Error>;

    fn unyt_action_list_refresh(
        &mut self,
        notification_links: NotificationLinks,
    ) -> UnytActionListRefreshResults;

    /// Lists incoming RAVE transactions.
    fn unyt_get_incoming_raves(&mut self) -> Result<Vec<Transaction>, anyhow::Error>;

    /// Collects funds from an incoming RAVE transaction.
    fn unyt_create_collect_from_rave(
        &mut self,
        tx: Transaction,
    ) -> Result<ActionHashB64, anyhow::Error>;

    /// Creates a parked spend entry.
    fn unyt_create_parked_spend(
        &mut self,
        park: CreateParkedSpendInput,
    ) -> Result<ActionHashB64, anyhow::Error>;

    /// Executes a RAVE agreement and returns the result.
    fn unyt_execute_rave(
        &mut self,
        inputs: RAVEExecuteInputs,
    ) -> Result<(RAVE, ActionHash), anyhow::Error>;

    /// Lists pending requests to execute agreements.
    fn unyt_get_requests_to_execute_agreements(
        &mut self,
    ) -> Result<Vec<Transaction>, anyhow::Error>;

    /// Creates a new proposal for a negotiated transaction.
    fn unyt_create_proposal(
        &mut self,
        proposal: ProposalInput,
    ) -> Result<ActionHashB64, anyhow::Error>;

    /// Creates a counter-proposal in response to an existing proposal.
    fn unyt_create_counter_proposal(
        &mut self,
        counter_proposal: CounterProposalInput,
    ) -> Result<ActionHashB64, anyhow::Error>;

    /// Commits to an existing proposal, converting it into a commitment.
    fn unyt_create_commit_to_proposal(
        &mut self,
        commit: CommitmentToProposalInput,
    ) -> Result<ActionHashB64, anyhow::Error>;

    /// Rejects a proposal or commitment.
    fn unyt_create_reject_proposal(
        &mut self,
        reject: RejectInput,
    ) -> Result<ActionHashB64, anyhow::Error>;

    /// Creates a receipt acknowledging an accepted transaction.
    fn unyt_create_receipt_for_accept(
        &mut self,
        receipt: ReceiptInput,
    ) -> Result<ActionHashB64, anyhow::Error>;

    /// Reclaims committed balance after a rejection.
    fn unyt_create_reclaim_balance(
        &mut self,
        reclaim: ReclaimInput,
    ) -> Result<ActionHashB64, anyhow::Error>;

    /// Creates a parked link, e.g. an oracle's proof-of-deposit record.
    /// Returns the link's action hash and the target executor's key.
    fn unyt_create_parked_link(
        &mut self,
        input: CreateParkedLinkInput,
    ) -> Result<(ActionHash, AgentPubKey), anyhow::Error>;

    /// Runs the bridging agent's deposit step, turning oracle-posted
    /// proof-of-deposit links into a RAVE that credits the depositor.
    fn unyt_blockchain_bridging_agent_initiate_deposit(
        &mut self,
        input: BridgingAgentInitiateDepositInput,
    ) -> Result<String, anyhow::Error>;

    /// Lists all lanes known to this agent.
    fn unyt_get_all_lane(&mut self) -> Result<Vec<LaneExt>, anyhow::Error>;

    /// Initializes a new lane from its definition.
    fn unyt_initialize_lane(&mut self, input: LaneInit) -> Result<ActionHashB64, anyhow::Error>;

    fn progenitor_init_alpha_env(&mut self) -> Result<(), anyhow::Error>;
}

impl<SV: UnytScenarioValues> UnytAgentExt
    for AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>
{
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
            // check if any titles in code templates start with "__system_credit_limit_computation"
            // if not return false
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
        if self.get().scenario_values.participating_agents().len() < MAX_NUMBER_OF_AGENTS_NEEDED {
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
            if let Some(progenitor_key) = self.get().scenario_values.progenitor_agent_pubkey() {
                let progenitor_b64 = progenitor_key.clone();
                unique_agents.retain(|agent| agent != &progenitor_b64);
            }
            self.get_mut()
                .scenario_values
                .set_participating_agents(unique_agents);
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

    fn unyt_get_status(&mut self, hash: ActionHashB64) -> Result<State, anyhow::Error> {
        let hash: ActionHash = hash.into();
        self.call_zome_alliance("get_status", hash)
    }

    fn unyt_get_transaction(&mut self, hash: ActionHashB64) -> Result<Transaction, anyhow::Error> {
        let hash: ActionHash = hash.into();
        self.call_zome_alliance("get_transaction", hash)
    }

    fn unyt_whoami(&mut self) -> Result<AgentPubKey, anyhow::Error> {
        self.call_zome_alliance("whoami", ())
    }

    fn unyt_get_smart_agreement(
        &mut self,
        hash: ActionHashB64,
    ) -> Result<SmartAgreementExt, anyhow::Error> {
        let hash: ActionHash = hash.into();
        self.call_zome_alliance("get_smart_agreement", hash)
    }

    fn unyt_get_all_notification_links(&mut self) -> Result<NotificationLinks, anyhow::Error> {
        self.call_zome_alliance("get_all_notification_links", Option::<GetStrategy>::None)
    }

    fn unyt_check_agent_exists(&mut self, agent: AgentPubKey) -> Result<bool, anyhow::Error> {
        self.call_zome_alliance("check_agent_exists", agent)
    }

    fn unyt_get_global_units_details(&mut self) -> Result<Vec<UnitDefinitionExt>, anyhow::Error> {
        self.call_zome_alliance("get_global_units_details", ())
    }

    fn unyt_get_sorted_requests_to_spend(&mut self) -> Result<Vec<Transaction>, anyhow::Error> {
        self.call_zome_alliance("get_sorted_requests_to_spend", ())
    }

    /// Mirror the UI behavior when refreshing the actions list
    /// by making the same set of zome calls the UI makes.
    fn unyt_action_list_refresh(
        &mut self,
        notification_links: NotificationLinks,
    ) -> UnytActionListRefreshResults {
        let actionable_links = [
            notification_links.proposal,
            notification_links.commitment,
            notification_links.accept,
            notification_links.reject,
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let actionable_transactions = self
            .call_zome_alliance::<_, Actionable>(
                "get_actionable_transactions",
                ZomeFnInput::with_local(actionable_links, false),
            )
            .map(Some);
        let incoming_raves = self.call_zome_alliance(
            "get_incoming_raves",
            ZomeFnInput::with_local(notification_links.incoming_collect_requests, false),
        );
        let requests_to_execute_agreements = self.call_zome_alliance(
            "get_requests_to_execute_agreements",
            ZomeFnInput::with_local(notification_links.requests_to_execute_agreements, false),
        );
        let sorted_requests_to_spend = self.call_zome_alliance(
            "get_sorted_requests_to_spend",
            ZomeFnInput::with_local(notification_links.requests_to_commit, false),
        );

        UnytActionListRefreshResults {
            actionable_transactions,
            incoming_raves,
            requests_to_execute_agreements,
            sorted_requests_to_spend,
        }
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

    fn unyt_create_proposal(
        &mut self,
        proposal: ProposalInput,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_proposal", proposal)
    }

    fn unyt_create_counter_proposal(
        &mut self,
        counter_proposal: CounterProposalInput,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_counter_proposal", counter_proposal)
    }

    fn unyt_create_commit_to_proposal(
        &mut self,
        commit: CommitmentToProposalInput,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_commit_to_proposal", commit)
    }

    fn unyt_create_reject_proposal(
        &mut self,
        reject: RejectInput,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_reject_proposal", reject)
    }

    fn unyt_create_receipt_for_accept(
        &mut self,
        receipt: ReceiptInput,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_receipt_for_accept", receipt)
    }

    fn unyt_create_reclaim_balance(
        &mut self,
        reclaim: ReclaimInput,
    ) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("create_reclaim_balance", reclaim)
    }

    fn unyt_create_parked_link(
        &mut self,
        input: CreateParkedLinkInput,
    ) -> Result<(ActionHash, AgentPubKey), anyhow::Error> {
        self.call_zome_alliance("create_parked_link", input)
    }

    fn unyt_blockchain_bridging_agent_initiate_deposit(
        &mut self,
        input: BridgingAgentInitiateDepositInput,
    ) -> Result<String, anyhow::Error> {
        self.call_zome_alliance("blockchain_bridging_agent_initiate_deposit", input)
    }

    fn unyt_get_all_lane(&mut self) -> Result<Vec<LaneExt>, anyhow::Error> {
        self.call_zome_alliance("get_all_lane", ())
    }

    fn unyt_initialize_lane(&mut self, input: LaneInit) -> Result<ActionHashB64, anyhow::Error> {
        self.call_zome_alliance("initialize_lane", input)
    }

    fn progenitor_init_alpha_env(&mut self) -> Result<(), anyhow::Error> {
        self.call_zome_alliance("progenitor_init_alpha_env", ())
    }
}

// Helper trait for the zome calling
trait ZomeTransactorExt {
    fn call_zome_alliance<I, O>(&mut self, fn_name: &str, payload: I) -> anyhow::Result<O>
    where
        O: std::fmt::Debug + serde::de::DeserializeOwned,
        I: serde::Serialize + std::fmt::Debug;
}

impl<SV: UnytScenarioValues> ZomeTransactorExt
    for AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>
{
    fn call_zome_alliance<I, O>(&mut self, fn_name: &str, payload: I) -> anyhow::Result<O>
    where
        O: std::fmt::Debug + serde::de::DeserializeOwned,
        I: serde::Serialize + std::fmt::Debug,
    {
        call_zome(self, "transactor", fn_name, payload)
    }
}
