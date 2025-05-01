## validation_receipts

### Description

Creates an entry, wait for required validation receipts, then repeat.

Records `wt.custom.validation_receipts_complete_time` which is the time taken from after the zome call that created the
data returns, to when we have enough validation receipts. This is measured to the nearest 20ms so that we don't keep the
agent too busy checking for receipts.

### Waiting for peer discovery

This scenario reads the environment variable `MIN_AGENTS` and waits for at least that many agents to be available before
starting the agent behaviour. It will wait up to two minutes then proceed regardless.

The scenario is not able to check that you have configured more agents than the minimum you have set, so you should
ensure that you have configured enough agents to meet the minimum.

Note that the number of agents seen by each node includes itself. So setting `MIN_AGENTS` to 2 means that each agent
will immediately see one agent after app installation.

You need around 10 agents, or they will never get the required number of validation receipts.

### NO_VALIDATION_COMPLETE

By default, this scenario will wait for a complete set of validation receipts before moving on to commit the next record. If you want to publish new records on every round, building up an ever-growing list of action hashes to check for validation complete, run with the `NO_VALIDATION_COMPLETE=1` environment variable.

Example:

```bash
NO_VALIDATION_COMPLETE=1 RUST_LOG=info MIN_AGENTS=10 cargo run --package validation_receipts -- --connection-string ws://localhost:8888 --agents 10 --duration 300
```

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info MIN_AGENTS=10 cargo run --package validation_receipts -- --connection-string ws://localhost:8888 --agents 10 --duration 300
```
