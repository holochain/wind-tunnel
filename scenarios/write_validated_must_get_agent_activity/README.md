## write_validated_must_get_agent_activity

### Description

A scenario where 'write' agents creates entries, while 'get_agent_activity' agents create entries which are validated with a `must_get_agent_activty` call for a single 'write' agent's activity.

Before a target 'write' peer and the requesting peer are in sync, the `must_get_agent_activity` request will go over the network. Once a 'write' peer reaches sync with the requesting 'must_get_agent_activity' peer, the 'write' peer will publish their actions and entries, and so the `must_get_agent_activity` calls will likely have the data it need locally. At that point this measures the database query performance and code paths through host functions.

### Suggested command

```bash
RUST_LOG=info MIN_AGENTS=2 cargo run --package write_validated_must_get_agent_activity -- --agents 2 --behaviour write:1 --behaviour must_get_agent_activity:1 --duration 300
```
