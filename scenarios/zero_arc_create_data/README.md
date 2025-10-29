## zero_arc_create_data

### Description

A zero-arc/full-arc mixed scenario where zero arc nodes create data and full arc nodes read the data. The scenario has two roles:

- _zero_: A zero arc conductor that just creates entries with a timestamp field. Those entries are linked to a known base hash so that full arc nodes can retrieve them.
  For each write the following metrics get recorded:
  - `wt.custom.zero_arc_create_data_entry_created_count`: The count of timed entries created by the zero arc node
  - `wt.custom.zero_arc_create_data_open_connections`: The number of currently open connections to other conductors
- _full_: A full arc conductor that reads the entries created by the zero arc node(s) and records the time lag between when the entry had been created and when it was first discovered.
  For each scenario run the following metrics get recorded:
  - `wt.custom.zero_arc_create_data_sync_lag`: For each newly found entry, the time lag between when it was created and when it was found via the `get_timed_entries_local` zome function.
  - `wt.custom.zero_arc_create_data_recv_count`: How many entries created by zero arc nodes that have been received and actively read so far
  - `wt.custom.zero_arc_create_data_open_connections`: The number of currently open connections to other conductors

### Suggested command

You can run the scenario locally with e.g. the following command:

```bash
RUST_LOG=info cargo run --package zero_arc_create_data -- --agents 2 --behaviour zero:1 --behaviour full:1 --duration 300
```

However, doing so is not that meaningful because data is all local so the lag should be minimal.

Running the scenario distributed is suggested to be done by partitioning your nodes. The first group run the command:

```bash
RUST_LOG=info cargo run --package zero_arc_create_data -- --behaviour zero --duration 300
```

Then the second group run command:

```bash
RUST_LOG=info cargo run --package zero_arc_create_data -- --behaviour full --duration 900
```
