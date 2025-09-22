## remote_signals

### Description

This scenario tests the throughput of `remote_signals` operations.

Two environment variables can further control this scenario:

- `SIGNAL_INTERVAL_MS` - the interval (in ms) per node at which to publish origin signals (defaults to 1000, or 1 signal every second)
- `RESPONSE_TIMEOUT_MS` - the interval (in ms) at which we will stop waiting for a response signal and record a `remote_signal_timeout` metric (see below).

Two custom metrics are recorded:

- `wt.custom.remote_signal_round_trip`: The time in floating-point seconds from origin signal dispatch to origin receive of the remote side's response signal.
- `wt.custom.remote_signal_timeout`: A counter (value 1) recorded every time there is a timeout waiting for the response signal. (Defaults to 20 seconds--see above)

### Waiting for peer discovery

This scenario reads the environment variable `MIN_AGENTS` and waits for at least that many agents to be available before
starting the agent behaviour. It will wait up to two minutes then proceed regardless.

The scenario is not able to check that you have configured more peers than the minimum you have set, so you should
ensure that you have configured enough peers to meet the minimum.

Note that the number of agents seen by each conductor includes itself. So having two conductors means that each will
immediately see one agent after app installation.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info MIN_AGENTS=2 cargo run -p remote_signals -- --agents 2 --duration 500
```
