use crate::prelude::HolochainRunnerContext;
use anyhow::Context;
use holochain_client_instrumented::prelude::AdminWebsocket;
use wind_tunnel_runner::prelude::{RunnerContext, WindTunnelResult};

/// Sets the `app_ws_url` value in [HolochainRunnerContext] using a valid app port on the target conductor.
///
/// After calling this function you will be able to use the app port in your agent hooks:
/// ```rust
/// use holochain_wind_tunnel_runner::prelude::{HolochainAgentContext, HolochainRunnerContext};
/// use wind_tunnel_runner::prelude::{AgentContext, HookResult};
///
/// fn agent_setup(ctx: &mut AgentContext<HolochainRunnerContext, HolochainAgentContext>) -> HookResult {
///     let app_ws_url = ctx.get().app_ws_url();
/// }
/// ```
///
/// Method:
/// - Connects to an admin port using the connection string from the context.
/// - Lists app interfaces and if there are any, uses the first one.
/// - If there are no app interfaces, attaches a new one.
/// - Reads the current admin URL from the [RunnerContext] and swaps the admin port for the app port.
/// - Sets the `app_ws_url` value in [HolochainRunnerContext].
pub fn configure_app_ws_url(ctx: &mut RunnerContext<HolochainRunnerContext>) -> WindTunnelResult<()> {
    let admin_ws_url = ctx.get_connection_string().to_string();
    let reporter = ctx.reporter();
    let app_port = ctx
        .executor()
        .execute_in_place(async move {
            log::debug!("Connecting a Holochain admin client: {}", admin_ws_url);
            let mut admin_client = AdminWebsocket::connect(admin_ws_url, reporter).await?;

            let existing_app_ports = admin_client
                .list_app_interfaces()
                .await
                .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;
            if !existing_app_ports.is_empty() {
                Ok(*existing_app_ports.first().context("No app ports found")?)
            } else {
                let attached_app_port = admin_client
                    // Don't specify the port, let the conductor pick one
                    .attach_app_interface(0)
                    .await
                    .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;
                Ok(attached_app_port)
            }
        })
        .context("Failed set up app port")?;

    // Use the admin URL with the app port we just got to derive a URL for the app websocket
    let admin_ws_url = ctx.get_connection_string();
    let mut admin_ws_url = url::Url::parse(admin_ws_url)
        .map_err(|e| anyhow::anyhow!("Failed to parse admin URL: {}", e))?;
    admin_ws_url
        .set_port(Some(app_port))
        .map_err(|_| anyhow::anyhow!("Failed to set app port on admin URL"))?;

    ctx.get_mut().app_ws_url = Some(admin_ws_url.to_string());

    Ok(())
}
