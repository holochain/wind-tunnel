## zome_call_single_value

### Description

Calls a zome function that returns a fixed value. This tests the maximum performance of zome calls when the zome
function is not calling into Holochain other otherwise doing any work.

### Suggested command

```bash
RUST_LOG=info cargo run --package zome_call_single_value -- --duration 300
```
