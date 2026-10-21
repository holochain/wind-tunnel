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

Agent-tagged measurements use public agent identities, joined to `wt.custom.agent_role` rather than inferred from a
runner's local behaviour counts. The registry includes infrastructure and records `agent`, `behaviour`, and `arc`
(`full` or `zero`). Instrumented zome calls record duration and success/failure automatically.

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

JSON and HTML expose exactly two entries in `workloads_by_behaviour`: `user` and `zero_user`, even when one population
has no observations. Filtering happens before aggregation, so successful full-arc work cannot hide absent zero-arc
activity. Optional values are `null`/“no data” when no matching samples exist, not fabricated zero measurements.

#### Run-wide fields

| Field | Interpretation |
| --- | --- |
| `workloads_by_behaviour` | Independent workload groups for full-arc `user` and zero-arc `zero_user`. |
| `error_count` | Failed instrumented zome calls across the entire run, including `initiate`. |
| `open_connections` | Sampled connection-count distributions partitioned by behaviour, including infrastructure. |
| `accepted_commitments` | Unique successful commitment/acceptor pairs grouped by actual author arc and acceptor arc. Each row has `author_arc`, `acceptor_arc`, and `accepted_count`; absent directions have no observed successful accepts. This is neither proposal RTT coverage nor proof of receipt completion. |

#### Fields within each workload group

| Field | Interpretation |
| --- | --- |
| `agent_count` | Registered population, including agents without any workload outcome. |
| `proposal_round_trip_accepted_agent_count` | Distinct agents with accepted RTT observations; not the number of proposals accepted. |
| `proposal_round_trip_rejected_agent_count` | Distinct agents with rejected RTT observations; an agent can contribute to both counts. |
| `error_count` | Failed instrumented zome calls for this workload only, across all operations. |
| `global_definition_propagation_time` | Distribution of session-start-to-initialization-read times in seconds. |
| `proposal_round_trip_accepted` | Per-agent proposal-creation-to-successful-acceptance duration in seconds, excluding receipt finalization. |
| `proposal_round_trip_rejected` | Per-agent proposal-creation-to-rejection-observation duration in seconds, excluding reclaim completion. |
| `negotiation_rounds` | Distribution of observed proposal history lengths at successful commitment; unitless, despite the shared timing-statistics shape. |
| `sync_lag_proposal` | Per-agent publish-to-first-observation delay for proposals, in seconds. |
| `sync_lag_commitment` | Corresponding delay for commitments. |
| `sync_lag_accept` | Corresponding delay for accepts. |
| `sync_lag_reject` | Corresponding delay for rejects. |
| `create_proposal_zome_call` | Successful proposal creation call latency, in seconds. |
| `create_counter_proposal_zome_call` | Successful counter-proposal creation latency. |
| `create_commit_to_proposal_zome_call` | Successful commitment creation latency. |
| `create_accept_zome_call` | Successful commitment acceptance latency, not receipt completion. |
| `create_reject_proposal_zome_call` | Successful proposal/commitment rejection latency. |
| `create_receipt_for_accept_zome_call` | Successful receipt creation latency. |
| `create_reclaim_balance_zome_call` | Successful rejected-balance reclaim latency. |
| `get_actionable_transactions_zome_call` | Successful actionable-list query latency. |
| `ui_action_list_refresh_duration_s` | Seconds spent refreshing the UI action list each iteration, including failed sub-calls. |
| `ui_action_list_refresh_failed_calls` | Cumulative failed action-list sub-calls, partitioned by agent. |
| `ui_transaction_detail_item_refresh_duration_s` | Seconds spent refreshing a single watched transaction item. |
| `ui_transaction_detail_refresh_duration_s` | Seconds spent refreshing all watched transaction details in an iteration. |
| `ui_transaction_detail_refresh_transactions_processed` | Cumulative watched detail items processed, not unique transactions or finalized transactions. |
| `ui_transaction_detail_refresh_primary_transaction_total_calls` | Cumulative primary transaction fetch attempts during detail refresh. |
| `ui_transaction_detail_refresh_primary_transaction_failed_calls` | Cumulative failed primary fetches. |
| `ui_transaction_detail_refresh_related_transaction_total_calls` | Cumulative related transaction fetch attempts. |
| `ui_transaction_detail_refresh_related_transaction_failed_calls` | Cumulative failed related fetches. |
| `completed_transaction_accepts` | Accept entries reported from agent histories at teardown; not globally deduplicated transaction completions. |
| `completed_transaction_spends` | Commitment entries reported from histories (the shared source names this metric “spends”). |
| `completed_transaction_raves` | RAVE entries reported from histories at teardown. |
| `actionable_transaction_proposals` | Proposals reported actionable at teardown. |
| `actionable_transaction_commitments` | Commitments reported actionable at teardown. |
| `actionable_transaction_accepts` | Accepts reported actionable at teardown. |
| `actionable_transaction_rejects` | Rejections reported actionable at teardown. |

Successful zome latencies exclude failed calls. Standard distributions expose `mean`, `std`, and `p50`/`p95`/`p99`
(median and upper percentiles). Their `trend` contains windowed means and `window_duration`. Per-agent timing aggregates
expose `mean` (average agent mean), `mean_std_dev` (average within-agent deviation, not cross-agent spread), `min_mean`
and `max_mean` (best/worst agent means), plus chronological `trend` and `window_duration`. Agents with no samples do not
enter latency statistics. Connection gauge partitions expose a behaviour `key` and `gauge_stats`: `mean`, `std`,
`p5`/`p95` (the central 90% operating range), and windowed mean `trend`.

UI counters are cumulative running totals: counter statistics aggregate increments, not repeated totals. `total_count`
is the aggregate event count, `partition_count` is observing agents, and `partitions_above_zero` counts agents with nonzero
events. `mean_count` is the rounded average per agent; `min_per_partition`/`max_per_partition` describe the spread.
`trend` is average new events per agent per window, with `window_duration` identifying the 10-second windows; counter
resets within a window are clamped to zero. An observed zero counter is distinct from a missing counter series.

Teardown aggregates expose `sum`, `mean`, `min`, `max`, and `partition_count` (reporting agents), which can be smaller than
the registered population. Minimum/maximum are reporting-agent counts, not global completion bounds. Missing reports
are absent, not zeros. Source limitations matter: the shared teardown substitutes
empty actionable lists if their query fails and history pagination can stop on failure, yielding fallback zeros or partial
history counts. A reported zero therefore cannot by itself prove a successful read or no outstanding work. Teardown is a
cutoff view, not evidence of eventual settlement; RTT coverage likewise does not establish eventual success or failure.

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

### Captured fixture

The mixed-arc snapshot uses a 300-second run with `initiate:1`, `user:2`, and `zero_user:2`.
It was captured with an isolated Holochain 0.7.0 build containing the publication guard from
<https://github.com/holochain/holochain/pull/6032>, not the repository-pinned or merged 0.7.1 conductor.
The repository's Holochain pins are unchanged.

Both workload populations have accepted RTT observations and all nine UI metric series. The observed acceptance
directions are full → full, full → zero, and zero → full; zero → zero and rejected RTT observations are absent.
The capture includes failed zome calls and partial-history/read failures, so it is coverage evidence, not a clean-run
performance baseline or settlement proof. Host metrics are absent because no Telegraf collector was attached.
