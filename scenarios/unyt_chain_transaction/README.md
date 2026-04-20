## Unyt Chain Transaction

### Description

This scenario tests the performance of a Unyt chain transaction system using a distributed credit ledger. The scenario simulates a financial network where agents can spend credits and create transaction chains that propagate through the network like unytes.

There are three roles:

#### `initiate` (Progenitor Agent)

The `initiate` agent is responsible for initializing the network. This involves:

- Creating system code templates for credit limit computation and transaction fee collection
- Setting up global configuration with effective dates, credit limits, and fee structures
- Establishing the foundational smart agreements that govern the network
- Staying idle once the network is properly initialized

#### `spend` (Transaction Agents)

The `spend` agents wait for the network to be initialized and then actively participate in the transaction system by:

- Waiting for and detecting network initialization
- Accepting incoming transactions from other agents
- Calculating spendable amounts based on current balance, fees, and applied credit limits
- Identifying other participating agents in the network
- Creating spend transactions distributed among available agents
- Continuously cycling through this process to create transaction chains

#### `smart_agreements` (Smart Agreement Agents)

The `smart_agreements` agents are responsible for creating and executing smart agreements. This involves:

- Accepting incoming transactions from other agents
- Executing smart agreements that are ready to be executed
- Calculating spendable amounts based on current balance, fees, and applied credit limits
- Creating and executing parked link spending transactions, which are a type of smart agreement, with other agents in the network.

### Metrics Collected

The scenario records several custom metrics:

- `wt.custom.global_definition_propagation_time`: Records the time at which the global definition becomes readable for each agent, helping measure network initialization propagation speed
- `wt.custom.ledger_balance`: Captures the final balance of the ledger at scenario teardown for analysis
- `wt.custom.ledger_fees`: Captures the final fees of the ledger at scenario teardown for analysis
- `wt.custom.sync_lag_commitment`: Captures the time between commitment creation and discovery by the receiving agent, tagged with `arc=zero` or `arc=full` to distinguish between zero-arc and full-arc agents
- `wt.custom.sync_lag_rave`: Captures the time between RAVE creation and discovery by smart_agreements agents, tagged with `arc=zero` or `arc=full` to distinguish between zero-arc and full-arc agents
- `wt.custom.sync_lag_grouped_parked`: Captures the time between grouped_parked request creation and discovery, tagged with `arc=zero` or `arc=full` to distinguish between zero-arc and full-arc agents
- `wt.custom.actionable_transaction_commitments`: Records the count of actionable transaction commitments at scenario teardown
- `wt.custom.completed_transaction_accepts`: Records the count of completed accept transactions at scenario teardown
- `wt.custom.completed_transaction_spends`: Records the count of completed spend transactions at scenario teardown
- `wt.custom.completed_transaction_raves`: Records the count of completed RAVE agreement executions at scenario teardown
- `wt.custom.parked_spends`: Records the count of parked spends at scenario teardown
- `wt.custom.check_agent_exists_duration_s`: Duration of the `check_agent_exists` loop per behaviour iteration
- `wt.custom.check_agent_exists_total_calls`: Total `check_agent_exists` calls made per iteration (one per participating agent)
- `wt.custom.check_agent_exists_failed_calls`: Total `check_agent_exists` calls that returned an error per iteration
- `wt.custom.check_agent_exists_missing_calls`: Total `check_agent_exists` calls that reported a missing agent per iteration
- `wt.custom.ui_action_list_refresh_duration_s`: Duration of the parallel action list refresh per behaviour iteration
- `wt.custom.ui_action_list_refresh_failed_calls`: Number of sub-calls that failed during the action list refresh per iteration
- `wt.custom.ui_routine_refresh_duration_s`: Duration of the full UI routine refresh per behaviour iteration, covering all UI-mirroring zome calls
- `wt.custom.ui_routine_refresh_watchlist_count`: Number of transactions on the watchlist at the time of each iteration
- `wt.custom.ui_transaction_detail_item_refresh_duration_s`: Duration of refreshing a single watched transaction's detail per iteration
- `wt.custom.ui_transaction_detail_item_refresh_primary_transaction_total_calls`: Primary transaction fetches made per watched item per iteration
- `wt.custom.ui_transaction_detail_item_refresh_primary_transaction_failed_calls`: Primary transaction fetches that failed per watched item per iteration
- `wt.custom.ui_transaction_detail_item_refresh_related_transaction_total_calls`: Related transaction fetches made per watched item per iteration
- `wt.custom.ui_transaction_detail_item_refresh_related_transaction_failed_calls`: Related transaction fetches that failed per watched item per iteration
- `wt.custom.ui_transaction_detail_refresh_duration_s`: Duration of refreshing all watched transaction details per behaviour iteration
- `wt.custom.ui_transaction_detail_refresh_transactions_processed`: Number of watched transactions processed in the detail refresh per iteration
- `wt.custom.ui_transaction_detail_refresh_primary_transaction_total_calls`: Total primary transaction fetches across all watched items per iteration
- `wt.custom.ui_transaction_detail_refresh_primary_transaction_failed_calls`: Total primary transaction fetches that failed across all watched items per iteration
- `wt.custom.ui_transaction_detail_refresh_related_transaction_total_calls`: Total related transaction fetches across all watched items per iteration
- `wt.custom.ui_transaction_detail_refresh_related_transaction_failed_calls`: Total related transaction fetches that failed across all watched items per iteration

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
RUST_LOG=info MIN_AGENTS=5 cargo run --package unyt_chain_transaction -- --agents 5 --behaviour initiate:1 --behaviour spend:4 --duration 300
```

```bash
RUST_LOG=wind_tunnel_unyt_scenario=warn,warn UNYT_NUMBER_OF_LINKS_TO_PROCESS=10 cargo run --package unyt_chain_transaction -- --agents 5 --behaviour initiate:1 --behaviour spend:2 --behaviour smart_agreements:2 --duration 300
```
