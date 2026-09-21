## peerkit_hole_punch

### Description

This scenario measures how often Peerkit manages to hole punch a direct
connection between two nodes. Every agent runs the single named behaviour
`node`, which repeats the following cycle:

1. List the peers discovered via the relay and pick one at random among
   those that are not currently connected.
2. Connect to that peer.
3. Poll the peer table until the connection is reported as `direct`, or
   until `PEERKIT_DIRECT_UPGRADE_TIMEOUT_MS` has elapsed.
4. Record the connection type that was reached: `direct` when the hole punch
   succeeded, `relayed` when the connection was still going through the
   relay at the timeout.
5. Disconnect from the peer.
6. Sleep for `PEERKIT_CYCLE_INTERVAL_MS` before starting the next cycle.

No application message is exchanged. Establishing a connection already opens
a stream for the handshake, which is enough to exercise hole punching.

Because peer identities are random and connections are driven by discovery
rather than a predicted peer ID, any number of agents may be assigned to the
`node` behaviour.

The scenario runs for 60 s by default unless otherwise configured with option
`--duration`.

### Metrics

- `wt.custom.peerkit_peer_discovery_time` (field `value_s`) — seconds between
  this node connecting to the relay and it discovering each peer. Because the
  measurement is relative to *this* node's own relay connection time, peers
  that start later inflate the reported value: it includes their startup
  skew, not just discovery latency.
- `wt.instruments.operation_duration` (`operation_id=connect`) — connection
  time: the time taken by each `conn` call, recorded automatically by the
  instrumented client. It covers establishing the connection, not the later
  upgrade to a direct one.
- `wt.instruments.operation_duration` (`operation_id=disconnect`) — time
  taken by each `dsct` call, recorded automatically by the instrumented
  client.
- `wt.custom.peerkit_connection_established` (tag `type` = `direct` |
  `relayed`, field `count`) — emitted once per successful connect, with the
  connection type reached when the upgrade wait ended. The hole-punch success
  rate is `direct / (direct + relayed)`. Two agents may dial each other in
  the same moment; both then record the one connection they share.
- `wt.custom.peerkit_error_count` (tag `kind` = `connect` | `disconnect` |
  `connection_lost` | `connection_type_unknown` | `behaviour`, field `count`) — emitted as
  errors happen. `connection_lost` counts connections that were reported as
  relayed and then disappeared before the upgrade wait ended, which is what
  happens when the other peer hangs up first; expect some of these in small
  networks, where agents often dial each other at the same time.
  `connection_type_unknown` counts connections for which the peer table never
  reported a type before the timeout. Neither produces a
  `peerkit_connection_established` point. `connect` and `disconnect` exclude
  the CLI's `Already connected` and `Not connected` refusals, because the
  requested state is reached either way. The framework's shutdown signal at
  the end of a run is not an error and is never counted here.
  `behaviour` counts terminal errors such as a failed peer-table command and
  stops the affected agent; other agents continue running.

### Prerequisites

- A running Peerkit relay reachable from where the scenario is run.
- The `peerkit` CLI available either on `PATH` or via the `WT_PEERKIT_PATH`
  environment variable pointing at the binary.
- `@peerkit/cli` version `0.1.0-alpha.16`, the version pinned by the Nix
  shell and the Nomad job template.
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
WT_PEERKIT_PATH="$peerkit_bin" RUST_LOG=info cargo run -p peerkit_hole_punch -- \
  --relay-dial-addr "$relay_addr" \
  --agents 3 --behaviour node:3 --duration 30 --no-progress
```

### Environment variables

- `PEERKIT_DIRECT_UPGRADE_TIMEOUT_MS` — how long, in milliseconds, an agent
  waits for a new connection to be upgraded to a direct one before recording
  it as `relayed`. Defaults to 10000.
- `PEERKIT_CYCLE_INTERVAL_MS` — the delay in milliseconds between behaviour
  cycles. Defaults to 1000.
- `PEERKIT_NETWORK_ACCESS` — the relay's access secret. When set, it is
  inherited automatically by every spawned `peerkit node` process and must
  match the value the relay was started with. It is never passed to
  `add_capture_env` and is never captured in run metadata, because it is a
  secret.
- `PEERKIT_RELAY_DIAL_ADDR` — fallback for the `--relay-dial-addr` flag used
  in the suggested command above; read automatically when the flag is not
  passed. Set by the Nomad job template as the relay dial address for the
  scenario task. The flag takes precedence when both are set.
