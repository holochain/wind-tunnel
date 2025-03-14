## kitsune_continuous_flow

### Description

The setup of an agent in this scenario creates a chatter and it immediately joins the network.
This step includes publishing its info to the bootstrap server, to be discoverable by peers. Once joined,
the chatter will create messages periodically at an interval between 10 and 1000 ms and publish them.

The number of messages per interval can be configured with the env var `NUM_MESSAGES` and defaults to 3.

The number of chatters to be created can be configured with the option `--agents`.

The scenario runs for 30 s by default unless otherwise configured with option `--duration`.

### Suggested command

You can run the scenario locally with the following commands:

Start bootstrap and signal server in one terminal:
```bash
nix develop -c bash -c "kitsune2-bootstrap-srv --listen 127.0.0.1:30000"
```

In a new terminal:
```bash
RUST_LOG=info cargo run -p kitsune_continuous_flow -- --bootstrap-server-url http://127.0.0.1:30000 --signal-server-url ws://127.0.0.1:30000 --agents 2
```
