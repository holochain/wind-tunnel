mod host;

use std::{path::PathBuf, sync::Arc};

use wind_tunnel_instruments::Reporter;

use self::host::HostMetricsReporter;

/// PostRun is a struct that is used to collect postrun metrics and reports for scenarios.
pub struct PostRun {
    pub(crate) host_metrics_reporter: Option<HostMetricsReporter>,
}

impl PostRun {
    /// Creates a new [`PostRun`] instance.
    ///
    /// if `host_metrics_file` is provided, it will initialize a [`HostMetricsReporter`] to collect host metrics.
    pub fn new<P>(reporter: Arc<Reporter>, host_metrics_file: Option<P>) -> Self
    where
        P: Into<PathBuf>,
    {
        PostRun {
            host_metrics_reporter: host_metrics_file
                .map(|file| HostMetricsReporter::new(file, reporter)),
        }
    }

    /// Collects postrun metrics and reports.
    pub fn run(&self) -> anyhow::Result<()> {
        if let Some(reporter) = &self.host_metrics_reporter {
            reporter.report()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::test_utils::test_reporter;

    use super::*;

    const JSON_PATH: &str = "tests/host_metrics.json";

    #[tokio::test]
    async fn test_should_run_postrun() {
        let reporter = test_reporter();
        let postrun = PostRun::new(reporter.clone(), Some(JSON_PATH));
        assert!(postrun.run().is_ok());
    }
}
