use holochain_types::prelude::ActionHash;
use std::time::{Duration, Instant};
use trycp_wind_tunnel_runner::prelude::*;
use validated_integrity::UpdateSampleEntryInput;

#[derive(Debug, Default)]
pub struct ScenarioValues {}

impl UserValuesConstraint for ScenarioValues {}

impl AsMut<ScenarioValues> for ScenarioValues {
    fn as_mut(&mut self) -> &mut ScenarioValues {
        self
    }
}

pub fn agent_setup_post_startup_pre_install_hook<Sv>(
    _ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<Sv>>,
) -> HookResult
where
    Sv: UserValuesConstraint + AsMut<ScenarioValues>,
{
    Ok(())
}

pub fn agent_behaviour_hook<Sv>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<Sv>>,
) -> HookResult
where
    Sv: UserValuesConstraint + AsMut<ScenarioValues>,
{
    let reporter = ctx.runner_context().reporter();

    let start = Instant::now();

    let action_hash: ActionHash = call_zome(
        ctx,
        "validated",
        "create_sample_entry",
        "this is a test entry value",
        Some(Duration::from_secs(80)),
    )?;

    reporter.add_custom(
        ReportMetric::new("create_sample_entry_time")
            .with_field("value", start.elapsed().as_secs_f64()),
    );

    let start = Instant::now();

    let _: ActionHash = call_zome(
        ctx,
        "validated",
        "update_sample_entry",
        UpdateSampleEntryInput {
            original: action_hash,
            new_value: "the old string was a bit boring".to_string(),
        },
        Some(Duration::from_secs(80)),
    )?;

    reporter.add_custom(
        ReportMetric::new("update_sample_entry_time")
            .with_field("value", start.elapsed().as_secs_f64()),
    );

    Ok(())
}
