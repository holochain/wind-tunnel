# Importing Host Metrics

Currently Telegraf is configured to output Host Metrics in a JSONL file.
This tool can be used to aggregate these metrics by the run test in the `run_summary.jsonl` file to create a influxdb compatible file, which is later imported into InfluxDB with a `telegraf` job, which is directly run by this tool.

## Usage

```bash
RUST_LOG=info cargo run -p import_host_metrics -- \
  --bucket windtunnel \
  --influxdb-url http://localhost:8087 \
  --influxdb-token ${INFLUX_TOKEN} \
  --host-metrics telegraf/metrics/metrics.jsonl \
  --organization holo \
  --run-summary run_summary.jsonl
```

Where:

- `--bucket` is the InfluxDB bucket to write the metrics to.
- `--influxdb-url` is the URL of the InfluxDB instance.
- `--influxdb-token` is the InfluxDB token to use for authentication.
- `--host-metrics` is the path to the JSONL file containing the host metrics.
- `--organization` is the InfluxDB organization to use.
- `--run-summary` is the path to the JSONL file containing the run summary.
