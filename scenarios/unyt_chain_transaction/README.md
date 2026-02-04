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
- `wt.custom.final:ledger_state`: Captures the final state of the ledger at scenario teardown for analysis
- `wt.custom.final.history`:
  - `actionable_transactions`: Records the count of actionable invoices and spends at scenario teardown
  - `completed_transactions`: Records the count of completed transactions, including accepts and spends, at scenario teardown
  - `parked_spends`: Records the count of parked spends at scenario teardown
  - `executed_agreements`: Records the count of executed agreements at scenario teardown

Additionally, all zome calls are automatically logged with timing and performance metrics by the Wind Tunnel framework.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info MIN_AGENTS=5 cargo run --package unyt_chain_transaction -- --agents 5 --behaviour initiate:1 --behaviour spend:4 --duration 300
```

```bash
RUST_LOG=info NUMBER_OF_LINKS_TO_PROCESS=10 cargo run --package unyt_chain_transaction -- --agents 5 --behaviour initiate:1 --behaviour spend:2 --behaviour smart_agreements:2 --duration 300
```
