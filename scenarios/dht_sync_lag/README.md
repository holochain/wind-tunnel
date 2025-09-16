## dht_sync_lag

### Description

This scenario has two roles:
- _write_: A simple job that just creates entries with a timestamp field. Those entries are linked to a known base hash.
  For each write, the metric `ws.custom.dht_sync_sent_count` is incremented.
- _record_lag_: A job that repeatedly queries for links from the known base hash. It keeps track of records that it has seen
  and when a new record is found, it calculates the time difference between the timestamp of the new record and the current time.
  That time difference is then recorded as a custom metric called `wt.custom.dht_sync_lag`.
  After each behaviour loop the metric `ws.custom.dht_sync_recv_count` is incremented.

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info cargo run --package dht_sync_lag -- --agents 2 --behaviour write:1 --behaviour record_lag:1 --duration 900
```

However, doing so is not that meaningful because data is all local so the lag should be minimal.

Running the scenario distributed is suggested to be done by partitioning your nodes. The first group run the command:

```bash
RUST_LOG=info cargo run --package dht_sync_lag -- --behaviour write --duration 300
```

Then the second group run command:

```bash
RUST_LOG=info cargo run --package dht_sync_lag -- --behaviour record_lag --duration 900
```
