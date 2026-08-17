## peerkit_first_connection

### Description

This scenario exercises the simplest possible Peerkit interaction: two nodes
connect to each other through a relay and exchange text messages.

The scenario uses two named behaviours, `initiator` and `responder`, and
requires exactly one agent per behaviour. Each agent spawns its own
`peerkit node` process during agent setup. The identity of each node is
derived deterministically from the run ID and the behaviour name, so the
initiator can compute the responder's peer ID offline (no discovery or
negotiation is needed) and connect to it directly.

Once connected, the initiator sends a text message on every behaviour
iteration and the responder drains and counts the messages it has received,
recording the count as the custom metric `peerkit_messages_received`.

The interval between behaviour iterations can be configured with the env var
`PEERKIT_SEND_INTERVAL_MS` and defaults to 1000 ms.

The scenario runs for 60 s by default unless otherwise configured with option
`--duration`.

### Prerequisites

- A running Peerkit relay reachable from where the scenario is run.
- The `peerkit` CLI available either on `PATH` or via the `WT_PEERKIT_PATH`
  environment variable pointing at the binary.
- `nix develop .#peerkit` provides both Node.js and a `peerkit` wrapper
  command for local use.

### Suggested command

You can run the scenario locally with the following commands:

Start a local relay in one terminal:

```bash
nix develop .#peerkit -c peerkit relay 127.0.0.1:9910
```

It prints a dial address such as
`/ip4/127.0.0.1/udp/9910/webrtc-direct/certhash/.../p2p/...`.

In a new terminal, run the scenario against it:

```bash
relay_addr="<paste the printed dial address>"
peerkit_bin="$(nix develop .#peerkit -c bash -c 'command -v peerkit')"
WT_PEERKIT_PATH="$peerkit_bin" RUST_LOG=info cargo run -p peerkit_first_connection -- \
  --relay-dial-addr "$relay_addr" \
  --agents 2 --behaviour initiator:1 --behaviour responder:1 \
  --duration 30 --no-progress
```

### Environment variables

- `PEERKIT_SEND_INTERVAL_MS` — the delay in milliseconds between behaviour
  iterations, for both the initiator and the responder. Defaults to 1000.
- `PEERKIT_NETWORK_ACCESS` — the relay's access secret. When set, it is
  inherited automatically by every spawned `peerkit node` process and must
  match the value the relay was started with. It is never passed to
  `add_capture_env` and is never captured in run metadata, because it is a
  secret.
- `PEERKIT_RELAY_DIAL_ADDR` — fallback for the `--relay-dial-addr` flag used
  in the suggested command above; read automatically when the flag is not
  passed. Set by the Nomad job template as the relay dial address for the
  scenario task. The flag takes precedence when both are set.

### Restrictions

Peer identities are derived from the run ID and the behaviour name alone, so
at most one agent may be assigned to each of the `initiator` and `responder`
behaviours. Assigning more than one agent to either behaviour is rejected
during agent setup.
