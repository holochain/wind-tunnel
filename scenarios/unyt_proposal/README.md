## Unyt Proposal

### Description

This scenario tests the performance of the Unyt negotiated transaction flow, where agents create proposals, exchange
counter-proposals, commit to agreed terms, and finalize or reject transactions. Unlike `unyt_chain_transaction` which
uses direct commitments, this scenario exercises the full proposal-based negotiation lifecycle including counter-offers,
rejections, balance reclaims, and receipt creation.

Proposals are bidirectional exchanges: the proposer offers to receive `unit 0` and send a small amount of `unit 1`.
Sending value is what requires the counterparty to create a receipt after accepting, so the scenario exercises the
complete `commit → accept → receipt` path (a proposal that only receives never produces a receipt).

There are two roles:

#### `initiate` (Progenitor Agent)

The `initiate` agent is responsible for initializing the network. This involves:

- Creating system code templates for credit limit computation and transaction fee collection
- Setting up global configuration with effective dates, credit limits, fee structures, and the two service units used by
  bidirectional proposals
- Establishing the foundational smart agreements that govern the network
- Staying idle once the network is properly initialized

#### `participate` (Participant Agents)

Each `participate` agent plays both sides of the negotiation: it originates proposals to its peers *and* responds to the
proposals and commitments it receives. Because every agent both spends and receives, value flows in both directions and
balances stay within the credit limit over a long run (a fixed proposer/responder split would drive one side's credit to
its limit). Each agent:

- Waits for and detects network initialization
- Creates bidirectional proposals to other participating agents, sized at `UNYT_SPEND_FRACTION_PCT` of its spendable
  amount so that several proposals can be in flight without exceeding the credit limit
- Handles incoming proposals by committing (only while affordable), counter-proposing, or rejecting according to
  `UNYT_PROPOSAL_WEIGHTS`
- Accepts or rejects incoming commitments according to `UNYT_COMMITMENT_ACCEPT_PCT`
- Creates receipts for accepted transactions and reclaims committed balance after rejections
- Polls transaction status and history to mirror UI behaviour

### Metrics Collected

The scenario records several custom metrics:

- `wt.custom.global_definition_propagation_time`: Records the time at which the global definition becomes readable for
  each agent, helping measure network initialization propagation speed
- `wt.custom.proposal_round_trip_time`: Measures the time from proposal creation to terminal state (accepted or
  rejected), tagged with the outcome
- `wt.custom.negotiation_rounds`: Records the number of counter-proposal rounds before a proposal reaches commitment
- `wt.custom.sync_lag`: Measures the delay (in seconds) between a transaction's publish timestamp and when it is first
  seen by the receiving agent, tagged with the transaction type (`proposal`, `commitment`, `reject`, `accept`)
- `wt.custom.ledger_balance` / `wt.custom.ledger_fees`: Captures the final ledger state at scenario teardown
- `wt.custom.actionable_transaction_{proposals,commitments,accepts,rejects}`: Records the count of each actionable
  transaction type at scenario teardown (one metric per type)
- `wt.custom.completed_transaction_{accepts,spends,raves}`: Records the count of each completed transaction type at
  scenario teardown (one metric per type)

Additionally, all zome calls are automatically logged with timing and performance metrics by the Wind Tunnel framework.

### Durable Objects store

This scenario requires all the agents to share data before it can run properly, this is achieved with a Durable Object worker from Cloudflare.
The URL and `SECRET_KEY` to access this store are retrieved from the environment variables `UNYT_DURABLE_OBJECTS_URL` and `UNYT_DURABLE_OBJECTS_SECRET`
which are required to be set for this scenario to run correctly. When running the scenario locally, a local instance of the store can be used and the
environment variables are already set in the Nix devShell (see below). When wanting to test with the official store, the `UNYT_DURABLE_OBJECTS_URL`
must be set to <https://wind-tunnel-durable-objects.holochain.org> and the `SECRET_KEY` can be found in the Holochain Foundation shared password
manager under `UNYT_DURABLE_OBJECTS_SECRET`, the `UNYT_DURABLE_OBJECTS_SECRET` environment variable must be set to that value.
When running the scenario on the Nomad clients, both of these are already stored as Nomad Variables which can be accessed by all clients.

#### Updating the `SECRET_KEY`

To update the `SECRET_KEY`, the value of `UNYT_DURABLE_OBJECTS_SECRET` in the shared password vault must be updated along with the Nomad Variable
under the same name, <https://nomad-server-01.holochain.org:4646/ui/variables/var/nomad/jobs@default>.

### Environment Variables

- `UNYT_DURABLE_OBJECTS_URL`: The URL of the instance of a Durable Objects store to use (can be local)
- `UNYT_DURABLE_OBJECTS_SECRET`: The secret required to set data in the Durable Object store
- `UNYT_PROPOSAL_WEIGHTS` (default: `60,20,20`): Comma-separated weights controlling how an agent responds to an incoming
  proposal. The three values represent the relative probability of commit, counter, and reject respectively (e.g.
  `100,0,0` for always commit, `0,0,100` for always reject)
- `UNYT_COMMITMENT_ACCEPT_PCT` (default: `80`): Percentage of incoming commitments that an agent will accept. The
  remaining commitments are rejected. For example, with a value of 80, 80% of commitments are accepted and 20% rejected
- `UNYT_COUNTER_ADJUSTMENT_PCT` (default: `10`): Percentage by which the counter-proposal amount is reduced from the
  original proposal amount. For example, if Alice proposes 100 and the adjustment is 10, Bob counters with 90
- `UNYT_MAX_NEGOTIATION_ROUNDS` (default: `5`): Maximum number of counter-proposal rounds before a proposal is
  force-accepted. Prevents infinite ping-pong when both sides have non-zero counter weights
- `UNYT_SPEND_FRACTION_PCT` (default: `10`): Percentage of an agent's spendable amount committed across each round of
  proposals. Kept well below 100 so that several proposals can be in flight at once without any single agent's
  accumulated spend reaching its credit limit

### Suggested command

You can run the scenario locally but to do this you first need to run a local
instance of the Durable Object store, do this by running the following command
from the project root directory:

```bash
nix run .#local-durable-objects
```

This will start a Durable Object store running locally in dev mode with the
port set to that of `UNYT_DURABLE_OBJECTS_URL` and the `SECRET_KEY` set to that
of `UNYT_DURABLE_OBJECTS_SECRET`.

Then, in another terminal pane, run the scenario with the following command:

```bash
RUST_LOG=warn,unyt_proposal=info cargo run --package unyt_proposal -- --agents 5 --behaviour initiate:1 --behaviour participate:4 --duration 300
```

#### Exercising rejections at small scale

The accept/counter/reject split rounds up (see `handle_proposals`), which favours accepts when the per-iteration batch
of actionable proposals is small. With the default `UNYT_PROPOSAL_WEIGHTS=60,20,20` and only a few agents, batches rarely
reach the size (5) at which the reject bucket gets any items, so rejections and reclaims are seldom exercised. At
canonical scale the batches are large enough that rejections occur naturally. To exercise the reject and reclaim paths in
a small local run, weight rejections more heavily so the reject bucket fills at batch size ≥ 2 — accept + counter must be
≤ 50%:

```bash
UNYT_PROPOSAL_WEIGHTS=30,20,50 UNYT_COMMITMENT_ACCEPT_PCT=50 \
  RUST_LOG=warn,unyt_proposal=info cargo run --package unyt_proposal -- \
  --agents 5 --behaviour initiate:1 --behaviour participate:4 --duration 300
```
