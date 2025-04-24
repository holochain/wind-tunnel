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

### Waiting for peer discovery

This scenario reads the environment variable `MIN_AGENTS` and waits for at least that many agents to be available before
starting the agent behaviour. It will wait up to two minutes then proceed regardless.

The scenario is not able to check that you have configured more peers than the minimum you have set, so you should
ensure that you have configured enough peers to meet the minimum.

Note that the number of agents seen by each conductor includes itself. So having two conductors means that each will
immediately see one agent after app installation.

This configuration defaults to 2 agents.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info MIN_AGENTS=5 cargo run --package two_party_countersigning -- --connection-string ws://localhost:8888 --agents 5 --behaviour initiate:2 --behaviour participate:3 --duration 300
```
