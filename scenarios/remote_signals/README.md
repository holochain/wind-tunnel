## remote_signals

### Description

This scenario tests the throughput of `remote_signals` operations.

Two environment variables can further control this scenario:

- `SIGNAL_INTERVAL_MS` - the interval (in ms) per node at which to publish origin signals (defaults to 1000, or 1 signal every second)
- `RESPONSE_TIMEOUT_MS` - the interval (in ms) at which we will stop waiting for a response signal and record a `remote_signal_timeout` metric (see below).

Two custom metrics are recorded:

- `wt.custom.remote_signal_round_trip`: The time in floating-point seconds from origin signal dispatch to origin receive of the remote side's response signal.
- `wt.custom.remote_signal_timeout`: A counter (value 1) recorded every time there is a timeout waiting for the response signal. (Defaults to 20 seconds--see above)

> [!WARNING]
> This is a TryCP-based scenario and needs to be run differently to other scenarios.

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
RUST_LOG=info CONDUCTOR_CONFIG="CI" TRYCP_RUST_LOG="info" MIN_PEERS=2 cargo run --package remote_signals -- --targets targets-ci.yaml --instances-per-target 2 --duration 300
```

This assumes that `trycp_server` is running. See the script `scripts/trycp.sh` and run with `start_trycp`.

To run the scenario against the current target list, you can run:

```bash
RUST_LOG=info MIN_PEERS=40 cargo run --package remote_signal_scenario -- --targets targets.yaml --duration 500
```
