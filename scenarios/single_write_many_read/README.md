## single_write_many_read

### Description

Creates an entry in the agent setup, then the agent behaviour is to read the record back. This tests the maximum read 
performance for reading back local data when the chain is short.

### Suggested command

Single agent

```bash
RUST_LOG=info cargo run --package single_write_many_read -- --duration 300
```

Multiple agents

```bash
RUST_LOG=info cargo run --package single_write_many_read -- --agents 10 --duration 300
```
