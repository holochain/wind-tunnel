use peerkit_wind_tunnel_runner::prelude::*;
use rand::seq::IndexedRandom;
use std::time::{Duration, Instant};
use wind_tunnel_core::prelude::{AgentBailError, ShutdownSignalError};

const NODE: &str = "node";

/// Delay between `peers` polls while waiting for a new connection to be
/// upgraded to a direct one. The CLI prints no event for the upgrade, so
/// polling the peer table is the only way to observe it.
const UPGRADE_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Delay between `peers` polls while waiting for a dialable peer.
const DISCOVERY_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// How the wait for the direct upgrade of a new connection ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionOutcome {
    /// The connection was reported as direct: the hole punch succeeded.
    Direct,
    /// The connection was still relayed when the wait timed out.
    Relayed,
    /// The connection was reported as relayed and then disappeared, for
    /// example because the peer hung up first.
    Lost,
    /// The peer was seen in the table but disappeared before ever being
    /// reported as relayed, for example because the dial failed silently.
    NeverConnected,
    /// The peer table never reported a connection type before the timeout.
    Unknown,
}

fn env_u64(name: &str, default: u64) -> anyhow::Result<u64> {
    match std::env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|e| anyhow::anyhow!("{name} must be a number: {e}")),
        Err(_) => Ok(default),
    }
}

fn report_error(ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>, kind: &str) {
    ctx.runner_context().reporter().add_custom(
        ReportMetric::new("peerkit_error_count")
            .with_tag("kind", kind.to_string())
            .with_field("count", 1u64),
    );
}

fn report_connection_established(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    connection_type: &str,
) {
    ctx.runner_context().reporter().add_custom(
        ReportMetric::new("peerkit_connection_established")
            .with_tag("type", connection_type.to_string())
            .with_field("count", 1u64),
    );
}

/// Report the peer discovery times recorded by the node since the last call.
fn report_discovery_times(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<()> {
    for discovery_time_s in take_discovery_times(ctx)? {
        ctx.runner_context().reporter().add_custom(
            ReportMetric::new("peerkit_peer_discovery_time")
                .with_field("value_s", discovery_time_s),
        );
    }
    Ok(())
}

/// Keep the discovered peers this node may dial: the ones that are not
/// connected. A peer that dialled this node first is already connected and
/// is left alone.
fn connect_candidates(peers: Vec<PeerInfo>) -> Vec<PeerInfo> {
    peers
        .into_iter()
        .filter(|peer| peer.status == Some(PeerStatus::NotConnected))
        .collect()
}

/// Decide whether one `peers` observation ends the upgrade wait.
///
/// `relayed_seen` tells whether an earlier observation of the same wait
/// already reported the connection as relayed. `ever_present` tells whether
/// `alias` has been seen in the peer table at all during the wait, relayed or
/// not. `peer_present` tells whether `alias` is still in the peer table on
/// this observation: a missing `status` while the peer is present just means
/// its connection type has not resolved yet, not that the peer disconnected.
/// Returns `None` while the wait must go on.
///
/// A peer that disappears is `Lost` if it was relayed first, `NeverConnected`
/// if it was only ever seen without a relayed status, distinguishing a peer
/// that hung up after connecting from one whose dial silently failed.
fn observe(
    relayed_seen: bool,
    ever_present: bool,
    peer_present: bool,
    status: Option<PeerStatus>,
) -> Option<ConnectionOutcome> {
    match status {
        Some(PeerStatus::Direct) => Some(ConnectionOutcome::Direct),
        Some(PeerStatus::Relayed) => None,
        Some(PeerStatus::NotConnected) if relayed_seen => Some(ConnectionOutcome::Lost),
        Some(PeerStatus::NotConnected) => None,
        None if !peer_present && relayed_seen => Some(ConnectionOutcome::Lost),
        None if !peer_present && ever_present => Some(ConnectionOutcome::NeverConnected),
        None => None,
    }
}

/// The outcome of an upgrade wait that reached its timeout.
fn timed_out(relayed_seen: bool) -> ConnectionOutcome {
    if relayed_seen {
        ConnectionOutcome::Relayed
    } else {
        ConnectionOutcome::Unknown
    }
}

/// Poll the peer table until the connection to `alias` is direct, is lost,
/// or `timeout` elapses. The table is always polled at least once.
fn wait_for_direct(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
    alias: &str,
    timeout: Duration,
) -> anyhow::Result<ConnectionOutcome> {
    let deadline = Instant::now() + timeout;
    let mut relayed_seen = false;
    let mut ever_present = false;
    loop {
        let peer = list_peers(ctx)?
            .into_iter()
            .find(|peer| peer.alias == alias);
        let peer_present = peer.is_some();
        let status = peer.and_then(|peer| peer.status);
        if let Some(outcome) = observe(relayed_seen, ever_present, peer_present, status) {
            return Ok(outcome);
        }
        relayed_seen |= status == Some(PeerStatus::Relayed);
        ever_present |= peer_present;
        if Instant::now() >= deadline {
            return Ok(timed_out(relayed_seen));
        }
        sleep(ctx, UPGRADE_POLL_INTERVAL)?;
    }
}

fn node_behaviour(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<()> {
    match node_cycle(ctx) {
        Err(error) if !error.is::<ShutdownSignalError>() => {
            log::warn!("peerkit agent cannot continue: {error:#}");
            report_error(ctx, "behaviour");
            // Recoverable connection errors are handled inside the cycle.
            // A failed REPL command leaves the client unusable, so stop this
            // agent instead of repeatedly submitting commands it will reject.
            Err(anyhow::Error::new(AgentBailError::default()).context(error))
        }
        result => result,
    }
}

fn node_cycle(
    ctx: &mut AgentContext<PeerkitRunnerContext, PeerkitAgentContext>,
) -> anyhow::Result<()> {
    let upgrade_timeout =
        Duration::from_millis(env_u64("PEERKIT_DIRECT_UPGRADE_TIMEOUT_MS", 10_000)?);

    report_discovery_times(ctx)?;

    let candidates = connect_candidates(list_peers(ctx)?);
    let Some(peer) = candidates.choose(&mut rand::rng()) else {
        // Nothing to dial yet: discovery is still in progress, or every
        // discovered peer has already dialled this node.
        return sleep(ctx, DISCOVERY_POLL_INTERVAL);
    };
    let alias = peer.alias.clone();

    if let Err(error) = connect_to_alias(ctx, &alias) {
        // The shutdown signal is how every run ends, so a `conn` it
        // interrupted is not a Peerkit failure. `should_shutdown` consumes
        // the signal, which is fine because the cycle stops right here.
        if ctx.shutdown_listener().should_shutdown() {
            return Ok(());
        }
        log::warn!("connect to {alias} failed: {error:#}");
        report_error(ctx, "connect");
        return Ok(());
    }

    let outcome = wait_for_direct(ctx, &alias, upgrade_timeout);
    // Hang up whatever the wait produced, so that a failed wait cannot leave
    // the peer connected and therefore out of the candidate pool for good.
    let disconnected = disconnect_from_alias(ctx, &alias);
    // The wait outcome is examined first: a wait interrupted by shutdown
    // leaves the client unusable, and the `dsct` failure that follows must
    // not be reported as a Peerkit error.
    match outcome? {
        ConnectionOutcome::Direct => report_connection_established(ctx, "direct"),
        ConnectionOutcome::Relayed => report_connection_established(ctx, "relayed"),
        ConnectionOutcome::Lost => report_error(ctx, "connection_lost"),
        ConnectionOutcome::NeverConnected => report_error(ctx, "never_connected"),
        ConnectionOutcome::Unknown => report_error(ctx, "connection_type_unknown"),
    }
    if let Err(error) = disconnected {
        if ctx.shutdown_listener().should_shutdown() {
            return Ok(());
        }
        log::warn!("disconnect from {alias} failed: {error:#}");
        report_error(ctx, "disconnect");
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

fn main() -> WindTunnelResult<()> {
    let builder = PeerkitScenarioDefinitionBuilder::<PeerkitRunnerContext, PeerkitAgentContext>::new_with_init(
        env!("CARGO_PKG_NAME"),
    )?
    .into_std()
    .add_capture_env("PEERKIT_DIRECT_UPGRADE_TIMEOUT_MS")
    .use_agent_setup(start_node)
    .use_named_agent_behaviour(NODE, node_behaviour)
    .use_agent_teardown(|ctx| {
        // A node discovered during the final cycle has no later cycle to
        // report its discovery time, so flush it here before shutdown.
        report_discovery_times(ctx)?;
        shutdown_node(ctx)
    })
    .with_default_duration_s(60);
    run(builder)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(alias: &str, status: Option<PeerStatus>) -> PeerInfo {
        PeerInfo {
            alias: alias.to_string(),
            short_agent_id: "aaaaaaaa…aaaa".to_string(),
            status,
        }
    }

    #[test]
    fn only_not_connected_peers_are_connect_candidates() {
        let peers = vec![
            peer("1", Some(PeerStatus::Direct)),
            peer("2", Some(PeerStatus::NotConnected)),
            peer("3", Some(PeerStatus::Relayed)),
            peer("4", None),
            peer("5", Some(PeerStatus::NotConnected)),
        ];

        let aliases: Vec<String> = connect_candidates(peers)
            .into_iter()
            .map(|peer| peer.alias)
            .collect();

        assert_eq!(aliases, ["2", "5"]);
    }

    #[test]
    fn a_direct_observation_ends_the_wait() {
        assert_eq!(
            observe(false, true, true, Some(PeerStatus::Direct)),
            Some(ConnectionOutcome::Direct)
        );
        assert_eq!(
            observe(true, true, true, Some(PeerStatus::Direct)),
            Some(ConnectionOutcome::Direct)
        );
    }

    #[test]
    fn a_relayed_observation_keeps_waiting_for_the_upgrade() {
        assert_eq!(observe(false, true, true, Some(PeerStatus::Relayed)), None);
        assert_eq!(observe(true, true, true, Some(PeerStatus::Relayed)), None);
    }

    #[test]
    fn a_connection_that_disappears_after_being_relayed_is_lost() {
        assert_eq!(
            observe(true, true, true, Some(PeerStatus::NotConnected)),
            Some(ConnectionOutcome::Lost)
        );
        assert_eq!(
            observe(true, true, false, None),
            Some(ConnectionOutcome::Lost)
        );
    }

    #[test]
    fn a_connection_that_disappears_without_ever_being_relayed_is_never_connected() {
        // The peer was seen in the table (`ever_present`) but never reached
        // a relayed status before vanishing: distinct from `Lost`, which
        // requires having been relayed first.
        assert_eq!(
            observe(false, true, false, None),
            Some(ConnectionOutcome::NeverConnected)
        );
    }

    #[test]
    fn a_connection_with_no_type_yet_keeps_waiting() {
        assert_eq!(
            observe(false, true, true, Some(PeerStatus::NotConnected)),
            None
        );
        // Never yet seen in the table at all: this may just be the peer
        // table catching up after `connect_to_alias`, so keep waiting rather
        // than reporting `NeverConnected` prematurely.
        assert_eq!(observe(false, false, false, None), None);
    }

    #[test]
    fn a_present_peer_with_unknown_status_keeps_waiting_even_after_being_relayed() {
        // The peer is still in the table (`peer_present`) but the next
        // `peers` poll has not resolved its connection type yet. This must
        // not be confused with the peer having disappeared.
        assert_eq!(observe(true, true, true, None), None);
    }

    #[test]
    fn a_timeout_reports_relayed_only_when_relayed_was_seen() {
        assert_eq!(timed_out(true), ConnectionOutcome::Relayed);
        assert_eq!(timed_out(false), ConnectionOutcome::Unknown);
    }
}
