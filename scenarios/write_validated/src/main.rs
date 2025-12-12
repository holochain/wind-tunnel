use holochain_types::prelude::ActionHash;
use holochain_wind_tunnel_runner::prelude::*;
use holochain_wind_tunnel_runner::scenario_happ_path;
use validated_integrity::UpdateSampleEntryInput;

fn agent_setup(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    start_conductor_and_configure_urls(ctx)?;
    install_app(
        ctx,
        scenario_happ_path!("validated"),
        &"validated".to_string(),
    )?;

    Ok(())
}

fn agent_behaviour(
    ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>,
) -> HookResult {
    let action_hash: ActionHash = call_zome(
        ctx,
        "validated",
        "create_sample_entry",
        "this is a test entry value",
    )?;

    let _: ActionHash = call_zome(
        ctx,
        "validated",
        "update_sample_entry",
        UpdateSampleEntryInput {
            original: action_hash,
            new_value: "the old string was a bit boring".to_string(),
        },
    )?;

    Ok(())
}

fn main() -> WindTunnelResult<()> {
    let builder =
        ScenarioDefinitionBuilder::<HolochainRunnerContext, HolochainAgentContext>::new_with_init(
            env!("CARGO_PKG_NAME"),
        )
        .with_default_duration_s(60)
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
