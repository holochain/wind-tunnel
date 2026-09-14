use peerkit_wind_tunnel_runner::prelude::*;
use std::collections::HashMap;
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

/// Number of `peers` polls used to let a newly established connection settle
/// on a concrete direct or relayed status.
const CONNECTION_STATUS_ATTEMPTS: u32 = 5;

/// Delay between connection status polls.
const CONNECTION_STATUS_INTERVAL: Duration = Duration::from_millis(200);

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
    let (sender_cycle, _sequence) = receive_message_header(text_prefix)?;
    Some(format!("{alias}:{sender_cycle}"))
}

fn receive_message_header(text_prefix: &str) -> Option<(u64, u64)> {
    let (sender_cycle, rest) = text_prefix.split_once('.')?;
    let (sequence, _payload) = rest.split_once('.')?;
    Some((sender_cycle.parse().ok()?, sequence.parse().ok()?))
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

fn should_sleep_after_shutdown(shutdown_latched: bool) -> bool {
    !shutdown_latched
}

fn cleanup_connected_peers<Disconnect>(connected: &[String], mut disconnect: Disconnect)
where
    Disconnect: FnMut(&str) -> anyhow::Result<()>,
{
    for alias in connected {
        if let Err(error) = disconnect(alias) {
            log::debug!("failed to clean up connection to {alias}: {error:#}");
        }
    }
}

fn connection_type(status: Option<PeerStatus>) -> &'static str {
    match status {
        Some(PeerStatus::Direct) => "direct",
        Some(PeerStatus::Relayed) => "relayed",
        Some(PeerStatus::NotConnected) | None => "unknown",
    }
}

/// Poll a newly connected peer until its transport reports a concrete status,
/// or until the bounded settle window expires. A peer that loses its concrete
/// status before the window ends is not returned as sendable: retaining the
/// last direct/relayed value could make the next batch use a dead connection.
fn connection_type_for_alias(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    alias: &str,
    shutdown_latch: &mut bool,
) -> anyhow::Result<Option<&'static str>> {
    let mut concrete_status = None;
    for attempt in 0..CONNECTION_STATUS_ATTEMPTS {
        if shutting_down(ctx, shutdown_latch) {
            return Ok(None);
        }
        let peers = match list_peers(ctx) {
            Ok(peers) => peers,
            Err(_error) if shutting_down(ctx, shutdown_latch) => return Ok(None),
            Err(error) => return Err(error),
        };
        let observed_status = peers
            .iter()
            .find(|peer| peer.alias == alias)
            .and_then(|peer| peer.status);
        match observed_status {
            Some(observed_status @ (PeerStatus::Direct | PeerStatus::Relayed)) => {
                if let Some(previous_status) = concrete_status
                    && previous_status != observed_status
                {
                    ctx.runner_context().reporter().add_custom(
                        ReportMetric::new("peerkit_connection_type_transition")
                            .with_tag("from", connection_type(Some(previous_status)))
                            .with_tag("to", connection_type(Some(observed_status)))
                            .with_field("count", 1u64),
                    );
                }
                concrete_status = Some(observed_status);
                if observed_status == PeerStatus::Direct {
                    break;
                }
            }
            Some(PeerStatus::NotConnected) | None if concrete_status.is_some() => {
                report_error(ctx, "connection_lost", 1);
                return Ok(None);
            }
            Some(PeerStatus::NotConnected) | None => {}
        }
        if attempt + 1 < CONNECTION_STATUS_ATTEMPTS {
            sleep_if_running(ctx, shutdown_latch, CONNECTION_STATUS_INTERVAL)?;
        }
    }
    let Some(concrete_status) = concrete_status else {
        report_error(ctx, "connection_type_unknown", 1);
        return Ok(None);
    };
    Ok(Some(connection_type(Some(concrete_status))))
}

/// Sleep only while the run is active. A shutdown that arrives during the
/// sleep is latched and treated as a normal cycle stop rather than returned as
/// a behaviour error.
fn sleep_if_running(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    shutdown_latch: &mut bool,
    duration: Duration,
) -> anyhow::Result<()> {
    if !should_sleep_after_shutdown(shutting_down(ctx, shutdown_latch)) {
        return Ok(());
    }
    match sleep(ctx, duration) {
        Ok(()) => Ok(()),
        Err(_error) if shutting_down(ctx, shutdown_latch) => Ok(()),
        Err(error) => Err(error),
    }
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

    // Record the established connection types after a bounded settle window.
    if !connected.is_empty() && !shutting_down(ctx, &mut shutdown_latch) {
        let mut ready_connected = Vec::new();
        let mut rejected_connected = Vec::new();
        for alias in &connected {
            let connection_type = match connection_type_for_alias(ctx, alias, &mut shutdown_latch) {
                Ok(Some(connection_type)) => connection_type,
                Ok(None) if shutting_down(ctx, &mut shutdown_latch) => break,
                Ok(None) => {
                    rejected_connected.push(alias.clone());
                    continue;
                }
                Err(error) => {
                    cleanup_connected_peers(&connected, |alias| disconnect_from_alias(ctx, alias));
                    return Err(error);
                }
            };
            ctx.runner_context().reporter().add_custom(
                ReportMetric::new("peerkit_connection_established")
                    .with_tag("type", connection_type)
                    .with_field("count", 1u64),
            );
            ready_connected.push(alias.clone());
        }
        if !shutdown_latch {
            cleanup_connected_peers(&rejected_connected, |alias| {
                disconnect_from_alias(ctx, alias)
            });
        }
        connected = ready_connected;
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
        if let Err(error) = drain_received(ctx, messages_per_peer) {
            cleanup_connected_peers(&connected, |alias| disconnect_from_alias(ctx, alias));
            return Err(error);
        }
        if attempt + 1 < drains {
            if let Err(error) = sleep_if_running(ctx, &mut shutdown_latch, RECEIVE_GRACE_INTERVAL) {
                cleanup_connected_peers(&connected, |alias| disconnect_from_alias(ctx, alias));
                return Err(error);
            }
            if shutdown_latch {
                break;
            }
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

    if let Err(error) = flush_pending_client_state(ctx, messages_per_peer) {
        cleanup_connected_peers(&connected, |alias| disconnect_from_alias(ctx, alias));
        return Err(error);
    }

    if let Some(error) = behavior_error
        && !shutting_down(ctx, &mut shutdown_latch)
    {
        return Err(error);
    }

    sleep_interval(ctx, &mut shutdown_latch)
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
    let completed = {
        let state = ctx.get_mut();
        fold_received_messages(
            &mut state.receive_trackers,
            &mut state.completed_receive_cycles,
            messages,
            messages_per_peer,
        )
    };
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

fn fold_received_messages(
    receive_trackers: &mut HashMap<String, ReceiveTracker>,
    completed_receive_cycles: &mut HashMap<String, u64>,
    messages: impl IntoIterator<Item = ReceivedMessage>,
    messages_per_peer: u64,
) -> Vec<String> {
    let mut completed = Vec::new();
    for message in messages {
        let Some((sender_cycle, sequence)) = receive_message_header(&message.text_prefix) else {
            continue;
        };
        let Some(key) = receive_tracker_key(&message.alias, &message.text_prefix) else {
            continue;
        };
        if messages_per_peer == 0 || sequence == 0 || sequence > messages_per_peer {
            continue;
        }
        if completed_receive_cycles
            .get(&message.alias)
            .is_some_and(|completed_cycle| sender_cycle <= *completed_cycle)
        {
            continue;
        }
        debug_assert_eq!(key, format!("{}:{sender_cycle}", message.alias));
        let tracker = receive_trackers
            .entry(key.clone())
            .or_insert(ReceiveTracker {
                first_at: message.received_at,
                last_at: message.received_at,
                received: 0,
                bytes: 0,
                sequences: Default::default(),
            });
        tracker.last_at = tracker.last_at.max(message.received_at);
        tracker.first_at = tracker.first_at.min(message.received_at);
        if tracker.sequences.insert(sequence) {
            tracker.received += 1;
            tracker.bytes += message.len as u64;
            if tracker.received == messages_per_peer {
                completed_receive_cycles
                    .entry(message.alias.clone())
                    .and_modify(|completed_cycle| {
                        *completed_cycle = (*completed_cycle).max(sender_cycle)
                    })
                    .or_insert(sender_cycle);
                completed.push(key);
            }
        }
    }
    completed
}

fn flush_pending_client_state(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    messages_per_peer: u64,
) -> anyhow::Result<()> {
    drain_received(ctx, messages_per_peer)?;

    let discovery_times = take_discovery_times(ctx)?;
    for discovery_time_s in discovery_times {
        ctx.runner_context().reporter().add_custom(
            ReportMetric::new("peerkit_peer_discovery_time")
                .with_field("value_s", discovery_time_s),
        );
    }

    let send_failures = take_send_failures(ctx)?;
    if send_failures > 0 {
        report_error(ctx, "send_async", send_failures);
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
    shutdown_latch: &mut bool,
) -> anyhow::Result<()> {
    let interval_ms = env_u64("PEERKIT_CYCLE_INTERVAL_MS", 1000)?;
    sleep_if_running(ctx, shutdown_latch, Duration::from_millis(interval_ms))
}

/// Agent teardown: stop the node, flush all state produced before its reader
/// exits, then classify batches that are still in flight.
///
/// Without this, a tracker that is incomplete when the run ends is dropped
/// with no metric at all, so a truncated batch would go entirely unreported.
fn agent_teardown(ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>) -> HookResult {
    let messages_per_peer = match env_u64("PEERKIT_MESSAGES_PER_PEER", 100) {
        Ok(messages_per_peer) => messages_per_peer,
        Err(error) => {
            log::error!("failed to read PEERKIT_MESSAGES_PER_PEER during teardown: {error:#}");
            100
        }
    };
    // Keep one Arc alive while `shutdown_node` removes the node from the
    // context. The stopped node still owns the reader state, which lets the
    // final flush include messages, discovery events and asynchronous send
    // failures emitted while the CLI was exiting.
    let node = ctx.get().node_handle();
    let shutdown_result = shutdown_node(ctx);
    if let Some(node) = node {
        ctx.get_mut().set_node(node);
    }
    let flush_result = flush_pending_client_state(ctx, messages_per_peer);
    let in_flight = ctx.get().receive_trackers.len() as u64;
    if in_flight > 0 {
        ctx.get_mut().receive_trackers.clear();
        report_error(ctx, "receive_incomplete", in_flight);
    }
    ctx.get_mut().clear_node();
    match (flush_result, shutdown_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(flush_error), Ok(())) => Err(flush_error),
        (Ok(()), Err(shutdown_error)) => Err(shutdown_error),
        (Err(flush_error), Err(shutdown_error)) => Err(anyhow::anyhow!(
            "failed to flush client state: {flush_error:#}; failed to shut down node: {shutdown_error:#}"
        )),
    }
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
    fn receive_tracker_key_rejects_a_malformed_message_header() {
        assert_eq!(receive_tracker_key("4", "cycle.not-a-sequence."), None);
        assert_eq!(receive_tracker_key("4", "cycle.2."), None);
    }

    #[test]
    fn connection_type_names_only_concrete_peer_statuses() {
        assert_eq!(connection_type(Some(PeerStatus::Direct)), "direct");
        assert_eq!(connection_type(Some(PeerStatus::Relayed)), "relayed");
        assert_eq!(connection_type(Some(PeerStatus::NotConnected)), "unknown");
        assert_eq!(connection_type(None), "unknown");
    }

    #[test]
    fn shutdown_latch_skips_follow_up_sleep() {
        assert!(!should_sleep_after_shutdown(true));
        assert!(should_sleep_after_shutdown(false));
    }

    #[test]
    fn cleanup_attempts_every_connected_peer_after_a_failure() {
        let connected = vec!["1".to_string(), "2".to_string(), "3".to_string()];
        let mut attempted = Vec::new();

        cleanup_connected_peers(&connected, |alias| {
            attempted.push(alias.to_string());
            if alias == "2" {
                Err(anyhow::anyhow!("disconnect failed"))
            } else {
                Ok(())
            }
        });

        assert_eq!(attempted, ["1", "2", "3"]);
    }

    #[test]
    fn queued_messages_are_folded_before_in_flight_trackers_are_flushed() {
        let received_at = Instant::now();
        let mut trackers = std::collections::HashMap::new();
        let messages = vec![ReceivedMessage {
            alias: "1".to_string(),
            text_prefix: message_payload(9, 1, 32),
            len: 32,
            received_at,
        }];
        let mut completed_cycles = HashMap::new();

        let completed = fold_received_messages(&mut trackers, &mut completed_cycles, messages, 2);

        assert!(completed.is_empty());
        assert_eq!(trackers.get("1:9").map(|tracker| tracker.received), Some(1));
    }

    #[test]
    fn duplicate_or_out_of_range_messages_do_not_complete_a_batch() {
        let received_at = Instant::now();
        let mut trackers = HashMap::new();
        let mut completed_cycles = HashMap::new();
        let messages = vec![
            ReceivedMessage {
                alias: "1".to_string(),
                text_prefix: message_payload(9, 1, 32),
                len: 32,
                received_at,
            },
            ReceivedMessage {
                alias: "1".to_string(),
                text_prefix: message_payload(9, 1, 32),
                len: 32,
                received_at,
            },
            ReceivedMessage {
                alias: "1".to_string(),
                text_prefix: message_payload(9, 3, 32),
                len: 32,
                received_at,
            },
        ];

        let completed = fold_received_messages(&mut trackers, &mut completed_cycles, messages, 3);

        assert!(completed.is_empty());
        assert_eq!(trackers.get("1:9").map(|tracker| tracker.received), Some(2));
        assert_eq!(trackers.get("1:9").map(|tracker| tracker.bytes), Some(64));
    }

    #[test]
    fn completed_batch_does_not_recreate_a_tracker_for_late_duplicates() {
        let received_at = Instant::now();
        let mut trackers = HashMap::new();
        let mut completed_cycles = HashMap::new();
        let messages = (1..=2)
            .map(|sequence| ReceivedMessage {
                alias: "1".to_string(),
                text_prefix: message_payload(9, sequence, 32),
                len: 32,
                received_at,
            })
            .collect::<Vec<_>>();

        let completed = fold_received_messages(&mut trackers, &mut completed_cycles, messages, 2);
        assert_eq!(completed, ["1:9"]);
        trackers.remove("1:9");

        let duplicate = fold_received_messages(
            &mut trackers,
            &mut completed_cycles,
            [ReceivedMessage {
                alias: "1".to_string(),
                text_prefix: message_payload(9, 1, 32),
                len: 32,
                received_at,
            }],
            2,
        );

        assert!(duplicate.is_empty());
        assert!(trackers.is_empty());
        assert_eq!(completed_cycles.get("1"), Some(&9));
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
