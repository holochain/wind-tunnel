use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use std::path::Path;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Instant;

fn setup(ctx: &mut RunnerContext<HolochainRunnerContext>) -> HookResult {
    configure_app_ws_url(ctx)?;
    Ok(())
}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    install_app(ctx, scenario_happ_path!("signal"), &"signal".into())?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    let received_count = Arc::new(AtomicU32::new(0));

    let app_client = ctx.get().app_client();
    ctx.runner_context().executor().execute_in_place({
        let received_count = received_count.clone();
        async move {
            app_client
                .on_signal(move |_signal| {
                    received_count.fetch_add(1, std::sync::atomic::Ordering::Acquire);
                })
                .await?;

            Ok(())
        }
    })?;

    let start = Instant::now();
    call_zome(ctx, "signal", "emit_10k_signals", ())?;
    let send_elapsed_s = start.elapsed().as_secs_f64();

    ctx.runner_context().executor().execute_in_place({
        let received_count = received_count.clone();
        async move {
            tokio::time::timeout(std::time::Duration::from_secs(30), async move {
                loop {
                    let received_count = received_count.load(std::sync::atomic::Ordering::Acquire);
                    if received_count >= 10_000 {
                        break;
                    } else {
                        // Lower values make the metrics more accurate, but a higher value lets the scenario use less CPU.
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    }
                }
            })
            .await
            .ok();

            Ok(())
        }
    })?;

    let recv_elapsed_s = start.elapsed().as_secs_f64();

    let metric = ReportMetric::new("signal_batch_send").with_field("value", send_elapsed_s);
    ctx.runner_context().reporter().clone().add_custom(metric);

    let metric = ReportMetric::new("signal_batch_recv").with_field("value", recv_elapsed_s);
    ctx.runner_context().reporter().clone().add_custom(metric);

    let received_count = received_count.load(std::sync::atomic::Ordering::Acquire);
    let metric =
        ReportMetric::new("signal_success_pct").with_field("value", received_count as f32 / 100.0);
    ctx.runner_context().reporter().clone().add_custom(metric);

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder =
        ScenarioDefinitionBuilder::<HolochainRunnerContext, HolochainAgentContext>::new_with_init(
            env!("CARGO_PKG_NAME"),
        )
        .with_default_duration_s(60)
        .use_setup(setup)
        .use_agent_setup(agent_setup)
        .use_agent_behaviour(agent_behaviour)
        .use_agent_teardown(|ctx| {
            uninstall_app(ctx, None).ok();
            Ok(())
        });

    run(builder)?;

    Ok(())
}
