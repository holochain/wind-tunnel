use peerkit_client_instrumented::{PeerStatus, PeerkitNode, PeerkitNodeConfig};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use wind_tunnel_core::prelude::ShutdownHandle;
use wind_tunnel_instruments::{ReportConfig, Reporter};

const PEER_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

const FAKE_PEERKIT: &str = r#"#!/usr/bin/env bash
set -u
prompt() { printf 'peerkit> '; }
event() { printf '\033[1G%s\n' "$1"; prompt; }
echo ""
echo "Node session started with agent ID aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
echo "Epoch window 86400000 ms"
echo "Auto-sync off (use 'pull')"
echo "Log file at /tmp/fake-peerkit.log"
echo ""
prompt
event "2026-08-12T10:00:00.000Z [Connected to relay with ID]: 12D3KooWFake"
event "2026-08-12T10:00:01.000Z [Peer discovered]: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
while IFS= read -r line; do
  case "$line" in
    peers)
      echo "1   [direct]   0 blob(s)  bbbbbbbb…bbbb"
      echo "2   [not connected] 0 blob(s)  cccccccc…cccc"
      prompt
      ;;
    "conn 1")
      echo "Connected to 1"
      prompt
      ;;
    "conn 4")
      sleep 0.5
      echo "Connected to 4"
      prompt
      ;;
    "conn 5")
      echo "Connected to 5"
      prompt
      ;;
    "conn 2")
      echo "Connecting to 2 failed: Error: Already connected to 2"
      prompt
      ;;
    "conn 9")
      echo "Connecting to 9 failed: Error: Unknown alias: 9"
      prompt
      ;;
    "send 1 delayed")
      event "2026-08-12T10:00:05.000Z [Message from __command_read__]: send 1 delayed"
      (sleep 0.2; prompt) &
      ;;
    "send 1 cancel")
      event "2026-08-12T10:00:05.000Z [Message from __command_read__]: send 1 cancel"
      (sleep 0.2; echo "Send failed: Error: cancelled send") &
      ;;
    "send 1 "*)
      event "2026-08-12T10:00:05.000Z [Message from 1]: pong"
      prompt
      ;;
    "send 9 "*)
      echo "Send failed: Error: Unknown alias: 9"
      prompt
      (sleep 0.2; echo "Send failed: Error: asynchronous failure") &
      ;;
    "dsct 1")
      echo "Disconnected from 1"
      prompt
      ;;
    "dsct 2")
      echo "Disconnecting from 2 failed: Error: Not connected to 2"
      prompt
      ;;
    "dsct 9")
      echo "Disconnecting from 9 failed: Error: Unknown alias: 9"
      prompt
      ;;
    exit)
      event "2026-08-12T10:00:06.000Z [Message from 1]: shutdown-message"
      echo "Closing peerkit CLI"
      exit 0
      ;;
    *)
      echo "Unknown command: $line. Type help for usage."
      prompt
      ;;
  esac
done
"#;

fn test_reporter() -> Arc<Reporter> {
    let runtime = tokio::runtime::Handle::current();
    let shutdown_listener = ShutdownHandle::new().new_listener();
    Arc::new(
        ReportConfig::new("".to_string(), "".to_string())
            .enable_in_memory()
            .init_reporter(&runtime, shutdown_listener)
            .unwrap(),
    )
}

fn write_fake_peerkit(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("fake_peerkit.sh");
    std::fs::write(&path, FAKE_PEERKIT).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

async fn start_fake_peerkit(dir: &std::path::Path) -> PeerkitNode {
    let peerkit_bin = write_fake_peerkit(dir);
    PeerkitNode::start(
        PeerkitNodeConfig {
            peerkit_bin,
            relay_dial_addrs: vec!["/ip4/127.0.0.1/udp/9000/webrtc-direct".to_string()],
            identity_path: dir.join("identity.key"),
        },
        test_reporter(),
    )
    .await
    .unwrap()
}

async fn wait_for_command_read(node: &PeerkitNode, command: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let found = node.take_messages().await.into_iter().any(|message| {
                message.alias == "__command_read__" && message.text_prefix == command
            });
            if found {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn drives_the_repl_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let node = start_fake_peerkit(dir.path()).await;

    assert_eq!(node.agent_id(), "a".repeat(64));

    node.wait_for_relay(Duration::from_secs(5)).await.unwrap();
    node.wait_for_peer_discovered(PEER_B, Duration::from_secs(5))
        .await
        .unwrap();

    let alias = node
        .request_alias(PEER_B, Duration::from_secs(5))
        .await
        .unwrap();
    assert_eq!(alias, "1");

    node.connect(&alias).await.unwrap();
    node.send_text(&alias, "ping").await.unwrap();

    // The fake answers `send` with a message event before the prompt, so the
    // message is already drained once `send_text` has returned.
    let received = node
        .take_messages()
        .await
        .into_iter()
        .find(|message| message.alias == "1" && message.text_prefix == "pong")
        .expect("pong received before the send completed");
    assert!(received.text_prefix.chars().count() <= 64);
    assert_eq!(received.len, "pong".len());

    // `peers` rows are complete as soon as the command is acknowledged: no
    // settle window is needed.
    let mut peers = node.list_peers().await.unwrap();
    peers.sort_by(|a, b| a.alias.cmp(&b.alias));
    assert_eq!(peers.len(), 2);
    assert_eq!(peers[0].alias, "1");
    assert_eq!(peers[0].status, Some(PeerStatus::Direct));
    assert_eq!(peers[1].alias, "2");
    assert_eq!(peers[1].status, Some(PeerStatus::NotConnected));

    // The desired state is reached, so these are not errors.
    node.connect("2").await.unwrap();
    node.disconnect("2").await.unwrap();
    // Genuine failures still surface.
    assert!(
        node.connect("9")
            .await
            .unwrap_err()
            .to_string()
            .contains("Unknown alias: 9")
    );
    assert!(
        node.disconnect("9")
            .await
            .unwrap_err()
            .to_string()
            .contains("Unknown alias: 9")
    );

    node.disconnect("1").await.unwrap();

    let times = node.take_discovery_times().await;
    assert_eq!(times.len(), 1);

    node.shutdown().await.unwrap();
    assert!(
        node.take_messages()
            .await
            .iter()
            .any(|message| message.text_prefix == "shutdown-message")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn send_completes_only_when_the_prompt_returns() {
    let dir = tempfile::tempdir().unwrap();
    let node = Arc::new(start_fake_peerkit(dir.path()).await);

    node.send_text("1", "first").await.unwrap();

    let mut delayed_send = tokio::spawn({
        let node = Arc::clone(&node);
        async move { node.send_text("1", "delayed").await }
    });
    wait_for_command_read(&node, "send 1 delayed").await;
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut delayed_send)
            .await
            .is_err()
    );
    delayed_send.await.unwrap().unwrap();

    node.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn command_responses_cannot_be_consumed_by_a_later_command() {
    let dir = tempfile::tempdir().unwrap();
    let node = Arc::new(start_fake_peerkit(dir.path()).await);
    node.wait_for_relay(Duration::from_secs(5)).await.unwrap();
    node.wait_for_peer_discovered(PEER_B, Duration::from_secs(5))
        .await
        .unwrap();

    let first = tokio::spawn({
        let node = Arc::clone(&node);
        async move { node.connect("4").await }
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let second = tokio::spawn({
        let node = Arc::clone(&node);
        async move { node.connect("5").await }
    });

    first.await.unwrap().unwrap();
    second.await.unwrap().unwrap();
    node.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn send_failures_are_returned_and_unclaimed_ones_are_counted() {
    let dir = tempfile::tempdir().unwrap();
    let node = start_fake_peerkit(dir.path()).await;

    // The failure printed before the prompt belongs to this call.
    let error = node.send_text("9", "failure").await.unwrap_err();
    assert!(error.to_string().contains("Unknown alias: 9"));

    // The one printed later, with no command waiting, is counted instead.
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(node.take_send_failures().await, 1);
    assert_eq!(node.take_send_failures().await, 0);

    // A reported failure is recoverable: the node keeps sending afterwards.
    node.send_text("1", "after-failure").await.unwrap();

    node.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn rejects_commands_after_a_cancelled_command() {
    let dir = tempfile::tempdir().unwrap();
    let node = Arc::new(start_fake_peerkit(dir.path()).await);

    let send_task = tokio::spawn({
        let node = Arc::clone(&node);
        async move { node.send_text("1", "cancel").await }
    });
    wait_for_command_read(&node, "send 1 cancel").await;
    send_task.abort();
    assert!(send_task.await.unwrap_err().is_cancelled());

    // The late prompt from the cancelled send can no longer be attributed, so
    // every later command is refused rather than acknowledged early.
    let error = node.send_text("1", "later").await.unwrap_err();
    assert!(error.to_string().contains("unusable"));
    let error = node.list_peers().await.unwrap_err();
    assert!(error.to_string().contains("unusable"));
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(node.take_send_failures().await, 1);

    node.shutdown().await.unwrap();
}
