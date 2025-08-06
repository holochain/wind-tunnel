#[macro_use]
extern crate log;

use clap::Parser as _;
use tempfile::NamedTempFile;

use crate::aggregator::HostMetricsAggregator;
use crate::influx::InfluxFileReporter;
use crate::jsonl::JsonlReader;
use crate::metrics::HostMetrics;
use crate::run_scenario::RunScenario;
use crate::telegraf::{Telegraf, TelegrafConfig};

mod aggregator;
mod cli;
mod influx;
mod jsonl;
mod metrics;
mod run_scenario;
mod telegraf;

const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> anyhow::Result<()> {
    env_logger::try_init()?;

    let args = cli::CliArgs::try_parse()?;
    info!("{CRATE_NAME} {CRATE_VERSION}");
    info!("Using host metrics file: {}", args.host_metrics.display());
    info!("Using run summary file: {}", args.run_summary.display());

    // parse the host metrics file
    debug!("Parsing host metrics file: {}", args.host_metrics.display());
    let host_metrics: Vec<HostMetrics> = JsonlReader::default()
        .allow_invalid_entries(true)
        .parse_from_file(&args.host_metrics)?;
    debug!("Parsed {} host metrics entries", host_metrics.len());

    // parse the run summary file
    debug!("Parsing run summary file: {}", args.run_summary.display());
    let run_summary: Vec<RunScenario> =
        JsonlReader::default().parse_from_file(&args.run_summary)?;
    debug!("Parsed {} run summary entries", run_summary.len());

    // Create the metrics writer
    let output_file = NamedTempFile::with_suffix(".influx")?;
    debug!(
        "Writing aggregated metrics to file: {}",
        output_file.path().display()
    );
    let reporter = InfluxFileReporter::from_file(&output_file)?;

    // aggregate and write the host metrics
    debug!("Aggregating host metrics");
    HostMetricsAggregator::aggregate_by_scenario(reporter, &run_summary, &host_metrics)?;

    // Run telegraf to report the metrics
    let telegraf_config = TelegrafConfig::default()
        .bucket(args.bucket)
        .influxdb_token(args.influxdb_token)
        .influxdb_url(args.influxdb_url)
        .metrics_file_path(output_file.path().to_path_buf())
        .organization(args.organization);
    // output config
    let telegraf_config_file = NamedTempFile::with_suffix(".telegraf.conf")?;
    debug!(
        "Writing Telegraf configuration to: {}",
        telegraf_config_file.path().display()
    );
    telegraf_config.write(telegraf_config_file.as_file())?;
    debug!("Telegraf configuration written successfully");
    // run telegraf
    debug!(
        "Running Telegraf with configuration file: {}",
        telegraf_config_file.path().display()
    );
    Telegraf::new(telegraf_config_file.path()).run()?;

    info!("Host metrics import completed successfully");

    Ok(())
}
