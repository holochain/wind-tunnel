/// Connection status of a peer as shown by the `peers` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerStatus {
    /// Direct (hole-punched or directly dialed) connection.
    Direct,
    /// Connection through the relay circuit.
    Relayed,
    /// Discovered but not currently connected.
    NotConnected,
}

/// A parsed line of `peerkit node` stdout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerkitEvent {
    /// Startup line carrying this node's full agent ID.
    SessionStarted { agent_id: String },
    /// The node holds a circuit address on the relay.
    RelayConnected { node_id: String },
    /// A peer's agent info was received; carries the full agent ID.
    PeerDiscovered { agent_id: String },
    /// A connection to a peer was established.
    PeerConnected { alias: String, agent_id: String },
    /// A peer connection was closed.
    PeerDisconnected { alias: String },
    /// A text message arrived from a connected peer.
    MessageReceived { alias: String, text: String },
    /// Response to a `conn` command.
    ConnectSucceeded { alias: String },
    /// Response to a failed `conn` command.
    ConnectFailed { alias: String, reason: String },
    /// Printed when a `send` command fails (success prints nothing).
    SendFailed { reason: String },
    /// One row of `peers` command output; the agent ID is truncated
    /// to the CLI's `first8…last4` form.
    PeersEntry {
        alias: String,
        short_agent_id: String,
        status: Option<PeerStatus>,
    },
    /// Response to a successful `dsct` command.
    DisconnectSucceeded { alias: String },
    /// Response to a failed `dsct` command.
    DisconnectFailed { alias: String, reason: String },
    /// Any other non-empty line (startup banner, help text, ...).
    Other(String),
}

/// Parse one raw stdout line into an event.
///
/// Returns `None` for empty lines and bare prompts. Lines that carry no
/// recognised event are returned as [PeerkitEvent::Other] so callers can log
/// them.
pub fn parse_line(raw: &str) -> Option<PeerkitEvent> {
    let line = clean_line(raw);
    if line.is_empty() {
        return None;
    }

    if let Some(agent_id) = line.strip_prefix("Node session started with agent ID ") {
        return Some(PeerkitEvent::SessionStarted {
            agent_id: agent_id.trim().to_string(),
        });
    }
    if let Some(alias) = line.strip_prefix("Connected to ") {
        return Some(PeerkitEvent::ConnectSucceeded {
            alias: alias.trim().to_string(),
        });
    }
    if let Some(rest) = line.strip_prefix("Connecting to ")
        && let Some((alias, reason)) = rest.split_once(" failed: ")
    {
        return Some(PeerkitEvent::ConnectFailed {
            alias: alias.to_string(),
            reason: reason.to_string(),
        });
    }
    if let Some(alias) = line.strip_prefix("Disconnected from ") {
        return Some(PeerkitEvent::DisconnectSucceeded {
            alias: alias.trim().to_string(),
        });
    }
    if let Some(rest) = line.strip_prefix("Disconnecting from ")
        && let Some((alias, reason)) = rest.split_once(" failed: ")
    {
        return Some(PeerkitEvent::DisconnectFailed {
            alias: alias.to_string(),
            reason: reason.to_string(),
        });
    }
    if let Some(reason) = line.strip_prefix("Send failed: ") {
        return Some(PeerkitEvent::SendFailed {
            reason: reason.to_string(),
        });
    }
    if let Some(event) = parse_bracketed_event(&line) {
        return Some(event);
    }
    if let Some(event) = parse_peers_entry(&line) {
        return Some(event);
    }

    Some(PeerkitEvent::Other(line))
}

/// Truncate a full agent ID to the CLI's display form: `first8…last4`.
pub fn short_agent_id(agent_id: &str) -> String {
    if agent_id.len() <= 12 {
        return agent_id.to_string();
    }
    format!("{}…{}", &agent_id[..8], &agent_id[agent_id.len() - 4..])
}

/// Async events look like `<ISO timestamp> [<tag>]: <payload>`.
fn parse_bracketed_event(line: &str) -> Option<PeerkitEvent> {
    let start = line.find('[')?;
    let close = line[start..].find("]: ")?;
    let tag = &line[start + 1..start + close];
    let payload = &line[start + close + 3..];

    match tag {
        "Connected to relay with ID" => Some(PeerkitEvent::RelayConnected {
            node_id: payload.to_string(),
        }),
        "Peer discovered" => Some(PeerkitEvent::PeerDiscovered {
            agent_id: payload.to_string(),
        }),
        "Peer connected" => {
            let (alias, agent_id) = payload.split_once(": ")?;
            Some(PeerkitEvent::PeerConnected {
                alias: alias.to_string(),
                agent_id: agent_id.to_string(),
            })
        }
        "Peer disconnected" => Some(PeerkitEvent::PeerDisconnected {
            alias: payload.to_string(),
        }),
        _ => tag
            .strip_prefix("Message from ")
            .map(|alias| PeerkitEvent::MessageReceived {
                alias: alias.to_string(),
                text: payload.to_string(),
            }),
    }
}

/// `peers` rows look like `<alias> [<status>] <n> blob(s)  <short-agent-id>`.
/// `<status>` may contain a space (`not connected`).
fn parse_peers_entry(line: &str) -> Option<PeerkitEvent> {
    let (head, short_agent_id) = line.rsplit_once(' ')?;
    if !head.contains("blob(s)") {
        return None;
    }
    let status_start = head.find('[')?;
    let status_end = head[status_start..].find(']').map(|i| status_start + i)?;
    let status = match &head[status_start + 1..status_end] {
        "direct" => Some(PeerStatus::Direct),
        "relayed" => Some(PeerStatus::Relayed),
        "not connected" => Some(PeerStatus::NotConnected),
        _ => None,
    };
    let alias = head[..status_start].trim();
    if alias.is_empty() || alias.contains(' ') {
        return None;
    }
    let looks_like_id = short_agent_id.contains('…')
        || (short_agent_id.len() <= 12
            && !short_agent_id.is_empty()
            && short_agent_id.chars().all(|c| c.is_ascii_hexdigit()));
    if !looks_like_id {
        return None;
    }
    Some(PeerkitEvent::PeersEntry {
        alias: alias.to_string(),
        short_agent_id: short_agent_id.to_string(),
        status,
    })
}

/// Remove ANSI CSI escapes, carriage returns and readline prompt fragments.
fn clean_line(raw: &str) -> String {
    let mut cleaned = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for esc in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&esc) {
                        break;
                    }
                }
            }
            continue;
        }
        if c != '\r' {
            cleaned.push(c);
        }
    }
    let mut line = cleaned.trim();
    while let Some(rest) = line.strip_prefix("peerkit>") {
        line = rest.trim_start();
    }
    line.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGENT_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn parses_session_started() {
        let line = format!("Node session started with agent ID {AGENT_A}");
        assert_eq!(
            parse_line(&line),
            Some(PeerkitEvent::SessionStarted {
                agent_id: AGENT_A.to_string()
            })
        );
    }

    #[test]
    fn parses_relay_connected() {
        let line = "2026-08-12T10:00:00.000Z [Connected to relay with ID]: 12D3KooWabc";
        assert_eq!(
            parse_line(line),
            Some(PeerkitEvent::RelayConnected {
                node_id: "12D3KooWabc".to_string()
            })
        );
    }

    #[test]
    fn parses_peer_discovered() {
        let line = format!("2026-08-12T10:00:01.000Z [Peer discovered]: {AGENT_A}");
        assert_eq!(
            parse_line(&line),
            Some(PeerkitEvent::PeerDiscovered {
                agent_id: AGENT_A.to_string()
            })
        );
    }

    #[test]
    fn parses_peer_connected() {
        let line = format!("2026-08-12T10:00:02.000Z [Peer connected]: 1: {AGENT_A}");
        assert_eq!(
            parse_line(&line),
            Some(PeerkitEvent::PeerConnected {
                alias: "1".to_string(),
                agent_id: AGENT_A.to_string()
            })
        );
    }

    #[test]
    fn parses_message_received() {
        let line = "2026-08-12T10:00:04.000Z [Message from 1]: ping pong";
        assert_eq!(
            parse_line(line),
            Some(PeerkitEvent::MessageReceived {
                alias: "1".to_string(),
                text: "ping pong".to_string()
            })
        );
    }

    #[test]
    fn parses_connect_results() {
        assert_eq!(
            parse_line("Connected to 1"),
            Some(PeerkitEvent::ConnectSucceeded {
                alias: "1".to_string()
            })
        );
        assert_eq!(
            parse_line("Connecting to 1 failed: Error: dial failure"),
            Some(PeerkitEvent::ConnectFailed {
                alias: "1".to_string(),
                reason: "Error: dial failure".to_string()
            })
        );
        assert_eq!(
            parse_line("Send failed: Error: Unknown alias: 9"),
            Some(PeerkitEvent::SendFailed {
                reason: "Error: Unknown alias: 9".to_string()
            })
        );
    }

    #[test]
    fn parses_peers_entry_with_multi_word_status() {
        let line = "1   [not connected] 0 blob(s)  aaaaaaaa…aaaa";
        assert_eq!(
            parse_line(line),
            Some(PeerkitEvent::PeersEntry {
                alias: "1".to_string(),
                short_agent_id: "aaaaaaaa…aaaa".to_string(),
                status: Some(PeerStatus::NotConnected),
            })
        );
    }

    #[test]
    fn parses_peers_entry_status() {
        let cases = [
            (
                "1   [direct]   0 blob(s)  aaaaaaaa…aaaa",
                Some(PeerStatus::Direct),
            ),
            (
                "1   [relayed]  0 blob(s)  aaaaaaaa…aaaa",
                Some(PeerStatus::Relayed),
            ),
            (
                "1   [not connected] 0 blob(s)  aaaaaaaa…aaaa",
                Some(PeerStatus::NotConnected),
            ),
        ];
        for (line, status) in cases {
            assert_eq!(
                parse_line(line),
                Some(PeerkitEvent::PeersEntry {
                    alias: "1".to_string(),
                    short_agent_id: "aaaaaaaa…aaaa".to_string(),
                    status,
                }),
                "line: {line}"
            );
        }
    }

    #[test]
    fn parses_disconnect_results() {
        assert_eq!(
            parse_line("Disconnected from 1"),
            Some(PeerkitEvent::DisconnectSucceeded {
                alias: "1".to_string()
            })
        );
        assert_eq!(
            parse_line("Disconnecting from 1 failed: Error: Not connected to 1"),
            Some(PeerkitEvent::DisconnectFailed {
                alias: "1".to_string(),
                reason: "Error: Not connected to 1".to_string()
            })
        );
    }

    #[test]
    fn strips_prompt_carriage_returns_and_ansi() {
        let line =
            format!("peerkit> \r\u{1b}[2K2026-08-12T10:00:01.000Z [Peer discovered]: {AGENT_A}");
        assert_eq!(
            parse_line(&line),
            Some(PeerkitEvent::PeerDiscovered {
                agent_id: AGENT_A.to_string()
            })
        );
    }

    #[test]
    fn ignores_noise() {
        assert_eq!(parse_line(""), None);
        assert_eq!(parse_line("peerkit> "), None);
        assert!(matches!(
            parse_line("Epoch window 86400000 ms"),
            Some(PeerkitEvent::Other(_))
        ));
    }

    #[test]
    fn short_agent_id_matches_cli_format() {
        assert_eq!(short_agent_id(AGENT_A), "aaaaaaaa…aaaa");
        assert_eq!(short_agent_id("abcdef"), "abcdef");
    }
}
