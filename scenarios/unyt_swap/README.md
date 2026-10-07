## Unyt Swap

### Description

This scenario exercises Unyt's two conversion flows. The `user` and `zero_user` roles run both in sequence, and
the swap flow builds on the wHOT produced by bridging:

- **HOT → wHOT (bridging):** an external blockchain token (HOT) is bridged into Unyt's internal wrapped credit
  (wHOT). It follows the reference `blockchain_transfer` sweettest: an oracle records a proof of deposit, a bridging
  agent turns it into a RAVE, and the depositor collects the RAVE to receive wHOT. In production the oracle and
  bridging agent are the same identity, so they are combined into a single `bridge_agent` role.
- **wHOT → HF (swap):** a HoloFuel commitment flow where the user commits to receive HF and pay the wHOT it holds,
  a swap agent accepts, and the user finalises with a receipt.

The network starts with a base unit at index 0. The bridging lane adds `wHOT` at index 1 and `HF` at index 2;
deposits credit unit 1, and swap commitments exchange units 1 and 2.

The scenario pins the Unyt v0.108.0 hApp and uses `rave_engine` 0.11 wire types. Version 0.12 requires an
`executed_timestamp` field that v0.108.0 does not return in RAVEs or transaction history.

There are five roles:

#### `initiate` (Progenitor Agent)

Bootstraps the network and the bridging lane:

- Creates the system code templates and smart agreements, and initialises the global definition with the base unit.
- Adds the `wHOT` and `HF` units when initialising the bridging lane.
- Fetches the `bridge_agent` key and builds the credit-limit-adjustment and bridging smart agreements (authorising the
  bridge agent as `oracle` and `bridging_agent`), then initialises the lane.
- Stays idle once the lane exists.

#### `bridge_agent` (Oracle + Bridging Agent)

Drives HOT → wHOT deposits. Each round it discovers participating agents and, for each one, posts a
proof-of-deposit parked link (oracle step), then executes the credit-limit-adjustment and bridging agreements to turn
that proof into a deposit RAVE that credits the user with wHOT (bridging-agent step). One deposit is processed per
call so the aggregated proof payload stays within Holochain's link tag size limit.

#### `swap_agent` (HoloFuel Counterparty)

Accepts every incoming swap commitment, paying out HF against the network's global credit limit.

#### `user` (Depositor + Swapper)

Receives bridged HOT as wHOT and swaps it for HF. Each round it polls for incoming deposit RAVEs and collects each
one (crediting its ledger with wHOT), then — if it holds any wHOT — commits that wHOT to the swap agent for HF and
finalises any accepted commitments with a receipt.

#### `zero_user` (Zero-arc Depositor + Swapper)

Runs the same deposit collection and swap flow as `user`, but with a zero-arc conductor that relies on full-arc peers.

Known limitation: zero-arc swap commitments can remain uncompleted due to an existing bug.

Agreement code copied from https://github.com/unytco/smart_agreement_library/blob/main/library/_lane_bridging_unyt.

### Metrics

Custom metrics emitted by the scenario:

- `bridge_parked_links_completed` — completed proof-of-deposit processing sequences
- `bridge_parked_links_failed` — failed proof-of-deposit processing sequences
- `deposit_raves_collected` — deposit RAVEs collected by users (completed HOT → wHOT)
- `swap_commitments_created` — swap commitments created by users (wHOT → HF offered)
- `commitments_accepted` — swap commitments accepted by the swap agent
- `swap_receipts_created` — receipts created by users for accepted commitments (completed wHOT → HF)
- `swap_completion_duration_s` — duration it took for a complete swap operation

Count metrics are cumulative per-agent counters; completion durations are individual samples.

Per-call zome timings (`create_parked_link`, `execute_rave`, `create_parked_spend`, `get_incoming_raves`,
`create_collect_from_rave`, `create_commitment`, `create_accept`, `create_receipt_for_accept`) are also captured by
the instrumented client and summarised per agent.

### Environment variables

- `UNYT_DURABLE_OBJECTS_URL` / `UNYT_DURABLE_OBJECTS_SECRET` — Durable Object endpoint used to share the progenitor,
  bridge-agent, and swap-agent keys across agents (required)
- `MIN_AGENTS` — minimum number of agents to wait for during setup

### Running locally

Both user modes require a `swap_agent`: deposit collection and swap initiation run in the same behaviour.

```bash
# In a separate terminal, start the local Durable Object service:
nix run .#local-durable-objects

# Full flow (bridging + swap):
nix develop -c env RUST_LOG=warn,unyt_swap=info MIN_AGENTS=4 cargo run --package unyt_swap -- --agents 4 --behaviour initiate:1 --behaviour bridge_agent:1 --behaviour swap_agent:1 --behaviour user:1 --duration 300 --fail-on-agent-panic --reporter influx-file

# Replace user:1 with zero_user:1 to exercise the zero-arc flow.
```
