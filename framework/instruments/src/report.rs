mod in_memory_reporter;
mod in_memory_with_custom_metrics_reporter;

use crate::OperationRecord;

pub use in_memory_reporter::InMemoryReporter;
pub use in_memory_with_custom_metrics_reporter::InMemoryWithCustomMetricsReporter;

/// The kind of custom metric being recorded.
#[derive(Debug, Clone, Copy, Default)]
pub enum MetricKind {
    /// A point-in-time measurement (e.g. open_connections, sync_lag).
    #[default]
    Gauge,
    /// A monotonically increasing cumulative value (e.g. entry_created_count).
    Counter,
}

/// A custom metric to be reported.
///
/// All custom metrics are emitted under the `wt.custom.<name>` namespace.
#[derive(Debug, Clone)]
pub struct ReportMetric {
    pub(crate) name: String,
    pub(crate) value: f64,
    pub(crate) tags: Vec<(String, String)>,
    pub(crate) kind: MetricKind,
}

impl ReportMetric {
    /// Create a new gauge metric with the given name and value.
    ///
    /// The name will be prefixed with `wt.custom.` when emitted.
    pub fn new(name: impl Into<String>, value: impl Into<f64>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            tags: Vec::new(),
            kind: MetricKind::Gauge,
        }
    }

    /// Create a new counter metric with the given name and value.
    ///
    /// Counters represent monotonically increasing cumulative values.
    /// The name will be prefixed with `wt.custom.` when emitted.
    pub fn counter(name: impl Into<String>, value: impl Into<f64>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            tags: Vec::new(),
            kind: MetricKind::Counter,
        }
    }

    /// Add a tag to this metric.
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.push((key.into(), value.into()));
        self
    }
}

pub trait ReportCollector {
    fn add_operation(&mut self, operation_record: &OperationRecord);

    /// Record a custom metric
    fn add_custom(&mut self, metric: ReportMetric);

    fn finalize(&self);
}
