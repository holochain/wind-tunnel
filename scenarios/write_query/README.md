## write_query

### Description

Creates an entry, then queries the source chain and performs a simple operation on the entries, then repeat.

### Suggested command

```bash
RUST_LOG=info cargo run --package write_query -- --connection-string ws://localhost:8888 --duration 300
```
