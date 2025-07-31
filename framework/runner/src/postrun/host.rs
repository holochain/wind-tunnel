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

        let mut errors = 0;
        let mut reported = 0;

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
                    errors += 1;
                    continue;
                }
            };
            // parse line into metrics
            let metrics = match HostMetrics::from_str(&line) {
                Ok(metrics) => metrics,
                Err(e) => {
                    log::error!("Failed to parse metrics line '{line}': {e}");
                    errors += 1;
                    continue;
                }
            };

            self.reporter.add_custom(metrics.report_metric());
            reported += 1;
        }

        log::info!("Reported {reported} host metrics with {errors} errors.");

        // if we have no valid metrics reported but encountered errors,
        // we return an error to indicate that something went wrong
        // this is to ensure that we don't silently ignore issues.
        // if we have at least one valid metric reported, we consider it a success
        if reported == 0 && errors > 0 {
            return Err(anyhow::anyhow!(
                "No valid metrics reported, but encountered {errors} errors."
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::test_utils::test_reporter;

    use super::*;

    const JSON_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/host_metrics.json");
    const MALFORMED_JSON_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/malformed_host_metrics.json"
    );
    const MALFORMED_JSON_WITH_ONE_VALID_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/malformed_host_metrics_with_one_valid.json"
    );
    const EMPTY_JSON_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/empty_host_metrics.json");

    #[tokio::test]
    async fn test_should_report_host_metrics() {
        let reporter = test_reporter();
        let metrics_file = PathBuf::from(JSON_PATH);
        let host_metrics_reporter = HostMetricsReporter::new(metrics_file, reporter.clone());

        assert!(host_metrics_reporter.report().is_ok());
    }

    #[tokio::test]
    async fn test_should_report_error_if_malformed_file() {
        let reporter = test_reporter();
        let metrics_file = PathBuf::from(MALFORMED_JSON_PATH);
        let host_metrics_reporter = HostMetricsReporter::new(metrics_file, reporter.clone());

        assert!(host_metrics_reporter.report().is_err());
    }

    #[tokio::test]
    async fn test_should_report_ok_if_at_least_one_metric_is_valid() {
        let reporter = test_reporter();
        let metrics_file = PathBuf::from(MALFORMED_JSON_WITH_ONE_VALID_PATH);
        let host_metrics_reporter = HostMetricsReporter::new(metrics_file, reporter.clone());

        assert!(host_metrics_reporter.report().is_ok());
    }

    #[tokio::test]
    async fn test_should_report_ok_if_empty_file() {
        let reporter = test_reporter();
        let metrics_file = PathBuf::from(EMPTY_JSON_PATH);
        let host_metrics_reporter = HostMetricsReporter::new(metrics_file, reporter.clone());

        assert!(host_metrics_reporter.report().is_ok());
    }

    #[tokio::test]
    async fn test_should_report_error_if_file_not_found() {
        let reporter = test_reporter();
        let metrics_file = PathBuf::from("invalid/path/to/metrics.json");
        let host_metrics_reporter = HostMetricsReporter::new(metrics_file, reporter.clone());

        assert!(host_metrics_reporter.report().is_err());
    }
}
