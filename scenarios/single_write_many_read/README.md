## single_write_many_read

### Description

Creates an entry in the agent setup, then the agent behaviour is to read the record back. This tests the maximum read 
performance for reading back local data when the chain is short.

### Suggested command

Single agent

```bash
RUST_LOG=info cargo run --package single_write_many_read -- --connection-string ws://localhost:8888 --duration 300
```

Multiple agents

```bash
RUST_LOG=info cargo run --package single_write_many_read -- --connection-string ws://localhost:8888 --agents 10 --duration 300
```
