## zome_call_single_value

### Description

Calls a zome function that returns a fixed value. This tests the maximum performance of zome calls when the zome
function is not calling into Holochain other otherwise doing any work.

### Suggested command

Single agent

```bash
RUST_LOG=info cargo run --package zome_call_single_value -- --connection-string ws://localhost:8888 --duration 300
```

Multiple agents

```bash
RUST_LOG=info cargo run --package zome_call_single_value -- --connection-string ws://localhost:8888 --agents 10 --duration 300
```

