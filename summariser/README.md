# Wind Tunnel: Summariser

This is a tool to summarise the results of Wind Tunnel scenario runs.

Start by running one or more scenarios with `--reporter influx-file` to create metrics on InfluxDB. You should find a
`run_summary.jsonl` file created in the root of the project. The summariser reads this to find metrics.

You can then run the summariser to generate a report:

```shell
RUST_LOG=info cargo run summariser 
```

This will create a new JSON file that summarises the results from the scenarios.

### Testing the summariser

The summariser comes with some tooling for testing it. With a `run_summary.jsonl` that contains a run which you want to 
use as a test case, you can run the following command:

```shell
RUST_LOG=debug cargo run --features test_data summariser
```

This will add:
- The run summary to `summariser/test_data/1_run_summaries/<scenario-name>-<scenario run fingerprint>.json`
- The raw that fetched from influx, as JSON to `summariser/test_data/2_query_results/<query fingerprint>.json`
- The generated report to `summariser/test_data/3_summary_outputs/<scenario-name>-<scenario run fingerprint>.json`

All of these should be added to Git, then you can write a test that loads the test data. This allows you to iterate on 
the summariser without needing to run the scenario again or even have a running InfluxDB.

Tests just look like:

```rust
#[tokio::test]
async fn scenario_name() -> anyhow::Result<()> {
    run_snapshot_test!("<scenario fingerprint>");
    Ok(())
}
```

If you make changes to the summariser, you should review the diff that this test prints and ensure it looks correct. If
so, then rather than update test data by hand, you will be prompted to run 

```shell
UPDATE_SNAPSHOTS=1 cargo test --test snapshot
```

This will overwrite the reports with their latest version. You can review and commit the diff to match the updated code.
