use peerkit_client_instrumented::{PeerStatus, PeerkitNode, PeerkitNodeConfig};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use wind_tunnel_core::prelude::ShutdownHandle;
use wind_tunnel_instruments::{ReportConfig, Reporter};

const PEER_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

const FAKE_PEERKIT: &str = r#"#!/usr/bin/env bash
set -u
echo ""
echo "Node session started with agent ID aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
echo "Epoch window 86400000 ms"
echo "Auto-sync off (use 'pull')"
echo "Log file at /tmp/fake-peerkit.log"
echo ""
echo "2026-08-12T10:00:00.000Z [Connected to relay with ID]: 12D3KooWFake"
echo "2026-08-12T10:00:01.000Z [Peer discovered]: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
while IFS= read -r line; do
  case "$line" in
    peers)
      echo "1   [direct]   0 blob(s)  bbbbbbbb…bbbb"
      ;;
    "conn 1")
      echo "Connected to 1"
      ;;
    "send 1 delayed")
      echo "2026-08-12T10:00:05.000Z [Message from __command_read__]: send 1 delayed"
      (sleep 0.2; echo "Sent to 1") &
      ;;
    "send 1 cancel")
      echo "2026-08-12T10:00:05.000Z [Message from __command_read__]: send 1 cancel"
      (sleep 0.2; echo "Sent to 1") &
      ;;
    "send 1 later")
      echo "2026-08-12T10:00:05.000Z [Message from __command_read__]: send 1 later"
      (sleep 0.4; echo "Sent to 1") &
      ;;
    "send 1 "*)
      echo "2026-08-12T10:00:05.000Z [Message from 1]: pong"
      echo "Sent to 1"
      ;;
    "send 9 "*)
      (sleep 0.2; echo "Send failed: Error: Unknown alias: 9"; sleep 0.2; echo "Send failed: Error: asynchronous failure") &
      ;;
    "dsct 1")
      echo "Disconnected from 1"
      ;;
    "dsct 9")
      echo "Disconnecting from 9 failed: Error: Unknown alias: 9"
      ;;
    exit)
      exit 0
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

    // The fake replies to `send` with a message event.
    let received = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let messages = node.take_messages().await;
            if let Some(message) = messages
                .into_iter()
                .find(|message| message.alias == "1" && message.text_prefix == "pong")
            {
                return message;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();

    // `text_prefix` is capped at 64 characters, not 64 bytes.
    assert!(received.text_prefix.chars().count() <= 64);
    assert_eq!(received.len, "pong".len());

    let peers = node.list_peers().await.unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].alias, "1");
    assert_eq!(peers[0].status, Some(PeerStatus::Direct));

    node.disconnect("1").await.unwrap();
    assert!(node.disconnect("9").await.is_err());

    let times = node.take_discovery_times().await;
    assert_eq!(times.len(), 1);

    node.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn waits_for_send_completion_events() {
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

    let error = node.send_text("9", "failure").await.unwrap_err();
    assert!(error.to_string().contains("Unknown alias: 9"));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(node.take_send_failures().await, 1);
    let unusable = node.send_text("1", "after-failure").await.unwrap_err();
    assert!(unusable.to_string().contains("unusable"));

    node.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn rejects_sends_after_cancellation_before_late_completion() {
    let dir = tempfile::tempdir().unwrap();
    let node = Arc::new(start_fake_peerkit(dir.path()).await);

    let send_task = tokio::spawn({
        let node = Arc::clone(&node);
        async move { node.send_text("1", "cancel").await }
    });
    wait_for_command_read(&node, "send 1 cancel").await;
    send_task.abort();
    assert!(send_task.await.unwrap_err().is_cancelled());

    let error = node.send_text("1", "later").await.unwrap_err();
    assert!(error.to_string().contains("unusable"));

    node.shutdown().await.unwrap();
}
