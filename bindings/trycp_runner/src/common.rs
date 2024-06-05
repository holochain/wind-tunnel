use crate::context::TryCPAgentContext;
use crate::runner_context::TryCPRunnerContext;
use std::sync::Arc;
use trycp_client_instrumented::prelude::TryCPClient;
use wind_tunnel_runner::prelude::{AgentContext, HookResult, UserValuesConstraint};

/// Connects to a TryCP server using the current agent index and the list of targets.
///
/// Call this function as follows:
/// ```rust
/// use std::path::Path;
/// use trycp_wind_tunnel_runner::prelude::{AgentContext, connect_trycp_client, HookResult, TryCPAgentContext, TryCPRunnerContext};
///
/// fn agent_setup(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     connect_trycp_client(ctx)?;
///     Ok(())
/// }
/// ```
pub fn connect_trycp_client<SV: UserValuesConstraint>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<SV>>,
) -> HookResult {
    let agent_index = ctx.agent_index();

    let target = ctx
        .runner_context()
        .get_connection_string()
        .split(',')
        .nth(agent_index);

    // This should never happen because the behaviour assignment should have checked that there were enough agents.
    let target = target
        .ok_or_else(|| anyhow::anyhow!("Not enough targets to pick a target URL for agent",))?;

    let signer = Arc::new(holochain_client::ClientAgentSigner::default());
    let reporter = ctx.runner_context().reporter();

    let client = ctx.runner_context().executor().execute_in_place({
        let signer = signer.clone();
        async move { Ok(TryCPClient::connect(target, signer.clone(), reporter).await?) }
    })?;

    ctx.get_mut().trycp_client = Some(client);
    ctx.get_mut().signer = Some(signer);

    Ok(())
}

/// Asks the TryCP server to reset all state for managed Holochain instances.
///
/// You must call `connect_trycp_client` before calling this function. Or otherwise ensure that the
/// [TryCPAgentContext] has a TryCPClient set.
///
/// Call this function as follows:
/// ```rust
/// use std::path::Path;
/// use trycp_wind_tunnel_runner::prelude::{AgentContext, connect_trycp_client, HookResult, reset_trycp_remote, TryCPAgentContext, TryCPRunnerContext};
///
/// fn agent_setup(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     connect_trycp_client(ctx)?;
///     reset_trycp_remote(ctx)?;
///     Ok(())
/// }
/// ```
pub fn reset_trycp_remote(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>,
) -> HookResult {
    let client = ctx.get().trycp_client();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            client.reset(None).await?;
            Ok(())
        })?;

    Ok(())
}

/// Disconnects the TryCP client.
///
/// You must call `connect_trycp_client` before calling this function. Or otherwise ensure that the
/// [TryCPAgentContext] has a TryCPClient set.
///
/// Note that you *must* call this function in the agent teardown, otherwise the `Drop`
/// implementation for the TrycpClient will panic when the runner drops the agent context.
///
/// Call this function as follows:
/// ```rust
/// use std::path::Path;
/// use trycp_wind_tunnel_runner::prelude::{AgentContext, connect_trycp_client, disconnect_trycp_client, HookResult, reset_trycp_remote, TryCPAgentContext, TryCPRunnerContext};
///
/// fn agent_setup(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     connect_trycp_client(ctx)?;
///     Ok(())
/// }
///
/// fn agent_teardown(ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext>) -> HookResult {
///     disconnect_trycp_client(ctx)?;
///     Ok(())
/// }
/// ```
pub fn disconnect_trycp_client<SV: UserValuesConstraint>(
    ctx: &mut AgentContext<TryCPRunnerContext, TryCPAgentContext<SV>>,
) -> HookResult {
    let client = ctx.get_mut().take_trycp_client();

    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            // The drop implementation requires launching a new task, so drop inside an async context.
            // Could also do a `runtime.enter` here but that's deliberately not exposed by the runner.
            drop(client);
            Ok(())
        })?;

    Ok(())
}
