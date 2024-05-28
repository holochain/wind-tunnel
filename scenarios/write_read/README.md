## single_write_many_read

### Description

Creates an entry and read it back, then repeat.

### Suggested command

```bash
RUST_LOG=info cargo run --package write_read -- --connection-string ws://localhost:8888 --duration 300
```
