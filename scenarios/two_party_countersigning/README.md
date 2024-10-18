## two_party_countersigning

### Description

This scenario tests the performance of `countersigning` operations.

There are two roles, `initiate` and the `participate`. 

The participants commit an entry to advertise that they are willing
to participate in sessions. They listen for sessions and participate in one at a time. Three metrics are recorded:
- `wt.custom.countersigning_session_accepted`: the number of sessions accepted by the participant
- `wt.custom.countersigning_session_accepted_success`: the number of sessions successfully completed by the participant
- `wt.custom.countersigning_session_accepted_duration`: the duration of the session from acceptance to completion

The initiators get a list of peers who are advertising that they are willing to participate in sessions. They then shuffle
that list and attempt to initiate with each peer in turn. Three metrics are recorded:
- `wt.custom.countersigning_session_initiated`: the number of sessions initiated by the initiator
- `wt.custom.countersigning_session_initiated_success`: the number of sessions successfully completed by the initiator
- `wt.custom.countersigning_session_initiated_duration`: the duration of a successful session from initiation to completion

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
RUST_LOG=info CONDUCTOR_CONFIG="CI" TRYCP_RUST_LOG="info" MIN_PEERS=5 cargo run --package two_party_countersigning -- --targets targets-ci.yaml --behaviour initiate:2 --behaviour participate:3 --instances-per-target 5 --duration 300
```

This assumes that `trycp_server` is running. See the script `scripts/trycp.sh` and run with `start_trycp`.

To run the scenario against the current target list, you can run:

```bash
RUST_LOG=info MIN_PEERS=40 cargo run --package two_party_countersigning -- --targets targets.yaml --behaviour initiate:1 --behaviour participate:1 --duration 500
```
