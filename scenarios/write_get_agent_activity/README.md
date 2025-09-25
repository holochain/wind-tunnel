## write_get_agent_activity

### Description

A scenario where 'write' agents creates entries, while 'get_agent_activity' agents each query a single 'write' agent's activity with `get_agent_activity`.

Before a target 'write' peer and the requesting peer are in sync, this will measure the `get_agent_activity` performance over a network. Once a `write` node reaches sync with a `get_agent_activity` node, the 'write' will publish their actions and entries, and so the `get_agent_activity` calls will likely have most of the data they need locally. At that point this measures the database query performance and code paths through host functions.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info MIN_AGENTS=2 cargo run --package write_get_agent_activity -- --agents 2 --behaviour write:1 --behaviour get_agent_activity:1 --duration 60
```
