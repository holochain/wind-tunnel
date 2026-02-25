use holochain_wind_tunnel_runner::happ_path;
use holochain_wind_tunnel_runner::prelude::*;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;
    install_app(ctx, happ_path!("signal"), &"signal".into())?;

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
                    received_count.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                })
                .await?;

            Ok(())
        }
    })?;

    let send_start = Instant::now();
    // Set after the zome call completes. The guard only emits `signal_batch_recv` when this is
    // Some; if the zome call never returns (e.g. shutdown mid-run) no metric is written.
    let recv_start: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));

    // Create a guard that will write recv metrics on drop (even on early exit)
    let reporter = ctx.runner_context().reporter().clone();
    let _metrics_guard = MetricsGuard {
        write_fn: Box::new({
            let reporter = reporter.clone();
            let received_count = received_count.clone();
            let recv_start = recv_start.clone();
            move || {
                let count = received_count.load(std::sync::atomic::Ordering::Acquire);

                // Only emit recv metric if the zome call completed and recv_start was set.
                // If it wasn't set (interrupted before the call returned), there is no
                // meaningful drain time to report.
                if let Some(recv_start_instant) = *recv_start.lock().unwrap() {
                    let recv_elapsed_s = recv_start_instant.elapsed().as_secs_f64();
                    let metric =
                        ReportMetric::new("signal_batch_recv").with_field("value", recv_elapsed_s);
                    reporter.clone().add_custom(metric);
                }

                let metric = ReportMetric::new("signal_success_ratio")
                    .with_field("value", count as f32 / 10_000.0);
                reporter.clone().add_custom(metric);
            }
        }),
    };

    call_zome::<_, (), _>(ctx, "signal", "emit_10k_signals", ())?;
    let send_elapsed_s = send_start.elapsed().as_secs_f64();
    // Start the recv timer now — signals emitted during the zome call are not included.
    *recv_start.lock().unwrap() = Some(Instant::now());

    // Write send metric immediately after zome call
    let metric = ReportMetric::new("signal_batch_send").with_field("value", send_elapsed_s);
    ctx.runner_context().reporter().clone().add_custom(metric);

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

    // Guard will write recv metrics on drop automatically

    Ok(())
}

// Guard struct that writes recv metrics when dropped, ensuring they're always written even on early exit
struct MetricsGuard {
    write_fn: Box<dyn FnOnce() + Send>,
}

impl Drop for MetricsGuard {
    fn drop(&mut self) {
        // FnOnce() gets consumed when executed,
        // since we're bound to a self reference (&mut self) because of the Drop trait,
        // we take it out and replace it with a no-op, so we can call and consume it
        // without consuming self.
        let write_fn = std::mem::replace(&mut self.write_fn, Box::new(|| {}));
        write_fn();
    }
}

fn main() -> WindTunnelResult<()> {
    let builder =
        ScenarioDefinitionBuilder::<HolochainRunnerContext, HolochainAgentContext>::new_with_init(
            env!("CARGO_PKG_NAME"),
        )
        .with_default_duration_s(180)
        .use_build_info(conductor_build_info)
        .use_agent_setup(agent_setup)
        .use_agent_behaviour(agent_behaviour)
        .use_agent_teardown(|ctx| {
            uninstall_app(ctx, None).ok();
            Ok(())
        });

    run(builder)?;

    Ok(())
}
