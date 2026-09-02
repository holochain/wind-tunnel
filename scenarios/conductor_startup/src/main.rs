use holochain_types::prelude::{AppBundleSource, InstallAppPayload};
use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;
use std::env::VarError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

static AGENTS_STARTED: AtomicUsize = AtomicUsize::new(0);
static AGENTS_COMPLETED: AtomicUsize = AtomicUsize::new(0);

fn parse_u32_value(name: &str, value: Option<&str>, default: u32) -> WindTunnelResult<u32> {
    let Some(value) = value else {
        return Ok(default);
    };

    value
        .parse()
        .map_err(|error| anyhow::anyhow!("{name} must be an unsigned 32-bit integer: {error}"))
}

fn env_u32(name: &str, default: u32) -> WindTunnelResult<u32> {
    match std::env::var(name) {
        Ok(value) => parse_u32_value(name, Some(&value), default),
        Err(VarError::NotPresent) => parse_u32_value(name, None, default),
        Err(VarError::NotUnicode(_)) => {
            anyhow::bail!("{name} must contain valid Unicode")
        }
    }
}

#[derive(Debug, Default)]
struct ScenarioValues {
    admin_client: Option<AdminWebsocket>,
    pending_apps: Vec<String>,
    cells_total: u32,
    cells_enabled: u32,
    restart_interval: u32,
}

impl UserValuesConstraint for ScenarioValues {}

fn enabled_pct(enabled: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        f64::from(enabled) / f64::from(total) * 100.0
    }
}

fn should_restart(enabled: u32, restart_interval: u32, remaining: usize) -> bool {
    restart_interval > 0 && enabled.is_multiple_of(restart_interval) && remaining > 0
}

fn is_agent_complete(remaining: usize) -> bool {
    remaining == 0
}

fn validate_agent_count(agent_count: usize) -> WindTunnelResult<()> {
    anyhow::ensure!(
        agent_count == 1,
        "conductor_startup requires exactly one agent"
    );
    Ok(())
}

fn validate_cell_count(cell_count: u32) -> WindTunnelResult<u32> {
    anyhow::ensure!(cell_count > 0, "WT_CELL_COUNT must be greater than zero");
    Ok(cell_count)
}

fn validate_connection_mode(connection_string: Option<&str>) -> HookResult {
    anyhow::ensure!(
        connection_string.is_none(),
        "conductor_startup does not support --connection-string because it must restart the conductor"
    );
    Ok(())
}

fn validate_scenario_completion(started: usize, completed: usize) -> WindTunnelResult<()> {
    anyhow::ensure!(started > 0, "conductor_startup did not start any agents");
    anyhow::ensure!(
        completed == started,
        "conductor_startup completed {completed} of {started} agents"
    );
    Ok(())
}

fn scenario_setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    AGENTS_STARTED.store(0, Ordering::SeqCst);
    AGENTS_COMPLETED.store(0, Ordering::SeqCst);
    validate_connection_mode(ctx.get_connection_string())
}

fn record_enable_result(
    values: &mut ScenarioValues,
    result: WindTunnelResult<Duration>,
) -> WindTunnelResult<Duration> {
    let elapsed = result?;
    values.pending_apps.pop();
    values.cells_enabled += 1;
    Ok(elapsed)
}

fn record_startup_metric(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    phase: &str,
    duration: Duration,
    cells_total: u32,
    cells_enabled: u32,
) {
    let metric = ReportMetric::new("conductor_startup")
        .with_tag("agent", ctx.agent_name().to_string())
        .with_tag("phase", phase.to_string())
        .with_field("value", duration.as_secs_f64())
        .with_field("cells_total", cells_total)
        .with_field("cells_enabled", cells_enabled)
        .with_field("enabled_pct", enabled_pct(cells_enabled, cells_total));
    ctx.runner_context().reporter().clone().add_custom(metric);
}

/// Stops and restarts the conductor while recording the start duration.
fn restart_conductor_and_record(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    phase: &str,
) -> HookResult {
    ctx.get_mut().scenario_values.admin_client = None;
    stop_holochain_conductor(ctx)?;

    let start = Instant::now();
    start_holochain_conductor_without_app(ctx)?;
    let elapsed = start.elapsed();

    let values = &ctx.get().scenario_values;
    record_startup_metric(
        ctx,
        phase,
        elapsed,
        values.cells_total,
        values.cells_enabled,
    );

    Ok(())
}

/// Takes the cached admin client or connects a replacement client.
fn take_or_connect_admin_client(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> WindTunnelResult<AdminWebsocket> {
    if let Some(client) = ctx.get_mut().scenario_values.admin_client.take() {
        return Ok(client);
    }

    let admin_ws_url = ctx.get().admin_ws_url();
    let reporter = ctx.runner_context().reporter();
    ctx.runner_context()
        .executor()
        .execute_in_place(
            async move { AdminWebsocket::connect(admin_ws_url, None, reporter).await },
        )
}

fn install_disabled_apps(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    cell_count: u32,
) -> WindTunnelResult<Vec<String>> {
    let admin_client = take_or_connect_admin_client(ctx)?;
    let run_id = ctx.runner_context().get_run_id().to_string();
    let agent_name = ctx.agent_name().to_string();
    let happ_path = happ_path!("callback");

    let (admin_client, app_ids) = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            let agent_key = admin_client.generate_agent_pub_key().await?;
            let content = std::fs::read(happ_path)?;
            let mut app_ids = Vec::new();

            for index in 0..cell_count {
                let app_id = format!("{agent_name}-app-{index}");
                admin_client
                    .install_app(InstallAppPayload {
                        source: AppBundleSource::Bytes(bytes::Bytes::from(content.clone())),
                        agent_key: Some(agent_key.clone()),
                        installed_app_id: Some(app_id.clone()),
                        roles_settings: None,
                        network_seed: Some(format!("{run_id}-{agent_name}-{index}")),
                        ignore_genesis_failure: false,
                        restore_from_dht: false,
                    })
                    .await?;
                app_ids.push(app_id);
            }

            Ok((admin_client, app_ids))
        })?;

    ctx.get_mut().scenario_values.admin_client = Some(admin_client);
    Ok(app_ids)
}

fn enable_app(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
    app_id: String,
) -> WindTunnelResult<Duration> {
    let admin_client = take_or_connect_admin_client(ctx)?;
    let start = Instant::now();
    let admin_client = ctx
        .runner_context()
        .executor()
        .execute_in_place(async move {
            admin_client.enable_app(app_id).await?;
            Ok(admin_client)
        })?;
    let elapsed = start.elapsed();

    ctx.get_mut().scenario_values.admin_client = Some(admin_client);
    Ok(elapsed)
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    AGENTS_STARTED.fetch_add(1, Ordering::SeqCst);
    let cell_count = validate_cell_count(env_u32("WT_CELL_COUNT", 10)?)?;
    ctx.get_mut().scenario_values.restart_interval = env_u32("WT_RESTART_INTERVAL", 0)?;

    let start = Instant::now();
    start_conductor_and_configure_urls(ctx)?;
    let elapsed = start.elapsed();
    record_startup_metric(ctx, "initial", elapsed, 0, 0);

    let pending_apps = install_disabled_apps(ctx, cell_count)?;
    let values = &mut ctx.get_mut().scenario_values;
    values.cells_total = cell_count;
    values.pending_apps = pending_apps;

    restart_conductor_and_record(ctx, "post_install")
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let Some(app_id) = ctx.get().scenario_values.pending_apps.last().cloned() else {
        log::info!("All cells enabled, stopping scenario");
        AGENTS_COMPLETED.fetch_add(1, Ordering::SeqCst);
        ctx.runner_context().force_stop_scenario();
        return Ok(());
    };

    let enable_result = enable_app(ctx, app_id);
    let elapsed = record_enable_result(&mut ctx.get_mut().scenario_values, enable_result)?;
    let values = &ctx.get().scenario_values;
    let total = values.cells_total;
    let enabled = values.cells_enabled;
    let restart_interval = values.restart_interval;
    let remaining = values.pending_apps.len();

    let metric = ReportMetric::new("cell_enable")
        .with_tag("agent", ctx.agent_name().to_string())
        .with_field("value", elapsed.as_secs_f64())
        .with_field("cells_total", total)
        .with_field("cells_enabled", enabled)
        .with_field("enabled_pct", enabled_pct(enabled, total));
    ctx.runner_context().reporter().clone().add_custom(metric);

    if is_agent_complete(remaining) {
        log::info!("All cells enabled, stopping scenario");
        AGENTS_COMPLETED.fetch_add(1, Ordering::SeqCst);
        ctx.runner_context().force_stop_scenario();
        return Ok(());
    }

    if should_restart(enabled, restart_interval, remaining) {
        restart_conductor_and_record(ctx, "periodic")?;
    }

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let cli = init();
    validate_agent_count(cli.agents.unwrap_or(1))?;
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new(env!("CARGO_PKG_NAME"), cli)
    .with_default_duration_s(300)
    .add_capture_env("WT_CELL_COUNT")
    .add_capture_env("WT_RESTART_INTERVAL")
    .use_setup(scenario_setup)
    .use_build_info(conductor_build_info)
    .use_agent_setup(agent_setup)
    .use_agent_behaviour(agent_behaviour);

    run(builder)?;
    validate_scenario_completion(
        AGENTS_STARTED.load(Ordering::SeqCst),
        AGENTS_COMPLETED.load(Ordering::SeqCst),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ScenarioValues, enabled_pct, is_agent_complete, parse_u32_value, record_enable_result,
        should_restart, validate_agent_count, validate_cell_count, validate_connection_mode,
        validate_scenario_completion,
    };
    use std::time::Duration;

    #[test]
    fn enabled_pct_returns_zero_when_no_cells_exist() {
        assert_eq!(enabled_pct(0, 0), 0.0);
    }

    #[test]
    fn enabled_pct_calculates_enabled_cells() {
        assert_eq!(enabled_pct(3, 12), 25.0);
    }

    #[test]
    fn should_restart_only_for_pending_apps_on_the_interval() {
        assert!(should_restart(4, 2, 1));
        assert!(!should_restart(4, 0, 1));
        assert!(!should_restart(3, 2, 1));
        assert!(!should_restart(4, 2, 0));
    }

    #[test]
    fn enable_error_preserves_pending_app_state() {
        let mut values = ScenarioValues {
            pending_apps: vec!["pending-app".to_string()],
            ..Default::default()
        };

        let result = record_enable_result(
            &mut values,
            Err(anyhow::anyhow!("transient enable failure")),
        );

        assert!(result.is_err());
        assert_eq!(values.pending_apps, ["pending-app"]);
        assert_eq!(values.cells_enabled, 0);
    }

    #[test]
    fn enable_success_advances_pending_app_state() -> anyhow::Result<()> {
        let mut values = ScenarioValues {
            pending_apps: vec!["pending-app".to_string()],
            ..Default::default()
        };
        let elapsed = Duration::from_millis(25);

        let recorded = record_enable_result(&mut values, Ok(elapsed))?;

        assert_eq!(recorded, elapsed);
        assert!(values.pending_apps.is_empty());
        assert!(is_agent_complete(values.pending_apps.len()));
        assert_eq!(values.cells_enabled, 1);
        Ok(())
    }

    #[test]
    fn zero_cell_count_is_rejected() {
        let error = validate_cell_count(0).expect_err("zero cells must be invalid");

        assert_eq!(error.to_string(), "WT_CELL_COUNT must be greater than zero");
    }

    #[test]
    fn malformed_numeric_configuration_is_rejected() {
        let error = parse_u32_value("WT_CELL_COUNT", Some("abc"), 10)
            .expect_err("malformed cell count must be invalid");

        assert!(error.to_string().contains("WT_CELL_COUNT"));
    }

    #[test]
    fn absent_numeric_configuration_uses_default() -> anyhow::Result<()> {
        assert_eq!(parse_u32_value("WT_CELL_COUNT", None, 10)?, 10);
        Ok(())
    }

    #[test]
    fn external_conductor_mode_is_rejected() {
        let error = validate_connection_mode(Some("ws://127.0.0.1:1234"))
            .expect_err("external conductor mode must be invalid");

        assert!(error.to_string().contains("connection-string"));
    }

    #[test]
    fn incomplete_scenario_is_rejected() {
        let error =
            validate_scenario_completion(1, 0).expect_err("an incomplete scenario must be invalid");

        assert!(error.to_string().contains("completed 0 of 1 agents"));
    }

    #[test]
    fn completed_scenario_is_accepted() -> anyhow::Result<()> {
        validate_scenario_completion(1, 1)
    }

    #[test]
    fn multiple_agents_are_rejected() {
        let error = validate_agent_count(2).expect_err("multiple agents must be invalid");

        assert!(error.to_string().contains("exactly one agent"));
    }
}
