## full_arc_create_validated_zero_arc_read

### Description

A zero-arc/full-arc mixed scenario where full-arc nodes create data that gets validated and zero-arc nodes read the data. The scenario has two roles:

- _zero_: A zero-arc node that reads the entries created by the full-arc node(s) and records the time lag between when the entry had been created and when it was first discovered.
  For each write the following metrics get recorded:
  - `wt.custom.full_arc_create_validated_zero_arc_read_open_connections`: The number of currently open connections to other conductors
  - `wt.custom.full_arc_create_validated_zero_arc_read_sync_lag`: For each newly found entry, the time lag between when it was created and when it was found via the `get_timed_entries_local` zome function.
  - `wt.custom.full_arc_create_validated_zero_arc_read_recv_count`: The number of entries created by full-arc nodes that have been successfully retrieved so far
  Furthermore, if an error occurs when trying to fetch an entry, the `wt.custom.full_arc_create_validated_zero_arc_read_retrieval_error` metric gets recorded.

- _full_: A full-arc node that just creates entries with a timestamp field. Those entries are linked to a known base hash so that zero-arc nodes can retrieve them.
  For each scenario run the following metrics get recorded:
  - `wt.custom.full_arc_create_validated_zero_arc_read_entry_created_count`: The count of timed entries created by the full-arc node
  - `wt.custom.full_arc_create_validated_zero_arc_read_open_connections`: The number of currently open connections to other conductors

### Suggested command

You can run the scenario locally with the following command:

```bash
RUST_LOG=info cargo run --package full_arc_create_validated_zero_arc_read -- --agents 3 --behaviour zero:1 --behaviour full:2 --duration 300
```

However, doing so is not that meaningful because data is all local so the lag should be minimal.

Running the scenario distributed is suggested to be done by partitioning your nodes. The first group run the command:

```bash
RUST_LOG=info cargo run --package full_arc_create_validated_zero_arc_read -- --behaviour zero --duration 300
```

Then the second group (at least two nodes) run the command:

```bash
RUST_LOG=info cargo run --package full_arc_create_validated_zero_arc_read -- --behaviour full --duration 900
```
