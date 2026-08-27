use peerkit_wind_tunnel_runner::prelude::*;
use std::time::{Duration, Instant};

const NODE: &str = "node";

/// How long an in-flight receive batch may go without completing before it is
/// dropped and counted as a `receive_incomplete` error.
///
/// Must stay comfortably below realistic run durations — 60 s by default here
/// and 300 s in the Nomad jobs — otherwise no batch can ever age out during a
/// run and the metric never fires. 30 s is still generous relative to the
/// 1000 ms default cycle interval.
const RECEIVE_TRACKER_TIMEOUT: Duration = Duration::from_secs(30);

/// Number of drain attempts made after dispatching a cycle's message batches
/// and before disconnecting from the peers they were sent to.
///
/// A completed `send_text` command includes local stream backpressure, but it
/// does not prove remote application delivery. Hanging up immediately after
/// the last dispatch can still cut a batch short on the peer, leaving a
/// `peerkit_receive_batch` that never completes. Polling for a bounded grace
/// period gives the other end real wall-clock time to receive. Together with
/// [RECEIVE_GRACE_INTERVAL] this budgets 2 s, which is short enough not to
/// dominate a cycle that has just moved megabytes.
const RECEIVE_GRACE_ATTEMPTS: u32 = 10;

/// Delay between the drain attempts of the receive grace period.
const RECEIVE_GRACE_INTERVAL: Duration = Duration::from_millis(200);

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
///
/// A `size` smaller than the header cannot be honoured, because the header is
/// what lets the receiving side attribute the message to a batch. The payload
/// is then just the header and is longer than `size`; callers that report the
/// size must use the returned payload's own length.
fn message_payload(cycle: u64, seq: u64, size: usize) -> String {
    let mut payload = format!("{cycle}.{seq}.");
    let fill = size.saturating_sub(payload.len());
    payload.push_str(&"x".repeat(fill));
    payload
}

/// Derive the receive-tracker key for a message received from `alias`.
///
/// The key pairs the sender's alias with the cycle number taken from the
/// `<cycle>.<seq>.` header written by [message_payload], so concurrent batches
/// from the same peer are tracked separately. Returns `None` when the message
/// carries no such header and therefore belongs to no batch.
fn receive_tracker_key(alias: &str, text_prefix: &str) -> Option<String> {
    let (sender_cycle, _rest) = text_prefix.split_once('.')?;
    Some(format!("{alias}:{sender_cycle}"))
}

struct SendBatchResult {
    sent: u64,
    sent_bytes: u64,
    error: Option<anyhow::Error>,
    stopped_for_shutdown: bool,
}

enum SendBatchError {
    Shutdown,
    Failed(anyhow::Error),
}

fn send_peer_batch<Send>(
    cycle: u64,
    messages_per_peer: u64,
    message_bytes: usize,
    mut send: Send,
) -> SendBatchResult
where
    Send: FnMut(String) -> Result<(), SendBatchError>,
{
    let mut result = SendBatchResult {
        sent: 0,
        sent_bytes: 0,
        error: None,
        stopped_for_shutdown: false,
    };

    for seq in 1..=messages_per_peer {
        let payload = message_payload(cycle, seq, message_bytes);
        let payload_bytes = payload.len() as u64;
        match send(payload) {
            Ok(()) => {
                result.sent += 1;
                // `message_payload` cannot shrink below its header, so the
                // payload's own length is the only accurate byte count.
                result.sent_bytes += payload_bytes;
            }
            Err(SendBatchError::Shutdown) => {
                result.stopped_for_shutdown = true;
                break;
            }
            Err(SendBatchError::Failed(error)) => {
                result.error = Some(error);
                break;
            }
        }
    }

    result
}

fn agent_setup(ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>) -> HookResult {
    start_node(ctx)
}

/// Update `latched` from the shutdown signal and report whether the run is
/// stopping.
///
/// `should_shutdown` consumes the broadcast value it observes, so a second read
/// taken after the signal has already been seen wrongly reports `false`. The
/// latch is monotonic: once it holds `true` no further read is taken, and while
/// it still holds `false` every call site takes a fresh read. A single cycle can
/// run for minutes, so each check point must be able to observe a signal that
/// arrived since the previous one without racing the other check points for the
/// one value the channel carries.
fn shutting_down(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    latched: &mut bool,
) -> bool {
    *latched = *latched || ctx.shutdown_listener().should_shutdown();
    *latched
}

fn node_behaviour(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<()> {
    let max_peers = env_u64("PEERKIT_MAX_PEERS", 10)? as usize;
    let messages_per_peer = env_u64("PEERKIT_MESSAGES_PER_PEER", 100)?;
    let message_bytes = env_u64("PEERKIT_MESSAGE_BYTES", 262_144)? as usize;

    let cycle = ctx.get().cycle;
    ctx.get_mut().cycle += 1;

    // The runner already breaks its own loop when the signal arrives between
    // cycles, so this first read almost always sees `false`. It is the latch
    // that every check point below refreshes through [shutting_down] which
    // stops a cycle already in flight, otherwise a cycle that has just started
    // keeps dispatching whole message batches past the run's deadline.
    let mut shutdown_latch = ctx.shutdown_listener().should_shutdown();

    // Connect to up to `max_peers` discovered peers that are not connected.
    let candidates: Vec<PeerInfo> = list_peers(ctx)?
        .into_iter()
        .filter(|peer| peer.status == Some(PeerStatus::NotConnected))
        .take(max_peers)
        .collect();
    let mut connected = Vec::new();
    for peer in &candidates {
        // A `conn` for a departed peer blocks for up to 30 s, so re-check
        // between attempts to keep a stalled cycle interruptible.
        if shutting_down(ctx, &mut shutdown_latch) {
            break;
        }
        match connect_to_alias(ctx, &peer.alias) {
            Ok(()) => connected.push(peer.alias.clone()),
            Err(e) => {
                // The signal can land during the attempt itself, and shutdown
                // is how every run ends, so it must never be counted as a
                // Peerkit failure.
                if shutting_down(ctx, &mut shutdown_latch) {
                    break;
                }
                log::warn!("connect to {alias} failed: {e:#}", alias = peer.alias);
                report_error(ctx, "connect", 1);
            }
        }
    }

    // Record the established connection types with a single `peers` poll.
    if !connected.is_empty() && !shutting_down(ctx, &mut shutdown_latch) {
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
    let mut behavior_error = None;
    for alias in &connected {
        if shutting_down(ctx, &mut shutdown_latch) {
            break;
        }
        let started = Instant::now();
        let result = send_peer_batch(cycle, messages_per_peer, message_bytes, |payload| {
            if shutting_down(ctx, &mut shutdown_latch) {
                return Err(SendBatchError::Shutdown);
            }
            send_text(ctx, alias, &payload).map_err(SendBatchError::Failed)
        });
        if let Some(e) = result.error
            && !shutting_down(ctx, &mut shutdown_latch)
        {
            log::warn!("send to {alias} failed: {e:#}");
            report_error(ctx, "send", 1);
            behavior_error.get_or_insert(e);
        }
        // A batch cut short by shutdown is still reported, for the part of it
        // that was actually dispatched.
        ctx.runner_context().reporter().add_custom(
            ReportMetric::new("peerkit_send_batch")
                .with_field("duration_s", started.elapsed().as_secs_f64())
                .with_field("messages", result.sent)
                .with_field("bytes", result.sent_bytes),
        );
        if result.stopped_for_shutdown || shutting_down(ctx, &mut shutdown_latch) {
            break;
        }
    }

    // Account for messages received from other agents' batches. When this
    // cycle dispatched a batch of its own, hold the connections open while
    // doing so — local send completion does not prove remote application
    // delivery. See [RECEIVE_GRACE_ATTEMPTS] for why disconnecting
    // immediately is unsafe. A cycle that sent nothing has nothing to wait
    // for, so it drains once and moves on.
    let drains = if connected.is_empty() {
        1
    } else {
        RECEIVE_GRACE_ATTEMPTS
    };
    for attempt in 0..drains {
        drain_received(ctx, messages_per_peer)?;
        if attempt + 1 < drains {
            sleep(ctx, RECEIVE_GRACE_INTERVAL)?;
        }
    }

    // Disconnect from every peer connected this cycle.
    for alias in &connected {
        if shutting_down(ctx, &mut shutdown_latch) {
            break;
        }
        if let Err(e) = disconnect_from_alias(ctx, alias) {
            // A `disc` that fails because the run is ending is not a Peerkit
            // failure, so re-check before reporting it as one.
            if shutting_down(ctx, &mut shutdown_latch) {
                break;
            }
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

    if let Some(error) = behavior_error
        && !shutting_down(ctx, &mut shutdown_latch)
    {
        return Err(error);
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
        let Some(key) = receive_tracker_key(&message.alias, &message.text_prefix) else {
            continue;
        };
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

/// Sleep for `duration`, returning early with a shutdown error if the run ends
/// while waiting.
fn sleep(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    duration: Duration,
) -> anyhow::Result<()> {
    ctx.runner_context()
        .executor()
        .execute_in_place(async move {
            tokio::time::sleep(duration).await;
            Ok(())
        })
}

/// Sleep between behaviour iterations. Configurable with env var
/// `PEERKIT_CYCLE_INTERVAL_MS`, defaults to 1000.
fn sleep_interval(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<()> {
    let interval_ms = env_u64("PEERKIT_CYCLE_INTERVAL_MS", 1000)?;
    sleep(ctx, Duration::from_millis(interval_ms))
}

/// Agent teardown: flush the batches still in flight, then stop the node.
///
/// Without this, a tracker that is incomplete when the run ends is dropped
/// with no metric at all, so a truncated batch would go entirely unreported.
fn agent_teardown(ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>) -> HookResult {
    let in_flight = ctx.get().receive_trackers.len() as u64;
    if in_flight > 0 {
        ctx.get_mut().receive_trackers.clear();
        report_error(ctx, "receive_incomplete", in_flight);
    }
    shutdown_node(ctx)
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
    .use_agent_teardown(agent_teardown)
    .with_default_duration_s(60);
    run(builder)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_payload_pads_to_the_requested_size() {
        let payload = message_payload(3, 7, 32);
        assert!(payload.starts_with("3.7."));
        assert_eq!(payload.len(), 32);
        assert!(payload["3.7.".len()..].chars().all(|c| c == 'x'));
    }

    #[test]
    fn message_payload_keeps_the_whole_header_when_the_size_is_too_small() {
        let payload = message_payload(1234, 5678, 2);
        assert_eq!(payload, "1234.5678.");
        assert!(payload.len() > 2);
    }

    #[test]
    fn receive_tracker_key_pairs_the_alias_with_the_sender_cycle() {
        assert_eq!(
            receive_tracker_key("4", &message_payload(12, 99, 16)).as_deref(),
            Some("4:12")
        );
    }

    #[test]
    fn receive_tracker_key_rejects_a_message_without_a_header() {
        assert_eq!(receive_tracker_key("4", "no-header-here"), None);
    }

    #[test]
    fn send_batch_stops_after_the_first_send_error() {
        let mut attempts = Vec::new();
        let result = send_peer_batch(1, 100, 32, |payload| {
            attempts.push(payload);
            Err(SendBatchError::Failed(anyhow::anyhow!("peer exited")))
        });

        assert_eq!(attempts.len(), 1);
        assert_eq!(result.sent, 0);
        assert_eq!(result.sent_bytes, 0);
        assert!(result.error.is_some());
        assert!(!result.stopped_for_shutdown);
    }

    #[test]
    fn send_batch_reports_messages_before_a_later_send_error() {
        let mut attempts = Vec::new();
        let result = send_peer_batch(1, 100, 32, |payload| {
            attempts.push(payload);
            if attempts.len() == 1 {
                Ok(())
            } else {
                Err(SendBatchError::Failed(anyhow::anyhow!("peer exited")))
            }
        });

        assert_eq!(attempts.len(), 2);
        assert_eq!(result.sent, 1);
        assert_eq!(result.sent_bytes, 32);
        assert!(result.error.is_some());
        assert!(!result.stopped_for_shutdown);
    }

    #[test]
    fn send_batch_stops_without_an_error_when_shutdown_arrives() {
        let mut attempts = Vec::new();
        let result = send_peer_batch(1, 100, 32, |payload| {
            attempts.push(payload);
            if attempts.len() == 1 {
                Ok(())
            } else {
                Err(SendBatchError::Shutdown)
            }
        });

        assert_eq!(attempts.len(), 2);
        assert_eq!(result.sent, 1);
        assert_eq!(result.sent_bytes, 32);
        assert!(result.error.is_none());
        assert!(result.stopped_for_shutdown);
    }

    #[test]
    fn a_failed_peer_batch_does_not_prevent_the_next_peer_batch() {
        let failed = send_peer_batch(1, 100, 32, |_payload| {
            Err(SendBatchError::Failed(anyhow::anyhow!("peer exited")))
        });
        let successful = send_peer_batch(1, 2, 32, |_payload| Ok(()));

        assert!(failed.error.is_some());
        assert_eq!(successful.sent, 2);
        assert_eq!(successful.sent_bytes, 64);
    }
}
