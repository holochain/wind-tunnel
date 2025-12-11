## mixed_arc_must_get_agent_activity

### Description

A scenario where zero-arc ('zero_write') and full-arc ('full_write') peers create entries in batches of 10 every `SLEEP_INTERVAL_WRITE_BEHAVIOUR_MS` milliseconds, while other zero-arc ('zero_must_get_agent_activity') peers each repeatedly attempt to create an entry that includes a `must_get_agent_activity` call as part of its validation.

- _full_write_ / _zero_write_: Announces itself as a writing peer (by virtue of creating a link from a `WRITE_AGENTS` anchor to its agent pubkey) as part of the agent setup hook. Then for each behaviour run it creates a batch of 10 `SampleEntry` entries as well as a link from a chain batch path of the form `CHAIN_BATCH_ANCHOR.[agent pub key].[batch number]` to the last entry of the batch, then sleeps for `SLEEP_INTERVAL_WRITE_BEHAVIOUR_MS` milliseconds.

  For each behaviour run the following metrics get recorded:

  - `wt.custom.mixed_arc_must_get_agent_activity_entry_created_count`: The total number of entries created so far.

  Furthermore, approximately every 3 seconds, the number of open connections gets recorded:

  - `wt.custom.mixed_arc_must_get_agent_activity_open_connections`: The number of currently open connections to other conductors.

- _zero_must_get_agent_activity_: First it selects a single 'zero_write' or 'full_write' agent for the duration of the scenario run by virtue of picking an agent pubkey linked from the `WRITE_AGENTS` anchor. Then it tries to create a `ValidatedSampleEntry` entry that calls `must_get_agent_activity` as part of validation for a specific chain top, where the chain top hash is read from the link attached to `CHAIN_BATCH_ANCHOR.[agent pub key].[batch number]` that had been created by the selected writing peer. It tries to create said entry for a specific batch number, starting at 0, until it succeeds (meaning that `must_get_agent_activity` returned the full chain up to the chain top associated to the batch) and then proceeds to the next higher batch number (one entry creation attempt per behaviour run).

  For each behaviour run the following metrics get recorded:

  - (if no error occurs during `ValidatedSampleEntry` creation) `wt.custom.mixed_arc_must_get_agent_activity_chain_batch_delay`: Once a `ValidatedSampleEntry` associated to a certain batch has been created successfully (i.e. `must_get_agent_activity` succeeded and returned the full chain up to the chain top associated to the given batch), the delay between the successful `ValidatedSampleEntry` creation and when the link from the last batch entry to the chain batch path had been created by the write peer is recorded. **Note that since writing peers may write entries quicker than reading peers can keep up with successful `must_get_agent_activity` calls this measure is conflating different sorts of delays.**

  - (if no error occurs during `ValidatedSampleEntry` creation) `wt.custom.mixed_arc_must_get_agent_activity_chain_len`: The current chain length (i.e. highest action sequence number) of the queried agent's agent activity.

  - (if an error occurs during `ValidatedSampleEntry` creation) `wt.custom.mixed_arc_must_get_agent_activity_retrieval_error_count`: An increasing count of errors that occurred when retrieving the agent activity via `must_get_agent_activity` while attempting to create a validated entry.

  Furthermore, approximately every 3 seconds, the number of open connections gets recorded:

  - `wt.custom.mixed_arc_must_get_agent_activity_open_connections`: The number of currently open connections to other conductors.

### Suggested command

> [!IMPORTANT]
> This scenario requires at least 2 full arc nodes. Otherwise they won't ever set their own storage arc to "full" and won't be considered as a valid publish target by the 'zero_write' nodes.

You can run the scenario locally with the following command:

```bash
RUST_LOG=info cargo run --package mixed_arc_must_get_agent_activity -- --agents 6 --behaviour zero_must_get_agent_activity:3 --behaviour zero_write:1 --behaviour full_write:2 --duration 300
```

Running the scenario in a distributed manner is suggested to be done by partitioning your nodes. The first group run the command:

```bash
RUST_LOG=info cargo run --package mixed_arc_must_get_agent_activity -- --behaviour zero_must_get_agent_activity --duration 300
```

Then the second group run the command:

```bash
RUST_LOG=info cargo run --package mixed_arc_must_get_agent_activity -- --behaviour zero_write --duration 300
```

And the third group (at least two nodes) run the command:

```bash
RUST_LOG=info cargo run --package mixed_arc_must_get_agent_activity -- --behaviour full_write --duration 300
```
