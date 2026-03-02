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
- Executing smart agreements that are ready to be executed, processing up to `NUMBER_OF_LINKS_TO_PROCESS` links per
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
- `wt.custom.ledger_state`: Captures the final state of the ledger at scenario teardown for analysis
- `wt.custom.actionable_transactions`: Records the count of actionable proposals, commitments, accepts, and rejects at
  scenario teardown
- `wt.custom.completed_transactions`: Records the count of completed transactions, including accepts, spends, and RAVE
  agreement executions, at scenario teardown
- `wt.custom.parked_spends`: Records the count of parked spends at scenario teardown

Additionally, all zome calls are automatically logged with timing and performance metrics by the Wind Tunnel framework.

### Environment Variables

- `NUMBER_OF_LINKS_TO_PROCESS` (default: `10`): Maximum number of parked-link transactions to process per smart
  agreement execution cycle
- `MIN_AGENTS`: Minimum number of agents that must join before the scenario starts

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info MIN_AGENTS=7 cargo run --package unyt_chain_transaction_zero_arc -- --agents 7 --behaviour initiate:1 --behaviour zero_spend:2 --behaviour zero_smart_agreements:2 --behaviour full_observer:1 --behaviour zero_observer:1 --duration 300
```
