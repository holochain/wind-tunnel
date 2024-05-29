## app_install

### Description

This scenario is for testing the speed and reliability of local signals. It uses a zome with a function that will emit
10,000 signals. The scenario measures how long it takes to send the signals and how many have been received by the time
the zome call ends.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info cargo run --package local_signals -- --connection-string ws://localhost:8888 --duration 300
```
