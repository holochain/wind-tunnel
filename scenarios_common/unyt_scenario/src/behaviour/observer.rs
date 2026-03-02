use crate::ArcType;
use crate::UnytScenarioValues;
use crate::unyt_agent::UnytAgentExt;
use anyhow::anyhow;
use holochain_wind_tunnel_runner::prelude::*;
use std::time::SystemTime;
use std::{thread, time::Duration};

/// Observer agent behaviour shared across Unyt scenarios.
///
/// Passively monitors data propagation by polling for new code templates
/// and measuring sync lag. The `arc_type` value is used to tag all emitted
/// metrics.
pub fn agent_behaviour<SV: UnytScenarioValues>(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<SV>>,
    arc_type: ArcType,
) -> HookResult {
    let reporter = ctx.runner_context().reporter();
    let session_started_at = ctx
        .get()
        .scenario_values
        .session_start_time()
        .ok_or(anyhow!("`session_started_at` not set"))?;

    // Wait for network initialization
    if !ctx.get().scenario_values.network_initialized() {
        if ctx.is_network_initialized() {
            log::info!(
                "Network initialized for {arc_type}-arc observer {}",
                ctx.get().cell_id().agent_pubkey()
            );
            reporter.add_custom(
                ReportMetric::new("global_definition_propagation_time")
                    .with_field("at", session_started_at.elapsed().as_secs())
                    .with_tag("agent", ctx.get().cell_id().agent_pubkey().to_string())
                    .with_tag("arc", arc_type.as_tag()),
            );
            ctx.get_mut().scenario_values.set_network_initialized(true);
        } else {
            log::info!(
                "Network not initialized for {arc_type}-arc observer {}, waiting...",
                ctx.get().cell_id().agent_pubkey()
            );
            thread::sleep(Duration::from_secs(2));
            return Ok(());
        }
    }

    // Query code templates to discover new data
    let code_templates = ctx.unyt_get_code_templates_lib()?;
    let agent_pub_key = ctx.get().cell_id().agent_pubkey().to_string();
    let now = SystemTime::now();

    for template in &code_templates {
        let template_id = template.id.clone();
        if ctx
            .get()
            .scenario_values
            .seen_templates()
            .contains(&template_id)
        {
            continue;
        }

        // Measure sync lag using published_at timestamp
        let published_at_us = template.published_at.as_micros() as u128;
        let now_us = now.duration_since(SystemTime::UNIX_EPOCH)?.as_micros();
        let lag_s = now_us.saturating_sub(published_at_us) as f64 / 1e6;

        reporter.add_custom(
            ReportMetric::new("sync_lag")
                .with_tag("agent", agent_pub_key.clone())
                .with_tag("arc", arc_type.as_tag())
                .with_field("value", lag_s),
        );

        ctx.get_mut()
            .scenario_values
            .seen_templates_mut()
            .insert(template_id);
    }

    // Report total discovered items
    reporter.add_custom(
        ReportMetric::new("recv_count")
            .with_tag("agent", agent_pub_key)
            .with_tag("arc", arc_type.as_tag())
            .with_field(
                "value",
                ctx.get().scenario_values.seen_templates().len() as f64,
            ),
    );

    // Throttle to avoid overwhelming
    thread::sleep(Duration::from_secs(2));
    Ok(())
}
