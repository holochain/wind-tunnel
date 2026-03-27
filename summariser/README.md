# Wind Tunnel: Summariser

This is a tool to summarise the results of Wind Tunnel scenario runs.

Start by running one or more scenarios with `--reporter influx-file` to create metrics on InfluxDB. You should find a
`run_summary.jsonl` file created in the root of the project. The summariser reads this to find metrics.

You can then run the summariser to generate a report:

```shell
RUST_LOG=info cargo run summariser
```

This will create a new JSON file that summarises the results from the scenarios.

## Testing the summariser

The summariser comes with some tooling for testing it. With a `run_summary.jsonl` that contains a run which you want to
use as a test case, you can run the following command:

```shell
RUST_LOG=debug cargo run --features test_data summariser
```

This will add:

- The run summary to `summariser/test_data/1_run_summaries/<scenario-name>-<scenario run fingerprint>.json`
- The raw data fetched from influx, as JSON to `summariser/test_data/2_query_results/<query fingerprint>.json`
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

## Adding scenarios to the summariser

Once a scenario emits metrics, you can add support for it in the summariser to generate structured reports:

1. **Create the scenario module** at `summariser/src/scenario/<scenario_name>.rs`:
   - Define a summary struct with `#[derive(Debug, Clone, Serialize, Deserialize)]`
   - Add doc comments on each field explaining what it measures
   - Implement `summarize_<scenario_name>(client: influxdb::Client, summary: RunSummary) -> anyhow::Result<YourSummary>`
   - Use helper functions from `crate::query` and `crate::analyze` to query InfluxDB for data and aggregate it

2. **Register the module** in `summariser/src/scenario.rs`:

   ```rust
   mod <scenario_name>;
   pub(crate) use <scenario_name>::summarize_<scenario_name>;
   ```

3. **Wire up dispatch** in `summariser/src/lib.rs` by adding a match arm in `execute_report_for_run_summary()`:

   ```rust
   "<scenario_name>" => Some(execute_report_with_host_metrics!(
       client, summary, summarize_<scenario_name>
   )),
   ```

4. **Development cycle to build out summary**:
   - Run the scenario with `--reporter influx-file`
   - Import metrics into local InfluxDB: `nix run .#local-upload-metrics`
   - Iterate on adding custom metrics and instrumented zome calls to the summary:
     - Add more metrics to the summary
     - Run summariser `RUST_LOG=holochain_summariser=debug cargo run summariser`
     - Repeat
   - Note that a metric can only record one field and its identifier must be `value`

5. **Capture test data** for snapshot testing:
   - Run the scenario with `--reporter=influx-file`
   - Import metrics: `nix run .#local-upload-metrics`
   - Capture test data: `RUST_LOG=debug cargo run --features test_data summariser`
   - This creates files in `summariser/test_data/{1_run_summaries,2_query_results,3_summary_outputs}/`

6. **Add a snapshot test** in `summariser/tests/snapshot.rs`:

   ```rust
   #[tokio::test]
   async fn <scenario_name>() -> anyhow::Result<()> {
       run_snapshot_test!("<fingerprint from filename>");
       Ok(())
   }
   ```

7. **Commit all test data files** to git. The snapshot tests replay captured InfluxDB responses without needing a live database.

### Debugging

If the summariser run with or without feature "test_data" fails, it's helpful to inspect the data that has been uploaded to the
local InfluxDB. Navigate to `http://localhost:8087` and log in with username and password `windtunnel`/`windtunnel`. Go to
"Data Explorer" -> "Buckets" -> "windtunnel". On the following screen you can create queries and view the results.
