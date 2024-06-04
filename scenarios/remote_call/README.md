## remote_call

### Description

This scenario tests the throughput of `remote_call` operations.

**warning** This is a TryCP-based scenario and needs to be run differently to other scenarios.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info cargo run --package remote_call -- --targets targets.yaml --duration 300
```
