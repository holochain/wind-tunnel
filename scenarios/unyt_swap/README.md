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

`initiate`, `bridge_agent`, and `swap_agent` always use full storage arcs. Only `zero_user` uses a zero storage
arc; it relies on full-arc peers and addresses its swap commitments to the full-arc `swap_agent`.
Zero-arc users wait for a full-arc peer before Unyt initialization and flag-template authoring.

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

#### `user` (Full-arc Depositor + Swapper)

Receives bridged HOT as wHOT and swaps it for HF. Each round it polls for incoming deposit RAVEs and collects each
one (crediting its ledger with wHOT), then — if it holds any wHOT — commits that wHOT to the swap agent for HF and
finalises any accepted commitments with a receipt.

#### `zero_user` (Zero-arc Depositor + Swapper)

Runs the same deposit collection and swap flow as `user`, but with a zero-arc conductor that relies on full-arc peers.

Known limitation: zero-arc swap commitments can remain uncompleted due to an existing bug.

Agreement code copied from https://github.com/unytco/smart_agreement_library/blob/main/library/_lane_bridging_unyt.

### Metrics

Custom metrics emitted by the scenario:

- `agent_role` — one setup sample per installed agent, with its public key, assigned behaviour, and storage arc
- `open_connections` — sampled connection counts, tagged by agent and behaviour
- `commitment_accepted` — one event after a successful accept, identifying the acceptor, actual commitment author,
  and commitment ID
- `bridge_parked_links_completed` — completed proof-of-deposit processing sequences
- `bridge_parked_links_failed` — failed proof-of-deposit processing sequences
- `deposit_raves_collected` — deposit RAVEs collected by users (completed HOT → wHOT)
- `swap_commitments_created` — swap commitments created by users (wHOT → HF offered)
- `commitments_accepted` — swap commitments accepted by the swap agent
- `swap_receipts_created` — receipts created by users for accepted commitments (completed wHOT → HF)
- `swap_completion_duration_s` — duration it took for a complete swap operation

Count metrics are cumulative per-agent counters; completion durations are individual samples.
The summary reports user metrics in `workloads_by_behaviour.user` and `workloads_by_behaviour.zero_user`,
joining agent-tagged measurements to the setup role registry. Each group includes its registered `agent_count`,
zome-call `error_count`, optional collection/commitment/receipt counters, and optional completion and zome-call timings.
Infrastructure metrics and the run-wide zome-call error count remain separate. No individual agent identities
are included in the summary or report.

Counter `partition_count` is the number of agents with observed counter samples, not the number of registered
users; `partitions_above_zero` is the number with positive outcomes. A counter with no observed agents is
**no data**, represented by `null`, not an observed zero. A sampled counter that remains zero is an observed zero. Missing completion
timings mean no completions were observed, not instantaneous completion; one arc's success never supplies
the other arc's missing timings.

Per-call zome timings (`create_parked_link`, `execute_rave`, `create_parked_spend`, `get_incoming_raves`,
`create_collect_from_rave`, `create_commitment`, `create_accept`, `create_receipt_for_accept`) are captured by
the instrumented client. Only successful calls contribute timings; failed calls contribute error counts.
User timings are aggregated per agent within each behaviour group.

`accepted_commitments` reports author-arc → acceptor-arc directions from the actual commitment creator and
successful acceptor identities. It counts unique commitment/acceptor pairs, not completed receipts:
acceptance can succeed while receipt completion remains stalled. `open_connections` reports observed
connection counts by behaviour. Open connections do not establish synchronization or application readiness.

### Environment variables

- `UNYT_DURABLE_OBJECTS_URL` / `UNYT_DURABLE_OBJECTS_SECRET` — Durable Object endpoint used to share the progenitor,
  bridge-agent, and swap-agent keys across agents (required)
- `MIN_AGENTS` — minimum number of agents to wait for during setup

### Running locally

Both user modes require a `swap_agent`: deposit collection and swap initiation run in the same behaviour.

```bash
# In a separate terminal, start the local Durable Object service:
nix run .#local-durable-objects

# All-full flow (bridging + swap):
nix develop -c env RUST_LOG=warn,unyt_swap=info MIN_AGENTS=4 cargo run --package unyt_swap -- --agents 4 --behaviour initiate:1 --behaviour bridge_agent:1 --behaviour swap_agent:1 --behaviour user:1 --duration 300 --fail-on-agent-panic --reporter influx-file

# Mixed full-arc and zero-arc users, with full-arc infrastructure:
nix develop -c env RUST_LOG=warn,unyt_swap=info MIN_AGENTS=5 cargo run --package unyt_swap -- --agents 5 --behaviour initiate:1 --behaviour bridge_agent:1 --behaviour swap_agent:1 --behaviour user:1 --behaviour zero_user:1 --duration 300 --fail-on-agent-panic --reporter influx-file
```
