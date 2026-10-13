## Unyt Proposal

### Description

This scenario tests the performance of the Unyt negotiated transaction flow, where agents create proposals, exchange
counter-proposals, commit to agreed terms, and finalize or reject transactions. It exercises the full proposal-based
negotiation lifecycle including counter-offers, rejections, balance reclaims, and receipt creation.

There are five roles. `initiate`, `full_propose`, and `full_respond` use full storage arcs;
`zero_propose` and `zero_respond` use zero storage arcs and rely on full-arc peers.
The storage arc is configured before conductor startup, and agents wait for a storage peer before authoring.

#### `initiate` (Progenitor Agent)

The `initiate` agent is responsible for initializing the network. This involves:

- Creating system code templates for credit limit computation and transaction fee collection
- Setting up global configuration with effective dates, credit limits, and fee structures
- Establishing the foundational smart agreements that govern the network
- Staying idle once the network is properly initialized

#### `full_propose` / `zero_propose` (Proposer Agents)

Both proposer roles run the same workload, differing only in their storage arc. They wait for the network to be initialized
and then actively create proposals and drive negotiations:

- Waiting for and detecting network initialization
- Creating proposals to other participating agents in the network
- Handling incoming counter-proposals by committing, countering back, or rejecting according to `UNYT_PROPOSER_WEIGHTS`
- Creating receipts for accepted transactions
- Reclaiming committed balance after rejections
- Polling transaction status and history to mirror UI behaviour

#### `full_respond` / `zero_respond` (Responder Agents)

Both responder roles run the same workload, differing only in their storage arc. They wait for the network to be initialized
and then react to incoming proposals:

- Waiting for and detecting network initialization
- Responding to incoming proposals by committing, counter-proposing, or rejecting according to `UNYT_RESPONDER_WEIGHTS`
- Accepting or rejecting incoming commitments from proposers according to `UNYT_RESPONDER_WEIGHTS`
- Creating receipts for accepted transactions
- Reclaiming committed balance after rejections
- Polling transaction status and history to mirror UI behaviour

### Metrics Collected

The scenario records several custom metrics:
- `wt.custom.agent_role`: Registered once after app installation, before authoring, with the agent public key,
  assigned `behaviour`, and `arc` (`full` or `zero`). Infrastructure agents also register.
- `wt.custom.open_connections`: Periodic connection-count samples tagged by agent and behaviour, collected without
  throttling the workload. Connection counts do not establish successful delivery, peer storage arcs, or whether
  a usable full-arc storage peer is reachable.
- `wt.custom.commitment_accepted`: Recorded only after successful `create_accept`, with the acceptor agent,
  the actual commitment creator (`author`), and commitment ID.

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

### Summary interpretation

JSON and HTML report `workloads_by_behaviour` separately for `full_propose`, `zero_propose`,
`full_respond`, and `zero_respond`. The registry supplies roles and arcs for agent-tagged custom metrics.
Each role is filtered before aggregation: proposer outcomes cannot conceal absent responder activity, and successful
full-arc outcomes cannot conceal missing zero-arc outcomes. `agent_count` counts configured registered agents;
the accepted/rejected round-trip agent counts count only agents that produced those outcomes.

Each role includes initialization propagation, accepted/rejected round-trip timing, negotiation rounds, sync lag by
transaction type, successful zome-call latency, zome errors, and completed/pending teardown counts.
Timing and teardown values are absent (`null`) when there are no matching observations, not fabricated zeros.
An observed zero negotiation-round value means direct commitment; an observed zero pending count means that
reporting agent had no actionable transactions of that type at teardown. Teardown aggregates expose the number of
reporting agents, which may be smaller than the configured population. Pending/actionable counts describe the
run cutoff; missing terminal round trips or pending work are not evidence of eventual failure or completion.

The run-wide error count includes infrastructure, while each workload error count includes only its role.
Zome-call timing excludes failed calls. Top-level `open_connections` groups samples by behaviour.
Top-level `accepted_commitments` correlates the actual commitment creator's registered arc with the acceptor's
registered arc, deduplicating successful commitment/acceptor pairs into at most four directions.
It does not infer the author from the counterparty or acceptor role, and an acceptance event does not imply that
the full proposal round trip or receipt completed before cutoff.

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

### Suggested commands

You can run the scenario locally but to do this you first need to run a local
instance of the Durable Object store, do this by running the following command
from the project root directory:

```bash
nix run .#local-durable-objects
```

This will start a Durable Object store running locally in dev mode with the
port set to that of `UNYT_DURABLE_OBJECTS_URL` and the `SECRET_KEY` set to that
of `UNYT_DURABLE_OBJECTS_SECRET`.

Then, in another terminal pane, run an all-full-arc workload:

```bash
RUST_LOG=info MIN_AGENTS=5 cargo run --package unyt_proposal -- --agents 5 --behaviour initiate:1 --behaviour full_propose:2 --behaviour full_respond:2 --duration 300
```

Or run a mixed-arc workload with one agent in each workload role:

```bash
RUST_LOG=info MIN_AGENTS=5 cargo run --package unyt_proposal -- --agents 5 --behaviour initiate:1 --behaviour full_propose:1 --behaviour zero_propose:1 --behaviour full_respond:1 --behaviour zero_respond:1 --duration 300
```
