## mixed_arc_get_agent_activity

### Description

A scenario where zero-arc ('zero_write') and full-arc ('full_write') peers create entries while other zero-arc ('zero_read') peers each repeatedly query a single 'zero_write' or 'full_write' agent's activity with `get_agent_activity`.

- _full_write_ / _zero_write_: Creates entries.

  For each scenario run the following metrics get recorded:

  - `wt.custom.mixed_arc_get_agent_activity_entry_created_count`: The total number of entries created so far.

  Furthermore, approximately every 3 seconds, the number of open connections gets recorded:

  - `wt.custom.mixed_arc_get_agent_activity_open_connections`: The number of currently open connections to other conductors

- _zero_read_: Selects a single 'zero-write' agent for the duration of the scenario run and queries the agent's activity.

  For each scenario run the following metrics get recorded:

  - `wt.custom.mixed_arc_get_agent_activity_new_chain_head_delay`: If a new chain head is observed, the time difference between now and when the previous chain head was observed (in case the chain head jumped by more than one action, the average time difference per action).

  - (if no error occurs when fetching agent activity) `wt.custom.mixed_arc_get_agent_activity_highest_observed_action_seq`: The current chain head (i.e. highest action sequence number) of the queried agent's agent activity.

  - (if an error occurs when fetching agent activity) `wt.custom.mixed_arc_get_agent_activity_retrieval_error`: A simple data point of value 1.

  Furthermore, approximately every 3 seconds, the number of open connections gets recorded:

  - `wt.custom.mixed_arc_get_agent_activity_open_connections`: The number of currently open connections to other conductors

### Suggested command

> [!IMPORTANT]
> This scenario requires at least 2 full arc nodes. Otherwise they won't ever set their own storage arc to "full" and won't be considered as a valid publish target by the 'zero_write' nodes.

You can run the scenario locally with the following command:

```bash
RUST_LOG=info cargo run --package mixed_arc_get_agent_activity -- --agents 6 --behaviour zero_read:3 --behaviour zero_write:1 --behaviour full_write:2 --duration 300
```

Running the scenario in a distributed manner is suggested to be done by partitioning your nodes. The first group run the command:

```bash
RUST_LOG=info cargo run --package mixed_arc_get_agent_activity -- --behaviour zero_read --duration 500
```

Then the second group run the command:

```bash
RUST_LOG=info cargo run --package mixed_arc_get_agent_activity -- --behaviour zero_write --duration 300
```

And the third group (at least two nodes) run the command:

```bash
RUST_LOG=info cargo run --package mixed_arc_get_agent_activity -- --behaviour full_write --duration 300
```
