## Unyt Proposal

### Description

This scenario exercises Unyt proposals, counter-proposals, commitments, acceptance or rejection, receipts, and balance
reclaims. It forward-ports the unified workload from `main-0.6`: every workload agent both creates proposals and responds
to incoming transactions. This allows value to flow in both directions instead of assigning permanently spending and
receiving populations.

There are three behaviours:

- `initiate`: full-arc progenitor that initializes the global definition, credit-limit and fee agreements, and two service
  units, then stays idle. Unit `0` is the base unit; unit `1` enables bidirectional proposals and receipt creation.
- `user`: full-arc workload agent.
- `zero_user`: zero-arc workload agent, using the **same implementation** as `user` and relying on full-arc storage peers.

Storage arcs are configured before conductor startup. Shared setup installs the app, registers the public agent identity,
and waits for storage-peer readiness. Non-initiators then wait in **agent setup** for network initialization, polling every
2 seconds for up to 180 seconds and respecting shutdown. The initiator must not wait here because its behaviour creates
that initialization. Propagation timing starts when common setup starts the agent session, not at process startup.

Each workload iteration simulates the UI action-list refresh, observes new transactions, reclaims rejected balances,
creates receipts for incoming accepts, and handles proposals and commitments. Proposal handling uses one shared set of
weights for original and counter-proposals. Commitments are deferred when available credit is insufficient. Available
credit accounts for fees and already-proposed balance. An agent creates proposals using a fraction of its spendable
amount, divided among participating peers: it receives unit `0` and sends `-1` of unit `1`. It refreshes watched transaction
details, polls statuses, and fetches recent history. Transient ledger or credit-limit reads defer work to a later iteration.

### Metrics collected

Agent-tagged measurements use public agent identities. The `wt.custom.agent_role` registry includes infrastructure
and records `agent`, `behaviour`, and `arc` (`full` or `zero`). Instrumented zome calls record duration and
success/failure automatically.

- `wt.custom.open_connections`: periodic samples labelled by behaviour; connection counts do not prove transaction
  delivery, DHT integration, or authority readiness.
- `wt.custom.commitment_accepted`: emitted after successful `create_accept`, with acceptor `agent`, actual commitment
  creator `author`, and commitment ID `commitment`.
- `wt.custom.global_definition_propagation_time`: seconds since the session began until initialization is first readable.
- `wt.custom.proposal_round_trip_time`: seconds since successful proposal creation, tagged `outcome=accepted` when
  `create_accept` succeeds, or `outcome=rejected` when a rejection for that tracked proposal is observed. Acceptance
  does **not** measure receipt finalization; rejection does **not** measure successful reclaim.
- `wt.custom.negotiation_rounds`: proposal history length when commitment creation succeeds; zero means direct commitment.
- `wt.custom.sync_lag`: transaction publish timestamp to first actionable observation, tagged `tx_type=proposal`,
  `commitment`, `accept`, or `reject`. Negative lag from clock skew is clamped to zero; clocks still affect measurements.
- `wt.custom.ui_action_list_refresh_duration_s`, `wt.custom.ui_transaction_detail_item_refresh_duration_s`, and
  `wt.custom.ui_transaction_detail_refresh_duration_s`: UI-simulation durations, tagged by observing agent.
- UI refresh cumulative counters: `wt.custom.ui_action_list_refresh_failed_calls` and
  `wt.custom.ui_transaction_detail_refresh_{transactions_processed,primary_transaction_total_calls,primary_transaction_failed_calls,related_transaction_total_calls,related_transaction_failed_calls}`.
- Teardown reports `wt.custom.completed_transaction_{accepts,spends,raves}` and
  `wt.custom.actionable_transaction_{proposals,commitments,accepts,rejects}`.
- Shared teardown also records raw base-unit `wt.custom.ledger_balance`, `wt.custom.ledger_fees`, and
  `wt.custom.parked_spends`. These remain available in raw captures, not this proposal summary.

### Summary interpretation

The summariser retains the existing flat `UnytProposalSummary` schema. Timing measurements already tagged by arc
and agent keep those partitions; other fields retain their existing run-wide or per-agent aggregation.
Behaviour-grouped summaries, registered-population counts, connection distributions, and acceptance-direction
analysis are deferred to the stacked summariser follow-up.

### Durable Objects store

All agents share discovery data through a Cloudflare Durable Object worker. Set `UNYT_DURABLE_OBJECTS_URL` and
`UNYT_DURABLE_OBJECTS_SECRET`. The Nix devShell supplies local defaults. For the official store use
<https://wind-tunnel-durable-objects.holochain.org> and the secret from the Holochain Foundation shared password manager
under `UNYT_DURABLE_OBJECTS_SECRET`. Nomad clients already obtain these from Nomad Variables.

To rotate the secret, update the shared password vault and the corresponding variable at
<https://nomad-server-01.holochain.org:4646/ui/variables/var/nomad/jobs@default>.

### Environment variables

Configuration is parsed and validated before app setup. Percentages must be integers from 0 through 100.

| Variable | Default | Meaning |
| --- | --- | --- |
| `UNYT_DURABLE_OBJECTS_URL` | Environment-specific | Discovery store URL; captured in run metadata. |
| `UNYT_DURABLE_OBJECTS_SECRET` | Environment-specific | Discovery store write secret; required but deliberately not captured. |
| `UNYT_PROPOSAL_WEIGHTS` | `60,20,20` | Commit, counter, reject percentages for incoming proposals and counter-proposals, shared by both workload arcs; must sum to 100. Batches are split with upward rounding favouring commits over rejection, not independent random decisions. |
| `UNYT_COMMITMENT_ACCEPT_PCT` | `80` | Percentage of incoming commitments accepted; the rest are rejected, with upward batch rounding. |
| `UNYT_COUNTER_ADJUSTMENT_PCT` | `10` | Percentage reduction applied to each unit in a counter-proposal. |
| `UNYT_MAX_NEGOTIATION_ROUNDS` | `5` | Proposal history length at which commitment is forced regardless of weights, still subject to available credit. |
| `UNYT_SPEND_FRACTION_PCT` | `10` | Fraction of spendable amount allocated per proposal-creation round and divided across participating peers. |
| `MIN_AGENTS` | Shared setup default | Population threshold for shared discovery readiness; set to the intended run population in examples. |

Run metadata captures all the above non-secret knobs, including `MIN_AGENTS`.

### Suggested commands

From the project root, start the local Durable Objects store:

```bash
nix run .#local-durable-objects
```

In another terminal run an all-full-arc workload:

```bash
RUST_LOG=info MIN_AGENTS=5 cargo run --package unyt_proposal -- --agents 5 --behaviour initiate:1 --behaviour user:4 --duration 300
```

Or compare the same workload on both arcs:

```bash
RUST_LOG=info MIN_AGENTS=5 cargo run --package unyt_proposal -- --agents 5 --behaviour initiate:1 --behaviour user:2 --behaviour zero_user:2 --duration 300
```
