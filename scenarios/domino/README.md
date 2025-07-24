## domino

### Description

This scenario tests the performance of a `domino` chain transaction.

There are two roles, `initiate` and `spend`.

The `initiate` agent is responsible for initializing the network. This involves creating system code templates and setting a global configuration. This agent is also known as the "progenitor".

The `spend` agents wait for the network to be initialized and then record a custom metric:

- `wt.custom.global_definition_propagation_time`: records the time at which the global definition is readable by for each agent.
  - This helps us know when each agent can start transacting
- `wt.custom.`

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info MIN_AGENTS=5 cargo run --package domino -- --connection-string ws://localhost:8888 --agents 5 --behaviour initiate:1 --behaviour spend:4 --duration 300
```
