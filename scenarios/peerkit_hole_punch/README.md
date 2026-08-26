## peerkit_hole_punch

### Description

This scenario exercises Peerkit's hole-punching and throughput behaviour
across an arbitrary number of nodes. Every agent runs the single named
behaviour `node`, which repeats the following cycle:

1. Discover peers via the relay and connect to up to `PEERKIT_MAX_PEERS`
   discovered peers that are not already connected.
2. Poll the peer table once and record the connection type (`direct`,
   `relayed`, or `unknown`) established with each newly connected peer.
3. Send `PEERKIT_MESSAGES_PER_PEER` messages of `PEERKIT_MESSAGE_BYTES` bytes
   each to every peer connected this cycle.
4. Fold any messages received from other agents' batches into per-sender
   trackers, emitting a metric once a batch of `PEERKIT_MESSAGES_PER_PEER`
   messages from one sender has fully arrived.
5. Disconnect from every peer connected this cycle.
6. Sleep for `PEERKIT_CYCLE_INTERVAL_MS` before starting the next cycle.

Because peer identities are random and connections are driven by discovery
rather than a predicted peer ID, any number of agents may be assigned to the
`node` behaviour.

The scenario runs for 60 s by default unless otherwise configured with option
`--duration`.

### Metrics

- `wt.instruments.operation_duration` (`operation_id=connect`) — time taken
  by each `conn` call, recorded automatically by the instrumented client.
- `wt.instruments.operation_duration` (`operation_id=disconnect`) — time
  taken by each `dsct` call, recorded automatically by the instrumented
  client.
- `peerkit_peer_discovery_time` (field `value_s`) — seconds between this
  node connecting to the relay and it discovering each peer. Because the
  measurement is relative to *this* node's own relay connection time, peers
  that start later inflate the reported value: it includes their startup
  skew, not just discovery latency.
- `peerkit_connection_established` (tag `type` = `direct` | `relayed` |
  `unknown`, field `count`) — emitted once per successful connect, after
  re-polling `peers`. The connection type is polled immediately after
  connecting, so a later DCUtR upgrade from `relayed` to `direct` is not
  re-reported.
- `peerkit_send_batch` (fields `duration_s`, `messages`, `bytes`) — emitted
  per peer per cycle after dispatching all sends to that peer. This measures
  REPL dispatch time on the sending side, not delivery — delivery time is
  captured separately by `peerkit_receive_batch` on the receiving node.
- `peerkit_receive_batch` (fields `duration_s`, `messages`, `bytes`) —
  emitted on the receiving node when the last message of a sender's batch
  arrives; `duration_s` is the time between the arrival of the first and
  last message in that batch.
- `peerkit_error_count` (tag `kind` = `connect` | `send` | `send_async` |
  `disconnect` | `receive_incomplete`, field `count`) — emitted as errors
  happen. `receive_incomplete` counts batches that were dropped because they
  did not complete within 300 s of receiving their first message.

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

In a new terminal, run the scenario against it. For a quick local run, scale
down the message batch size with `PEERKIT_MESSAGES_PER_PEER` and
`PEERKIT_MESSAGE_BYTES`:

```bash
relay_addr="<paste the printed dial address>"
peerkit_bin="$(nix develop .#peerkit -c bash -c 'command -v peerkit')"
WT_PEERKIT_PATH="$peerkit_bin" PEERKIT_MESSAGES_PER_PEER=5 PEERKIT_MESSAGE_BYTES=1024 \
  RUST_LOG=info cargo run -p peerkit_hole_punch -- \
  --relay-dial-addr "$relay_addr" \
  --agents 3 --behaviour node:3 --duration 30 --no-progress
```

### Environment variables

- `PEERKIT_MAX_PEERS` — the maximum number of not-connected discovered peers
  an agent connects to per cycle. Defaults to 10.
- `PEERKIT_MESSAGES_PER_PEER` — the number of messages sent to each peer
  connected in a cycle. Defaults to 100.
- `PEERKIT_MESSAGE_BYTES` — the size in bytes of each message payload.
  Defaults to 32768 (32 KiB). This is temporarily lower than the 256 KiB
  originally requested in [holochain/wind-tunnel#692][issue-692], because the
  `peerkit` CLI crashes under the full 256 KiB x 100 message workload; see
  [holochain/wind-tunnel#704][issue-704] for the investigation tracking
  issue. Raise this back to 262144 once that issue is resolved.
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

[issue-692]: https://github.com/holochain/wind-tunnel/issues/692
[issue-704]: https://github.com/holochain/wind-tunnel/issues/704
