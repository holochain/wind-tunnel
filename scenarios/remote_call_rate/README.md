## remote_call

### Description

This scenario tests the throughput of `remote_call` operations.

**warning** This is a TryCP-based scenario and needs to be run differently to other scenarios.

### Waiting for peer discovery

This scenario reads the environment variable `MIN_PEERS` and waits for at least that many peers to be available before
starting the agent behaviour. It will wait up to two minutes then proceed regardless.

The scenario is not able to check that you have configured more peers than the minimum you have set, so you should
ensure that you have configured enough peers to meet the minimum.

Note that the number of peers seen by each node includes itself. So having two nodes means that each node will 
immediately see one peer after app installation.

This configuration defaults to 2 peers.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info CONDUCTOR_CONFIG="CI" MIN_PEERS=2 cargo run --package remote_call_rate -- --targets targets-ci.yaml --instances-per-target 2 --duration 300
```

This assumes that `trycp_server` is running. See the script `scripts/trycp.sh` and run with `start_trycp`.

To run the scenario against the current target list, you can run:

```bash
RUST_LOG=info MIN_PEERS=40 cargo run --package remote_call_rate -- --targets targets.yaml --duration 500
```
