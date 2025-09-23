## write_get_agent_activity

### Description

An agent creates entries, while another agent gets their agent activity with `get_agent_activity`.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info MIN_AGENTS=2 cargo run --package write_get_agent_activity -- --agents 2 --behaviour write:1 --behaviour get_agent_activity:1 --duration 60
```
