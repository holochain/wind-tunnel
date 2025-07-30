mod metrics;

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use wind_tunnel_instruments::Reporter;

use self::metrics::HostMetrics;

/// Reporter for Host metrics in the Wind Tunnel framework.
pub struct HostMetricsReporter {
    /// Path to the metrics file.
    metrics_file: PathBuf,
    reporter: Arc<Reporter>,
}

impl HostMetricsReporter {
    /// Creates a new [`HostMetricsReporter`] instance.
    pub fn new<P>(metrics_file: P, reporter: Arc<Reporter>) -> Self
    where
        P: Into<PathBuf>,
    {
        HostMetricsReporter {
            metrics_file: metrics_file.into(),
            reporter,
        }
    }

    /// Collects host metrics and reports.
    ///
    /// Returns an [`Err`] if there is an issue in opening or processing the metrics file.
    pub fn report(&self) -> anyhow::Result<()> {
        // open the metrics file and prepare for reporting
        let file = File::open(&self.metrics_file)
            .map_err(|e| anyhow::anyhow!("Failed to open metrics file: {e}"))?;

        // iterate over lines
        let reader = BufReader::new(file);
        for line in reader.lines() {
            // handle each line, expecting it to be a valid HostMetrics entry
            // we don't return errors for each line, but log them instead
            // this allows us to continue processing even if some lines are malformed
            let line = match line {
                Ok(line) => line,
                Err(e) => {
                    log::error!("Failed to read line from metrics file: {e}");
                    continue;
                }
            };
            // parse line into metrics
            let metrics = match HostMetrics::from_str(&line) {
                Ok(metrics) => metrics,
                Err(e) => {
                    log::error!("Failed to parse metrics line '{line}': {e}");
                    continue;
                }
            };

            self.reporter.add_custom(metrics.report_metric());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use wind_tunnel_core::prelude::ShutdownHandle;
    use wind_tunnel_instruments::ReportConfig;

    use super::*;

    const JSON_PATH: &str = "tests/host_metrics.json";

    #[tokio::test]
    async fn test_should_report_host_metrics() {
        let reporter = test_reporter();
        let metrics_file = PathBuf::from(JSON_PATH);
        let host_metrics_reporter = HostMetricsReporter::new(metrics_file, reporter.clone());

        assert!(host_metrics_reporter.report().is_ok());
    }

    fn test_reporter() -> Arc<Reporter> {
        let runtime = tokio::runtime::Handle::current();
        let shutdown_listener = ShutdownHandle::new().new_listener();
        Arc::new(
            ReportConfig::new("".to_string(), "".to_string())
                .enable_in_memory()
                .init_reporter(&runtime, shutdown_listener)
                .unwrap(),
        )
    }
}
