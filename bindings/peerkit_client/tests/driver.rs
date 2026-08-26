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
    "send 1 "*)
      echo "2026-08-12T10:00:05.000Z [Message from 1]: pong"
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

#[tokio::test(flavor = "multi_thread")]
async fn drives_the_repl_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let peerkit_bin = write_fake_peerkit(dir.path());

    let node = PeerkitNode::start(
        PeerkitNodeConfig {
            peerkit_bin,
            relay_dial_addrs: vec!["/ip4/127.0.0.1/udp/9000/webrtc-direct".to_string()],
            identity_path: dir.path().join("identity.key"),
        },
        test_reporter(),
    )
    .await
    .unwrap();

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
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let messages = node.take_messages().await;
            if messages
                .iter()
                .any(|message| message.alias == "1" && message.text_prefix == "pong")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();

    let peers = node.list_peers().await.unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].alias, "1");
    assert_eq!(peers[0].status, Some(PeerStatus::Direct));

    node.disconnect("1").await.unwrap();
    assert!(node.disconnect("9").await.is_err());

    let times = node.take_discovery_times().await;
    assert_eq!(times.len(), 1);

    let messages = node.take_messages().await;
    assert!(messages.iter().all(|m| m.text_prefix.len() <= 64));

    node.shutdown().await.unwrap();
}
