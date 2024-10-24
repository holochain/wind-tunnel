## validation_receipts

### Description

Creates an entry, wait for required validation receipts, then repeat.

Records `wt.custom.validation_receipts_complete_time` which is the time taken from after the zome call that created the 
data returns, to when we have enough validation receipts. This is measured to the nearest 20ms so that we don't keep the
agent too busy checking for receipts.

**warning** This is a TryCP-based scenario and needs to be run differently to other scenarios.

### Waiting for peer discovery

This scenario reads the environment variable `MIN_PEERS` and waits for at least that many peers to be available before
starting the agent behaviour. It will wait up to two minutes then proceed regardless.

The scenario is not able to check that you have configured more peers than the minimum you have set, so you should
ensure that you have configured enough peers to meet the minimum.

Note that the number of peers seen by each node includes itself. So having two nodes means that each node will 
immediately see one peer after app installation.

You need around at least 10 peers, or the nodes will never get the required number of validation receipts.

### NO_VALIDATION_COMPLETE

By default, this scenario will wait for a complete set of validation receipts before moving on to commit the next record. If you want to publish new records on every round, building up an ever-growing list of action hashes to check for validation complete, run with the `NO_VALIDATION_COMPLETE=1` environment variable.

Example:

```bash
NO_VALIDATION_COMPLETE=1 RUST_LOG=info CONDUCTOR_CONFIG="CI" TRYCP_RUST_LOG="info" MIN_PEERS=10 cargo run --package validation_receipts -- --targets targets-ci.yaml --instances-per-target 10 --duration 300
```

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info CONDUCTOR_CONFIG="CI" TRYCP_RUST_LOG="info" MIN_PEERS=10 cargo run --package validation_receipts -- --targets targets-ci.yaml --instances-per-target 10 --duration 300
```

This assumes that `trycp_server` is running. See the script `scripts/trycp.sh` and run with `start_trycp`.

To run the scenario against the current target list, you can run:

```bash
RUST_LOG=info MIN_PEERS=40 cargo run --package validation_receipts -- --targets targets.yaml --duration 500
```
