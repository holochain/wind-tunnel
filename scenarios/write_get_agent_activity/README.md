## write_get_my_agent_activity

### Description

An agent creates an entry, then gets their own agent activity with `get_agent_activity`

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info MIN_AGENTS=2 cargo run --package write_get_agent_activity -- --connection-string ws://localhost:8888 --agents 2 --behaviour write:1 --behaviour get_agent_activity:1 --duration 60
```
