## write_get_agent_activity_volatile

### Description

A scenario where 'write' peers create entries, while 'get_agent_activity_volatile' peers each query a single 'write' agent's activity with `get_agent_activity` but shutdown and restsart their conductors at semi random intervals.

Before a target 'write' peer and the requesting 'get_agent_activity_volatile' peer are in sync, this will measure the `get_agent_activity` call performance over a network. Once a 'write' peer reaches sync with a 'get_agent_activity' peer, the 'write' peer will publish their actions and entries, and so the `get_agent_activity` calls will likely have most of the data they need locally. At that point this measures the database query performance and code paths through host functions.

The `get_agent_activity_volatile` peers start their conductor, run their behavior for a random duration from 10-30 seconds, shutdown their conductor, wait for a random duration from 2-10 seconds, and repeat.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info MIN_AGENTS=2 cargo run --package write_get_agent_activity_volatile -- --agents 2 --behaviour write:1 --behaviour get_agent_activity_volatile:1 --duration 60
```
