## remote_call

### Description

This scenario tests the throughput of `remote_call` operations.

It measures two sections:
- The time between sending a remote call and the remote handler being invoked, as `wt.custom.remote_call_dispatch`
- The total elapsed time to get a response to the client, as `wt.custom.remote_call_round_trip`

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
RUST_LOG=warn MIN_AGENTS=2 cargo run --package remote_call_rate -- --agents 2 --duration 30
```
