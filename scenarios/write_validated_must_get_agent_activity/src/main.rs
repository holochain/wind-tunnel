use holochain_types::prelude::{ActionHash, AgentPubKey};
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use std::time::Duration;

#[derive(Debug, Default)]
pub struct ScenarioValues {
    write_peer: Option<AgentPubKey>,
}

impl UserValuesConstraint for ScenarioValues {}

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;
    install_app(
        ctx,
        scenario_happ_path!("validated_must_get_agent_activity"),
        &"validated_must_get_agent_activity".to_string(),
    )?;
    try_wait_for_min_agents(ctx, Duration::from_secs(120))?;

    // 'write' peers create a link to announce their behaviour so 'get_agent_activity' peers can find them
    if ctx.assigned_behaviour() == "write" {
        let _: ActionHash = call_zome(
            ctx,
            "validated_must_get_agent_activity",
            "announce_write_behaviour",
            (),
        )?;
    }

    Ok(())
}

fn agent_behaviour_write(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    let _: () = call_zome(
        ctx,
        "validated_must_get_agent_activity",
        "create_sample_entries_batch",
        25_u64,
    )?;

    Ok(())
}

fn agent_behaviour_must_get_agent_activity(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext<ScenarioValues>>,
) -> HookResult {
    match ctx.get().scenario_values.write_peer.clone() {
        Some(write_peer) => {
            let chain_len: usize = call_zome(
                ctx,
                "validated_must_get_agent_activity",
                "create_validated_sample_entry",
                write_peer.clone(),
            )?;

            let reporter = ctx.runner_context().reporter();
            reporter.add_custom(
                ReportMetric::new("write_validated_must_get_agent_activity")
                    .with_tag(
                        "must_get_agent_activity_agent",
                        ctx.get().cell_id().agent_pubkey().to_string(),
                    )
                    .with_tag("write_agent", write_peer.to_string())
                    .with_field("chain_len", chain_len as f64),
            );
        }
        _ => {
            let maybe_write_peer: Option<AgentPubKey> = call_zome(
                ctx,
                "validated_must_get_agent_activity",
                "get_random_agent_with_write_behaviour",
                (),
            )?;

            if let Some(write_peer) = maybe_write_peer {
                ctx.get_mut().scenario_values.write_peer = Some(write_peer.clone());
            }
        }
    }

    std::thread::sleep(Duration::from_secs(1));

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder = ScenarioDefinitionBuilder::<
        HolochainRunnerContext,
        HolochainAgentContext<ScenarioValues>,
    >::new_with_init(env!("CARGO_PKG_NAME"))
    .with_default_duration_s(60)
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour("write", agent_behaviour_write)
    .use_named_agent_behaviour(
        "must_get_agent_activity",
        agent_behaviour_must_get_agent_activity,
    )
    .use_agent_teardown(|ctx| {
        uninstall_app(ctx, None).ok();
        Ok(())
    });

    run(builder)?;

    Ok(())
}
