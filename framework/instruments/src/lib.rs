use crate::report::ReportCollector;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use wind_tunnel_core::prelude::DelegatedShutdownListener;

mod report;

pub mod prelude {
    pub use crate::report::{ReportCollector, ReportMetric};
    pub use crate::{report_operation, OperationRecord, ReportConfig, Reporter};
}

pub struct ReportConfig {
    pub dir: Option<PathBuf>,
    pub scenario_name: String,
    pub enable_in_memory: bool,
    pub enable_influx_client: bool,
    pub enable_influx_file: bool,
}

impl ReportConfig {
    pub fn new(scenario_name: String) -> Self {
        ReportConfig {
            dir: None,
            scenario_name,
            enable_in_memory: false,
            enable_influx_client: false,
            enable_influx_file: false,
        }
    }

    pub fn enable_in_memory(mut self) -> Self {
        self.enable_in_memory = true;
        self
    }

    pub fn enable_influx_client(mut self) -> Self {
        self.enable_influx_client = true;
        self
    }

    pub fn enable_influx_file(mut self, dir: PathBuf) -> Self {
        self.dir = Some(dir);
        self.enable_influx_file = true;
        self
    }

    pub fn init_reporter(
        self,
        runtime: &Runtime,
        shutdown_listener: DelegatedShutdownListener,
    ) -> anyhow::Result<Reporter> {
        if self.enable_influx_client && self.enable_influx_file {
            log::warn!("Influx client metrics and Influx file metrics are enabled at the same time. This is not recommended!");
        }

        Ok(Reporter {
            inner: [
                self.enable_in_memory.then(|| {
                    RwLock::new(Box::new(report::InMemoryReporter::new())
                        as Box<(dyn ReportCollector + Send + Sync)>)
                }),
                if self.enable_influx_client {
                    let metrics_collector = report::InfluxClientReportCollector::new(
                        runtime,
                        shutdown_listener.clone(),
                    )?;
                    Some(RwLock::new(
                        Box::new(metrics_collector) as Box<(dyn ReportCollector + Send + Sync)>
                    ))
                } else {
                    None
                },
                if self.enable_influx_file {
                    let influx_file_reporter = report::InfluxFileReportCollector::new(
                        runtime,
                        shutdown_listener,
                        self.dir.unwrap(),
                        self.scenario_name,
                    );
                    Some(RwLock::new(
                        Box::new(influx_file_reporter) as Box<(dyn ReportCollector + Send + Sync)>
                    ))
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            .collect(),
        })
    }
}

pub struct Reporter {
    inner: Vec<RwLock<Box<(dyn ReportCollector + Send + Sync)>>>,
}

impl Reporter {
    fn add_operation(&self, operation_record: &OperationRecord) {
        for collector in &self.inner {
            collector.write().add_operation(operation_record);
        }
    }

    pub fn add_custom(&self, metric: report::ReportMetric) {
        for collector in &self.inner {
            collector.write().add_custom(metric.clone());
        }
    }

    pub fn finalize(&self) {
        for collector in &self.inner {
            collector.write().finalize();
        }
    }
}

impl std::fmt::Debug for Reporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reporter").finish()
    }
}

#[derive(Clone)]
pub struct OperationRecord {
    /// The ID of the operation, application specific value
    operation_id: String,
    /// The instant when the operation started
    started: std::time::Instant,
    /// Extra attributes to be reported
    attr: HashMap<String, String>,
    /// Elapsed time of the operation
    elapsed: Option<std::time::Duration>,
    /// Whether the operation failed
    is_error: bool,
}

impl std::fmt::Debug for OperationRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperationRecord")
            .field("operation_id", &self.operation_id)
            .field("attr", &self.attr)
            .field("elapsed", &self.elapsed)
            .field("is_error", &self.is_error)
            .finish()
    }
}

impl OperationRecord {
    pub fn new(operation_id: String) -> Self {
        Self {
            operation_id,
            started: std::time::Instant::now(),
            attr: HashMap::new(),
            elapsed: None,
            is_error: false,
        }
    }

    pub fn add_attr(&mut self, key: &str, value: String) {
        self.attr.insert(key.to_string(), value);
    }

    pub fn duration(&self) -> Option<std::time::Duration> {
        self.elapsed
    }

    fn finish(&mut self) {
        self.elapsed = Some(self.started.elapsed());
    }

    fn set_error(&mut self, is_error: bool) {
        self.is_error = is_error;
    }
}

pub fn report_operation<T, E>(
    reporter: Arc<Reporter>,
    mut operation_record: OperationRecord,
    response: &Result<T, E>,
) {
    operation_record.finish();
    operation_record.set_error(response.is_err());
    reporter.add_operation(&operation_record);
}
