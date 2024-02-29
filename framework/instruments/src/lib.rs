use crate::report::ReportCollector;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

mod metrics;
mod report;

pub struct ReportConfig {
    pub enable_metrics: bool,
    pub enable_summary: bool,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            enable_metrics: false,
            enable_summary: false,
        }
    }
}

impl ReportConfig {
    pub fn enable_metrics(mut self) -> Self {
        self.enable_metrics = true;
        self
    }

    pub fn enable_summary(mut self) -> Self {
        self.enable_summary = true;
        self
    }

    pub fn init(self) -> Reporter {
        Reporter {
            inner: [
                self.enable_metrics.then(|| {
                    RwLock::new(Box::new(report::MetricsReportCollector::new())
                        as Box<(dyn ReportCollector + Send + Sync)>)
                }),
                self.enable_summary.then(|| {
                    RwLock::new(Box::new(report::SummaryReportCollector::new())
                        as Box<(dyn ReportCollector + Send + Sync)>)
                }),
            ]
            .into_iter()
            .filter_map(|x| x)
            .collect(),
        }
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
