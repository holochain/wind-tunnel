use peerkit_wind_tunnel_runner::prelude::*;
use std::sync::OnceLock;
use std::time::Duration;

const INITIATOR: &str = "initiator";
const RESPONDER: &str = "responder";

fn agent_setup(ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>) -> HookResult {
    start_node(ctx)
}

fn initiator_behaviour(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<()> {
    if ctx.get().target_alias.is_none() {
        // The responder's peer ID is derived (injected), not discovered by
        // guessing: both sides compute it from the run ID + behaviour name.
        let responder_id = agent_id_for_behaviour(ctx, RESPONDER);
        let alias = connect_to_agent(ctx, &responder_id, Duration::from_secs(120))?;
        log::info!("connected to responder {responder_id} as alias {alias}");
        ctx.get_mut().target_alias = Some(alias);
    }
    let alias = ctx
        .get()
        .target_alias
        .clone()
        .expect("target alias set above");
    let timestamp = std::time::UNIX_EPOCH
        .elapsed()
        .expect("time went backwards")
        .as_millis();
    send_text(ctx, &alias, &format!("ping-{timestamp}"))?;
    sleep_interval(ctx)
}

fn responder_behaviour(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<()> {
    let messages = take_received_messages(ctx)?;
    if !messages.is_empty() {
        ctx.runner_context().reporter().add_custom(
            ReportMetric::new("peerkit_messages_received")
                .with_tag("behaviour", RESPONDER)
                .with_field("count", messages.len() as u32),
        );
    }
    sleep_interval(ctx)
}

/// The configured send interval, parsed once from `PEERKIT_SEND_INTERVAL_MS`
/// (defaults to 1000ms).
fn send_interval_ms() -> anyhow::Result<u64> {
    static INTERVAL_MS: OnceLock<Result<u64, String>> = OnceLock::new();
    INTERVAL_MS
        .get_or_init(|| {
            std::env::var("PEERKIT_SEND_INTERVAL_MS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .map_err(|e| format!("PEERKIT_SEND_INTERVAL_MS must be a number: {e}"))
        })
        .clone()
        .map_err(anyhow::Error::msg)
}

/// Sleep between behaviour iterations. Configurable with env var
/// `PEERKIT_SEND_INTERVAL_MS`, defaults to 1000.
fn sleep_interval(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<()> {
    let interval_ms = send_interval_ms()?;
    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
            Ok(())
        })
}

fn main() -> WindTunnelResult<()> {
    let builder = PeerkitScenarioDefinitionBuilder::<PeerkitRunnerContext, PeerkitAgentContext>::new_with_init(
        env!("CARGO_PKG_NAME"),
    )?
    .into_std()
    .add_capture_env("PEERKIT_SEND_INTERVAL_MS")
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour(INITIATOR, initiator_behaviour)
    .use_named_agent_behaviour(RESPONDER, responder_behaviour)
    .use_agent_teardown(shutdown_node)
    .with_default_duration_s(60);
    run(builder)?;
    Ok(())
}
