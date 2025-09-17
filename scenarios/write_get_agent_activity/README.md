## write_get_agent_activity

### Description

One agent continuously creates new entries, while a second agent continuously gets their agent activity.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info MIN_AGENTS=2 cargo run --package write_get_agent_activity -- --connection-string ws://localhost:8888 --agents 2 --behaviour write:1 --behaviour get_agent_activity:1 --duration 60
```

Running the scenario distributed is suggested to be done by partitioning your nodes. The first group run the command:

```bash
RUST_LOG=info cargo run --package write_get_agent_activity -- --connection-string ws://localhost:8888 --behaviour write --duration 60
```

Then the second group run command:

```bash
RUST_LOG=info cargo run --package write_get_agent_activity -- --connection-string ws://localhost:8888 --behaviour get_agent_activity --duration 60
```
