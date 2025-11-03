## zero_arc_create_and_read

### Description

A zero-arc/full-arc mixed scenario with two types of zero arc nodes, ones that create data and ones that read data, as well as full arc nodes to "relay" the data. The scenario has three roles:

- _zero_write_: A zero arc conductor that just creates entries with a timestamp field. Those entries are linked to a known base hash so that _zero_read_ nodes can retrieve them.
  For each write the following metrics get recorded:
  - `wt.custom.zero_arc_create_and_read_entry_created_count`: The count of timed entries created by the zero arc node
  - `wt.custom.zero_arc_create_and_read_open_connections`: The number of currently open connections to other conductors
- _zero_read_: A zero arc conductor that reads the entries created by the zero arc node(s) and records the time lag between when the entry had been created and when it was first discovered.
  For each scenario run the following metrics get recorded:
  - `wt.custom.zero_arc_create_and_read_sync_lag`: For each newly found entry, the time lag between when it was created and when it was found via the `get_timed_entries_network` zome function.
  - `wt.custom.zero_arc_create_and_read_recv_count`: How many entries created by zero arc nodes that have been received and actively read so far
  - `wt.custom.zero_arc_create_and_read_open_connections`: The number of currently open connections to other conductors
- _full_: A full arc conductor that is just here to serve entries to zero arc nodes.
  For each scenario run the following metrics get recorded:
  - `wt.custom.zero_arc_create_and_read_open_connections`: The number of currently open connections to other conductors


### Suggested command

> [!IMPORTANT]
> This scenario requires at least 2 full arc nodes. Otherwise they won't ever set their own storage arc to "full" and won't be considered as a valid publish target by zero arc nodes.

You can run the scenario locally with the following command:

```bash
RUST_LOG=info cargo run --package zero_arc_create_and_read -- --agents 4 --behaviour zero_read:1 --behaviour zero_write:1 --behaviour full:2 --duration 300
```

However, doing so is not that meaningful because data is all local so the lag should be minimal.

Running the scenario distributed is suggested to be done by partitioning your nodes. The first group run the command:

```bash
RUST_LOG=info cargo run --package zero_arc_create_and_read -- --behaviour zero_write --duration 300
```

Then the second group run the command:

```bash
RUST_LOG=info cargo run --package zero_arc_create_and_read -- --behaviour zero_read --duration 300
```

And the third group (at least 2 nodes) run the command:

```bash
RUST_LOG=info cargo run --package zero_arc_create_and_read -- --behaviour full --duration 300
```
