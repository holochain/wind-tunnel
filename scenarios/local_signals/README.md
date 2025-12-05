## local_signals

### Description

This scenario is for testing the speed and reliability of local signals. It uses a zome with a function that will emit
10,000 signals. The scenario measures how long it takes to send the signals and how many have been received by the time
the zome call ends.

Records `wt.custom.signal_batch_send` which is the time taken to emit a signal batch of 10,000 signals. Then `wt.custom.signal_batch_recv`
which is the time taken to receive the complete batch, to the nearest 250ms. Then `wt.custom.signal_success_ratio` which is the ratio
of the batch that was received out of the 10,000 sent.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info cargo run --package local_signals -- --duration 300
```
