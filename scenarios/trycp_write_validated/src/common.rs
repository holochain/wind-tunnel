use holochain_types::prelude::ActionHash;
use std::time::{Duration, Instant};
use trycp_wind_tunnel_runner::prelude::*;
use validated_integrity::UpdateSampleEntryInput;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct ScenarioValues {
    pub write_interval: Duration,
    pub last_write: Arc<Mutex<std::time::Instant>>,
}

fn env_dur(n: &'static str, d: u64) -> Duration {
    match std::env::var(n) {
        Ok(n) => Duration::from_millis(n.parse::<u64>().unwrap()),
        _ => Duration::from_millis(d),
    }
}

impl Default for ScenarioValues {
    fn default() -> Self {
        let write_interval = env_dur("WRITE_INTERVAL_MS", 0);

        Self {
            write_interval,
            last_write: Arc::new(Mutex::new(std::time::Instant::now())),
        }
    }
}

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
    let last_write = ctx.get_mut().scenario_values.as_mut().last_write.clone();
    let write_interval = ctx.get_mut().scenario_values.as_mut().write_interval;

    {
        let now_inst = std::time::Instant::now();
        let mut last_inst = last_write.lock().unwrap();
        if now_inst - *last_inst < write_interval {
            // Don't hammer with signals
            return Ok(());
        }
        *last_inst = now_inst;
    }

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
