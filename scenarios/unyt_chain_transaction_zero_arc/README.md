## Unyt Chain Transaction Zero Arc

### Description

This scenario tests the performance of a Unyt chain transaction system where some agents operate with a **0-arc** DHT
configuration, meaning they do not store any DHT data locally and must rely on full-arc peers for data retrieval. It
builds on the same credit ledger and smart-agreement primitives as `unyt_chain_transaction`, but adds observability into
how data propagates between zero-arc and full-arc nodes.

There are five roles:

#### `initiate` (Progenitor Agent)

The `initiate` agent is responsible for initializing the network. This involves:

- Creating system code templates for credit limit computation and transaction fee collection
- Setting up global configuration with effective dates, credit limits, and fee structures
- Establishing the foundational smart agreements that govern the network
- Staying idle once the network is properly initialized

#### `zero_spend` (Zero-Arc Transaction Agents)

The `zero_spend` agents run with a 0-arc DHT configuration and actively participate in the transaction system by:

- Waiting for and detecting network initialization
- Accepting incoming commitment transactions from other agents
- Calculating spendable amounts based on current balance, fees, and applied credit limits
- Identifying other participating agents in the network
- Creating spend transactions distributed among available agents
- Continuously cycling through this process to create transaction chains

#### `zero_smart_agreements` (Zero-Arc Smart Agreement Agents)

The `zero_smart_agreements` agents run with a 0-arc DHT configuration and are responsible for creating and executing
smart agreements. This involves:

- Collecting incoming RAVE transactions from other agents
- Executing smart agreements that are ready to be executed, processing up to `UNYT_NUMBER_OF_LINKS_TO_PROCESS` links per
  agreement
- Calculating spendable amounts based on current balance, fees, and applied credit limits
- Creating and executing parked link spending transactions with other agents in the network

#### `full_observer` (Full-Arc Observer Agents)

The `full_observer` agents run with a full-arc DHT configuration and passively monitor data propagation across the
network by:

- Waiting for and detecting network initialization
- Periodically querying the code template library to discover new entries
- Measuring sync lag between when a code template was published and when it becomes visible
- Reporting the total number of discovered templates over time

#### `zero_observer` (Zero-Arc Observer Agents)

The `zero_observer` agents run with a 0-arc DHT configuration and passively monitor data propagation across the network
by:

- Waiting for and detecting network initialization
- Periodically querying the code template library to discover new entries
- Measuring sync lag between when a code template was published and when it becomes visible
- Reporting the total number of discovered templates over time

This role enables a direct comparison of data propagation times between zero-arc and full-arc nodes.

### Metrics Collected

The scenario records several custom metrics:

- `wt.custom.global_definition_propagation_time`: Records the time at which the global definition becomes readable for
  each agent, tagged with `arc=zero` or `arc=full` to distinguish between zero-arc and full-arc agents
- `wt.custom.sync_lag`: Measures the delay (in seconds) between a code template's publish timestamp and when it is
  observed, tagged with `arc=zero` or `arc=full` to compare propagation times
- `wt.custom.recv_count`: Tracks the total number of unique code templates discovered by each observer agent, tagged
  with `arc=zero` or `arc=full`
- `wt.custom.ledger_balance`: Captures the final balance of the ledger at scenario teardown for analysis
- `wt.custom.ledger_fees`: Captures the final fees of the ledger at scenario teardown for analysis
- `wt.custom.actionable_transaction_proposals`: Records the count of actionable transaction proposals at scenario teardown
- `wt.custom.actionable_transaction_commitments`: Records the count of actionable transaction commitments at scenario teardown
- `wt.custom.actionable_transaction_accepts`: Records the count of actionable transaction accepts at scenario teardown
- `wt.custom.actionable_transaction_rejects`: Records the count of actionable transaction rejects at scenario teardown
- `wt.custom.completed_transaction_accepts`: Records the count of completed accept transactions at scenario teardown
- `wt.custom.completed_transaction_spends`: Records the count of completed spend transactions at scenario teardown
- `wt.custom.completed_transaction_raves`: Records the count of completed RAVE agreement executions at scenario teardown
- `wt.custom.parked_spends`: Records the count of parked spends at scenario teardown

Additionally, all zome calls are automatically logged with timing and performance metrics by the Wind Tunnel framework.

### Durable Objects store

This scenario requires all the agents to share data before it can run properly, this is achieved with a Durable Object worker from Cloudflare.
The URL and `SECRET_KEY` to access this store are retrieved from the environment variables `UNYT_DURABLE_OBJECTS_URL` and `UNYT_DURABLE_OBJECTS_SECRET`
which are required to be set for this scenario to run correctly. When running the scenario locally, a local instance of the store can be used and the
environment variables are already set in the Nix devShell. When wanting to test with the official store, the `UNYT_DURABLE_OBJECTS_URL` must be set to
<https://wind-tunnel-durable-objects.holochain.org> and the `SECRET_KEY` can be found in the Holochain Foundation shared password manager under
`UNYT_DURABLE_OBJECTS_SECRET`, the `UNYT_DURABLE_OBJECTS_SECRET` environment variable must be set to that value. When running the scenario on the Nomad
clients, both of these are already stored as Nomad Variables which can be accessed by all clients.

#### Updating the `SECRET_KEY`

To update the `SECRET_KEY`, the value of `UNYT_DURABLE_OBJECTS_SECRET` in the shared password vault must be updated along with the Nomad Variable
under the same name, <https://nomad-server-01.holochain.org:4646/ui/variables/var/nomad/jobs@default>.

### Environment Variables

- `UNYT_NUMBER_OF_LINKS_TO_PROCESS` (default: `10`): Maximum number of parked-link transactions to process per smart
  agreement execution cycle
- `UNYT_DURABLE_OBJECTS_URL`: The URL of the instance of a Durable Objects store to use (can be local)
- `UNYT_DURABLE_OBJECTS_SECRET`: The secret required to set data in the Durable Object store
- `MIN_AGENTS`: Minimum number of agents that must join before the scenario starts

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
RUST_LOG=warn cargo run --package unyt_chain_transaction_zero_arc -- --agents 7 --behaviour initiate:1 --behaviour zero_spend:2 --behaviour zero_smart_agreements:2 --behaviour full_observer:1 --behaviour zero_observer:1 --duration 300
```
