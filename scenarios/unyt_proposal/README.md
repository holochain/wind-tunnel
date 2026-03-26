## Unyt Proposal

### Description

This scenario tests the performance of the Unyt negotiated transaction flow, where agents create proposals, exchange
counter-proposals, commit to agreed terms, and finalize or reject transactions. Unlike `unyt_chain_transaction` which
uses direct commitments, this scenario exercises the full proposal-based negotiation lifecycle including counter-offers,
rejections, balance reclaims, and receipt creation.

There are three roles:

#### `initiate` (Progenitor Agent)

The `initiate` agent is responsible for initializing the network. This involves:

- Creating system code templates for credit limit computation and transaction fee collection
- Setting up global configuration with effective dates, credit limits, and fee structures
- Establishing the foundational smart agreements that govern the network
- Staying idle once the network is properly initialized

#### `propose` (Proposer Agents)

The `propose` agents wait for the network to be initialized and then actively create proposals and drive negotiations:

- Waiting for and detecting network initialization
- Creating proposals to other participating agents in the network
- Handling incoming counter-proposals by committing, countering back, or rejecting according to `UNYT_PROPOSER_WEIGHTS`
- Creating receipts for accepted transactions
- Reclaiming committed balance after rejections
- Polling transaction status and history to mirror UI behaviour

#### `respond` (Responder Agents)

The `respond` agents wait for the network to be initialized and then react to incoming proposals:

- Waiting for and detecting network initialization
- Responding to incoming proposals by committing, counter-proposing, or rejecting according to `UNYT_RESPONDER_WEIGHTS`
- Accepting or rejecting incoming commitments from proposers according to `UNYT_RESPONDER_WEIGHTS`
- Creating receipts for accepted transactions
- Reclaiming committed balance after rejections
- Polling transaction status and history to mirror UI behaviour

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
- `UNYT_PROPOSER_WEIGHTS` (default: `60,20,20`): Comma-separated weights controlling how the proposer responds to
  incoming counter-proposals. The three values represent the relative probability of accept, counter, and reject
  respectively (e.g. `100,0,0` for always accept, `0,0,100` for always reject)
- `UNYT_RESPONDER_WEIGHTS` (default: `60,20,20`): Comma-separated weights controlling how the responder reacts to
  incoming proposals and commitments. The three values represent the relative probability of commit, counter, and reject
  respectively
- `UNYT_COMMITMENT_ACCEPT_PCT` (default: `80`): Percentage of incoming commitments that the responder will accept. The
  remaining commitments are rejected. For example, with a value of 80, 80% of commitments are accepted and 20% rejected
- `UNYT_COUNTER_ADJUSTMENT_PCT` (default: `10`): Percentage by which the counter-proposal amount is reduced from the
  original proposal amount. For example, if Alice proposes 100 and the adjustment is 10, Bob counters with 90
- `UNYT_MAX_NEGOTIATION_ROUNDS` (default: `5`): Maximum number of counter-proposal rounds before a proposal is
  force-accepted. Prevents infinite ping-pong when both sides have non-zero counter weights

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
RUST_LOG=info MIN_AGENTS=5 cargo run --package unyt_proposal -- --agents 5 --behaviour initiate:1 --behaviour propose:2 --behaviour respond:2 --duration 300
```
