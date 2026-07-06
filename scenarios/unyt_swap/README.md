## Unyt Swap

### Description

This scenario exercises Unyt's two conversion flows. The `bridge` role runs both in sequence, and the swap flow
builds on the wHOT produced by bridging:

- **HOT → wHOT (bridging):** an external blockchain token (HOT) is bridged into Unyt's internal wrapped credit
  (wHOT). It follows the reference `blockchain_transfer` sweettest: an oracle records a proof of deposit, a bridging
  agent turns it into a RAVE, and the depositor collects the RAVE to receive wHOT. In production the oracle and
  bridging agent are the same identity, so they are combined into a single `bridge_agent` role.
- **wHOT → HF (swap):** a HoloFuel commitment flow where the bridge commits to receive HF and pay the wHOT it holds,
  a swap agent accepts, and the bridge finalises with a receipt.

The network defines two global units — `HF` (index 0, the base unit) and `wHOT` (index 1) — so either unit can be
transacted on the network's global credit limit regardless of which behaviours are running.

There are four roles:

#### `initiate` (Progenitor Agent)

Bootstraps the network and the bridging lane:

- Creates the system code templates and smart agreements, and initialises the global definition with the `HF` and
  `wHOT` units.
- Fetches the `bridge_agent` key and builds the credit-limit-adjustment and bridging smart agreements (authorising the
  bridge agent as `oracle` and `bridging_agent`), then initialises the lane.
- Stays idle once the lane exists.

#### `bridge_agent` (Oracle + Bridging Agent)

Drives HOT → wHOT deposits. Each round it discovers the participating `bridge` users and, for each one, posts a
proof-of-deposit parked link (oracle step), then executes the credit-limit-adjustment and bridging agreements to turn
that proof into a deposit RAVE that credits the user with wHOT (bridging-agent step). One deposit is processed per
call so the aggregated proof payload stays within Holochain's link tag size limit.

#### `swap_agent` (HoloFuel Counterparty)

Accepts every incoming swap commitment, paying out HF against the network's global credit limit.

#### `user` (Depositor + Swapper)

Receives bridged HOT as wHOT and swaps it for HF. Each round it polls for incoming deposit RAVEs and collects each
one (crediting its ledger with wHOT), then — if it holds any wHOT — commits that wHOT to the swap agent for HF and
finalises any accepted commitments with a receipt.

Agreement code copied from https://github.com/unytco/smart_agreement_library/blob/main/library/_lane_bridging_unyt.

### Metrics

Custom metrics emitted by the scenario:

- `bridge_parked_links_created` — proof-of-deposit parked links the bridge agent created per round (one per user)
- `deposit_raves_collected` — deposit RAVEs the bridge collected per round (completed HOT → wHOT)
- `swap_commitments_created` — swap commitments the bridge created (wHOT → HF offered)
- `commitments_accepted` — swap commitments the swap agent accepted per round
- `swap_receipts_created` — receipts the bridge created for accepted commitments (completed wHOT → HF)
- `swap_completion_duration_s` — duration it took for a complete swap operation

Per-call zome timings (`create_parked_link`, `execute_rave`, `create_parked_spend`, `get_incoming_raves`,
`create_collect_from_rave`, `create_commitment`, `create_accept`, `create_receipt_for_accept`) are also captured by
the instrumented client and summarised per agent.

### Environment variables

- `UNYT_DURABLE_OBJECTS_URL` / `UNYT_DURABLE_OBJECTS_SECRET` — Durable Object endpoint used to share the progenitor,
  bridge-agent, and swap-agent keys across agents (required)
- `MIN_AGENTS` — minimum number of agents to wait for during setup

### Running locally

```bash
# bridging only
RUST_LOG=warn,unyt_swap=info cargo run --package unyt_swap -- --agents 3 --behaviour initiate:1 --behaviour bridge_agent:1 --behaviour user:1 --duration 60

# full flow (bridging + swap)
RUST_LOG=warn,unyt_swap=info cargo run --package unyt_swap -- --agents 4 --behaviour initiate:1 --behaviour bridge_agent:1 --behaviour swap_agent:1 --behaviour user:1 --duration 60
```
