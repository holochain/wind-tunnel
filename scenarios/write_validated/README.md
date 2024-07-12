## write_validated

### Description

Creates an entry, updates it and links the create to the update, then repeat. Each of the three actions is validated
by the included hApp.

### Suggested command

```bash
RUST_LOG=info cargo run --package write_validated -- --connection-string ws://localhost:8888 --duration 300
```
