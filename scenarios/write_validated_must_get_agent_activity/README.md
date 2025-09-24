## write_validated_must_get_agent_activity

### Description

One agent behavior 'write' creates entries. The other agent behavior 'must_get_agent_activity' creates entries which are validated with a `must_get_agent_activity` call for the other peer's agent activity.

### Suggested command

```bash
RUST_LOG=info MIN_AGENTS=2 cargo run --package write_validated_must_get_agent_activity -- --agents 2 --behaviour write:1 --behaviour must_get_agent_activity:1 --duration 300
```
