use peerkit_wind_tunnel_runner::prelude::*;
use std::time::{Duration, Instant};

const NODE: &str = "node";

/// How long an in-flight receive batch may go without completing before it is
/// dropped and counted as a `receive_incomplete` error.
const RECEIVE_TRACKER_TIMEOUT: Duration = Duration::from_secs(300);

fn env_u64(name: &str, default: u64) -> anyhow::Result<u64> {
    match std::env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|e| anyhow::anyhow!("{name} must be a number: {e}")),
        Err(_) => Ok(default),
    }
}

fn report_error(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    kind: &str,
    count: u64,
) {
    ctx.runner_context().reporter().add_custom(
        ReportMetric::new("peerkit_error_count")
            .with_tag("kind", kind.to_string())
            .with_field("count", count),
    );
}

/// Build one message payload: a `<cycle>.<seq>.` header (parsed back on the
/// receiving side) padded with `x` up to `size` bytes. No whitespace — the
/// CLI normalizes whitespace in `send` arguments.
fn message_payload(cycle: u64, seq: u64, size: usize) -> String {
    let mut payload = format!("{cycle}.{seq}.");
    let fill = size.saturating_sub(payload.len());
    payload.push_str(&"x".repeat(fill));
    payload
}

fn agent_setup(ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>) -> HookResult {
    start_node(ctx)
}

fn node_behaviour(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<()> {
    let max_peers = env_u64("PEERKIT_MAX_PEERS", 10)? as usize;
    let messages_per_peer = env_u64("PEERKIT_MESSAGES_PER_PEER", 100)?;
    let message_bytes = env_u64("PEERKIT_MESSAGE_BYTES", 262_144)? as usize;

    let cycle = ctx.get().cycle;
    ctx.get_mut().cycle += 1;

    // Connect to up to `max_peers` discovered peers that are not connected.
    let candidates: Vec<PeerInfo> = list_peers(ctx)?
        .into_iter()
        .filter(|peer| peer.status == Some(PeerStatus::NotConnected))
        .take(max_peers)
        .collect();
    let mut connected = Vec::new();
    for peer in &candidates {
        match connect_to_alias(ctx, &peer.alias) {
            Ok(()) => connected.push(peer.alias.clone()),
            Err(e) => {
                log::warn!("connect to {} failed: {e:#}", peer.alias);
                report_error(ctx, "connect", 1);
            }
        }
    }

    // Record the established connection types with a single `peers` poll.
    if !connected.is_empty() {
        let peers = list_peers(ctx)?;
        for alias in &connected {
            let connection_type = match peers
                .iter()
                .find(|peer| &peer.alias == alias)
                .and_then(|peer| peer.status)
            {
                Some(PeerStatus::Direct) => "direct",
                Some(PeerStatus::Relayed) => "relayed",
                _ => "unknown",
            };
            ctx.runner_context().reporter().add_custom(
                ReportMetric::new("peerkit_connection_established")
                    .with_tag("type", connection_type)
                    .with_field("count", 1u64),
            );
        }
    }

    // Send the message batch to every connected peer.
    for alias in &connected {
        let started = Instant::now();
        for seq in 1..=messages_per_peer {
            let payload = message_payload(cycle, seq, message_bytes);
            if let Err(e) = send_text(ctx, alias, &payload) {
                log::warn!("send to {alias} failed: {e:#}");
                report_error(ctx, "send", 1);
            }
        }
        ctx.runner_context().reporter().add_custom(
            ReportMetric::new("peerkit_send_batch")
                .with_field("duration_s", started.elapsed().as_secs_f64())
                .with_field("messages", messages_per_peer)
                .with_field("bytes", messages_per_peer * message_bytes as u64),
        );
    }

    // Account for messages received from other agents' batches.
    drain_received(ctx, messages_per_peer)?;

    // Disconnect from every peer connected this cycle.
    for alias in &connected {
        if let Err(e) = disconnect_from_alias(ctx, alias) {
            log::warn!("disconnect from {alias} failed: {e:#}");
            report_error(ctx, "disconnect", 1);
        }
    }

    // Report discovery times and asynchronous send failures.
    for discovery_time_s in take_discovery_times(ctx)? {
        ctx.runner_context().reporter().add_custom(
            ReportMetric::new("peerkit_peer_discovery_time")
                .with_field("value_s", discovery_time_s),
        );
    }
    let send_failures = take_send_failures(ctx)?;
    if send_failures > 0 {
        report_error(ctx, "send_async", send_failures);
    }

    sleep_interval(ctx)
}

/// Fold newly received messages into the per-sender-batch trackers and emit a
/// `peerkit_receive_batch` metric for every batch that completed. Trackers
/// that have been in flight longer than [RECEIVE_TRACKER_TIMEOUT] are dropped
/// and counted as `receive_incomplete` errors.
fn drain_received(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    messages_per_peer: u64,
) -> anyhow::Result<()> {
    let messages = take_received_messages(ctx)?;
    let mut completed = Vec::new();
    for message in messages {
        // Header format written by `message_payload`: `<cycle>.<seq>.<fill>`.
        let Some((sender_cycle, _rest)) = message.text_prefix.split_once('.') else {
            continue;
        };
        let key = format!("{}:{sender_cycle}", message.alias);
        let tracker = ctx
            .get_mut()
            .receive_trackers
            .entry(key.clone())
            .or_insert(ReceiveTracker {
                first_at: message.received_at,
                last_at: message.received_at,
                received: 0,
                bytes: 0,
            });
        tracker.last_at = tracker.last_at.max(message.received_at);
        tracker.first_at = tracker.first_at.min(message.received_at);
        tracker.received += 1;
        tracker.bytes += message.len as u64;
        if tracker.received >= messages_per_peer {
            completed.push(key);
        }
    }
    for key in completed {
        if let Some(tracker) = ctx.get_mut().receive_trackers.remove(&key) {
            ctx.runner_context().reporter().add_custom(
                ReportMetric::new("peerkit_receive_batch")
                    .with_field(
                        "duration_s",
                        tracker
                            .last_at
                            .duration_since(tracker.first_at)
                            .as_secs_f64(),
                    )
                    .with_field("messages", tracker.received)
                    .with_field("bytes", tracker.bytes),
            );
        }
    }
    let now = Instant::now();
    let stale: Vec<String> = ctx
        .get()
        .receive_trackers
        .iter()
        .filter(|(_, tracker)| now.duration_since(tracker.first_at) > RECEIVE_TRACKER_TIMEOUT)
        .map(|(key, _)| key.clone())
        .collect();
    if !stale.is_empty() {
        report_error(ctx, "receive_incomplete", stale.len() as u64);
        for key in stale {
            ctx.get_mut().receive_trackers.remove(&key);
        }
    }
    Ok(())
}

/// Sleep between behaviour iterations. Configurable with env var
/// `PEERKIT_CYCLE_INTERVAL_MS`, defaults to 1000.
fn sleep_interval(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<()> {
    let interval_ms = env_u64("PEERKIT_CYCLE_INTERVAL_MS", 1000)?;
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
    .add_capture_env("PEERKIT_MAX_PEERS")
    .add_capture_env("PEERKIT_MESSAGES_PER_PEER")
    .add_capture_env("PEERKIT_MESSAGE_BYTES")
    .add_capture_env("PEERKIT_CYCLE_INTERVAL_MS")
    .use_agent_setup(agent_setup)
    .use_named_agent_behaviour(NODE, node_behaviour)
    .use_agent_teardown(shutdown_node)
    .with_default_duration_s(60);
    run(builder)?;
    Ok(())
}
