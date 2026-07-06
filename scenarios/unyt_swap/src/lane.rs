//! Lane setup, run once by the progenitor.
//!
//! The progenitor creates the credit limit adjustment and bridging smart
//! agreements (authorizing the bridge agent as oracle and bridging_agent),
//! then initializes a lane that wires them together and defines the lane's
//! units, wHOT and HF. `rave_engine` template code (the Rhai execution and
//! JSON signatures) is embedded verbatim from the Unyt smart agreement
//! library.

use anyhow::Context;
use holochain_types::prelude::{ActionHashB64, AgentPubKey, Timestamp};
use holochain_wind_tunnel_runner::prelude::*;
use rave_engine::types::{
    LaneInit, PermissionSpace, TagFilter, UnitIndexMap,
    entries::{
        AddressBook, AgreementDefInput, CodeTemplate, CommonRAVEAgreements, CommonSpecialAgents,
        DataFetchInstruction, EARole, ExecutionEngine, ExecutorRules, InputRules, Instruction,
        LaneBasicProperties, LaneDefinition, NonValidatedQuery, ProvidedBy, RoleQualification,
        SmartAgreement, UnitDefinition, UnytType,
    },
};
use std::time::Duration;
use wind_tunnel_unyt_scenario::UnytScenarioValues;
use wind_tunnel_unyt_scenario::unyt_agent::UnytAgentExt;

// Unit indices are positions in the happ's global unit list, not positions in
// the lane's `unit_definitions`. The happ appends every unit to that one list,
// so initializing the network creates the base unit at index 0 and the lane's
// units follow it in the order they are defined below.
/// The bridged HOT credit the deposit flow issues.
pub const WHOT_UNIT_INDEX: u32 = 1;
/// The HoloFuel the swap flow pays out.
pub const HF_UNIT_INDEX: u32 = 2;

/// Sets up the lane: the credit-limit-adjustment agreement, the bridging
/// agreement, and the lane that wires them to the bridge agent and defines
/// the wHOT and HF units. Returns the initialized lane's definition hash.
pub fn setup_lane<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    bridge_key: &AgentPubKey,
) -> anyhow::Result<ActionHashB64> {
    let credit_limit_adjustment = setup_credit_limit_adjustment(ctx, bridge_key)
        .context("Failed to set up credit limit adjustment agreement")?;
    let bridging_agreement =
        setup_bridging_agreement(ctx, bridge_key).context("Failed to set up bridging agreement")?;
    let lane_init = build_lane_init(ctx, bridge_key, credit_limit_adjustment, bridging_agreement);

    ctx.unyt_initialize_lane(lane_init)
        .context("Failed to initialize lane")
}

/// Creates the credit-limit-adjustment smart agreement, authorizing the
/// bridge agent as the oracle that posts proof-of-deposit links.
fn setup_credit_limit_adjustment<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    bridge_key: &AgentPubKey,
) -> anyhow::Result<ActionHashB64> {
    let code_template_id = ctx.unyt_create_code_template(CodeTemplate {
        version: semver::Version::new(0, 0, 0),
        title: "lane_credit_limit_adjustment_unyt".to_string(),
        execution_engine: ExecutionEngine::Rhai,
        execution_code: rmp_serde::to_vec(include_str!("lane/credit_limit_adjustment.rhai"))?,
        agreement_definition_input: AgreementDefInput::new(serde_json::from_str(include_str!(
            "lane/cla_agreement_definition.json"
        ))?),
        runtime_input_signature: serde_json::from_str(include_str!(
            "lane/cla_runtime_input_signature.json"
        ))?,
        output_signature: serde_json::from_str(include_str!("lane/cla_output_signature.json"))?,
        one_time_run: false,
        aggregate_execution: true,
        tags: vec![TagFilter::Lane("credit-adjustment".to_string())],
        permissions: PermissionSpace::Lane(None),
    })?;

    ctx.unyt_create_smart_agreement(SmartAgreement {
        title: "Lane: credit limit adjustment".to_string(),
        version: semver::Version::new(0, 0, 0),
        code_template_id: code_template_id.into(),
        input_rules: InputRules(vec![
            DataFetchInstruction {
                name: "proof_of_deposit".to_string(),
                instruction: Instruction::ProvidedBy(ProvidedBy("oracle".to_string())),
            },
            DataFetchInstruction {
                name: "previous_execution".to_string(),
                instruction: Instruction::Custom(NonValidatedQuery::GetPreviousExecution),
            },
            // Index of the external unit whose credit limit this adjusts (wHOT).
            DataFetchInstruction {
                name: "unyt_index".to_string(),
                instruction: Instruction::Fixed(serde_json::json!("1")),
            },
        ]),
        roles: vec![EARole {
            ct_role_id: "oracle".to_string(),
            display_name: "oracle".to_string(),
            description: "The oracle role".to_string(),
            qualification: RoleQualification::Authorized(vec![bridge_key.clone().into()]),
        }],
        executor_rules: ExecutorRules::Any,
        tags: vec![TagFilter::Lane("credit-adjustment".to_string())],
        permissions: PermissionSpace::Lane(None),
    })
}

/// Creates the bridging smart agreement, authorizing the bridge agent as
/// the bridging_agent. The withdrawer role stays open (`Any`).
fn setup_bridging_agreement<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    bridge_key: &AgentPubKey,
) -> anyhow::Result<ActionHashB64> {
    let code_template_id = ctx.unyt_create_code_template(CodeTemplate {
        version: semver::Version::new(0, 0, 0),
        title: "lane_bridging_unyt".to_string(),
        execution_engine: ExecutionEngine::Rhai,
        execution_code: rmp_serde::to_vec(include_str!("lane/blockchain_bridging.rhai"))?,
        agreement_definition_input: AgreementDefInput::new(serde_json::from_str(include_str!(
            "lane/bridging_agreement_definition.json"
        ))?),
        runtime_input_signature: serde_json::from_str(include_str!(
            "lane/bridging_runtime_input_signature.json"
        ))?,
        output_signature: serde_json::from_str(include_str!(
            "lane/bridging_output_signature.json"
        ))?,
        one_time_run: false,
        aggregate_execution: true,
        tags: vec![
            TagFilter::Lane("bridging".to_string()),
            TagFilter::Public("bridging".to_string()),
        ],
        permissions: PermissionSpace::Lane(None),
    })?;

    ctx.unyt_create_smart_agreement(SmartAgreement {
        title: "Lane: blockchain bridge".to_string(),
        version: semver::Version::new(0, 0, 0),
        code_template_id: code_template_id.into(),
        input_rules: InputRules(vec![
            provided_by("bridging_agent_allocations", "bridging_agent"),
            provided_by("proof_of_deposit", "bridging_agent"),
            provided_by("withdrawer_allocations", "withdrawer"),
            provided_by("withdraw_contract_address", "withdrawer"),
            provided_by("withdraw_to_address", "withdrawer"),
            DataFetchInstruction {
                name: "previous_execution".to_string(),
                instruction: Instruction::Custom(NonValidatedQuery::GetPreviousExecution),
            },
            executor_provided("coupons"),
            DataFetchInstruction {
                name: "cool_down_period".to_string(),
                instruction: Instruction::Fixed(serde_json::json!("2")),
            },
        ]),
        roles: vec![
            EARole {
                ct_role_id: "bridging_agent".to_string(),
                display_name: "Bridging Agent".to_string(),
                description: "The bridging agent role".to_string(),
                qualification: RoleQualification::Authorized(vec![bridge_key.clone().into()]),
            },
            EARole {
                ct_role_id: "withdrawer".to_string(),
                display_name: "Withdrawer".to_string(),
                description: "The withdrawer role".to_string(),
                qualification: RoleQualification::Any,
            },
        ],
        executor_rules: ExecutorRules::Any,
        tags: vec![TagFilter::Lane("bridging".to_string())],
        permissions: PermissionSpace::Lane(None),
    })
}

/// Builds the lane definition. The bridge agent is the lane's bridging
/// agent, and the progenitor (this caller) is the lane editor. The lane
/// defines wHOT and HF, which take [`WHOT_UNIT_INDEX`] and [`HF_UNIT_INDEX`]
/// in the order they appear here.
fn build_lane_init<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    bridge_key: &AgentPubKey,
    credit_limit_adjustment: ActionHashB64,
    bridging_agreement: ActionHashB64,
) -> LaneInit {
    let progenitor_key: AgentPubKey = ctx.get().cell_id().agent_pubkey().clone();
    // 30 days; runs must not exceed this.
    let expiration_date = (Timestamp::now() + Duration::from_secs(30 * 24 * 60 * 60))
        .expect("30 day expiration is in range");

    LaneInit {
        basic_properties: LaneBasicProperties {
            name: "HOT bridge".to_string(),
            abbreviation: "HOT".to_string(),
            description: "Lane for bridging HOT into wHOT".to_string(),
            url: "https://example.com".to_string(),
            theme: "#084FA2".to_string(),
            lane_editors: vec![progenitor_key.into()],
        },
        definition: LaneDefinition {
            effective_start_date: Timestamp::now(),
            expiration_date,
            special_agents: CommonSpecialAgents {
                bridging_agent: AddressBook {
                    pub_key: bridge_key.clone().into(),
                    address_book_data: serde_json::Value::Null,
                },
                ops_accounts: vec![],
                service_infrastructure_account: None,
                unit_issuers: Default::default(),
            },
            rave_agreements: CommonRAVEAgreements {
                credit_limit_adjustment: credit_limit_adjustment.clone(),
                bridging_agreement: Some(bridging_agreement),
                proof_of_service: credit_limit_adjustment,
            },
            additional_special_agents: vec![],
            additional_rave_agreements: vec![],
            service_units: UnitIndexMap::new(),
        },
        unit_definitions: vec![
            UnitDefinition {
                unit_type: UnytType::default(),
                unit_symbol: "wHOT".to_string(),
                unit_name: "Wrapped HOT".to_string(),
                unit_description: "Bridged HOT credit".to_string(),
                unit_color: "#02b4b3".to_string(),
            },
            UnitDefinition {
                unit_type: UnytType::default(),
                unit_symbol: "HF".to_string(),
                unit_name: "Fuel".to_string(),
                unit_description: "HoloFuel".to_string(),
                unit_color: "#02b4b3".to_string(),
            },
        ],
    }
}

/// A `ProvidedBy` input rule, sourced from the named role's parked links.
fn provided_by(name: &str, role: &str) -> DataFetchInstruction {
    DataFetchInstruction {
        name: name.to_string(),
        instruction: Instruction::ProvidedBy(ProvidedBy(role.to_string())),
    }
}

/// An input rule whose value the executor supplies at execution time.
fn executor_provided(name: &str) -> DataFetchInstruction {
    DataFetchInstruction {
        name: name.to_string(),
        instruction: Instruction::ExecutorProvided,
    }
}
