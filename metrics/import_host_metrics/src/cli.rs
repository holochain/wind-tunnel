use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(about, long_about = None)]
pub struct CliArgs {
    /// InfluxDB bucket name.
    #[arg(long, default_value = "windtunnel")]
    pub bucket: String,

    /// URL of the InfluxDB instance.
    #[arg(long, default_value = "http://127.0.0.1:8087")]
    pub influxdb_url: String,

    /// InfluxDB token for authentication.
    #[arg(long, env = "INFLUX_TOKEN")]
    pub influxdb_token: String,

    /// Path to the host metrics file with InfluxDB line protocol format.
    #[arg(long, default_value = "telegraf/metrics/host.influx")]
    pub host_metrics: PathBuf,

    /// InfluxDB organization name.
    #[arg(long, default_value = "holo")]
    pub organization: String,

    /// Path to the run_summary file.
    #[arg(long, default_value = "run_summary.jsonl")]
    pub run_summary: PathBuf,
}
