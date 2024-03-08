mod metrics_report;
mod summary_report;

use std::ops::Deref;
use influxive_core::{Metric, StringType};
use crate::OperationRecord;

pub use metrics_report::MetricsReportCollector;
pub use summary_report::SummaryReportCollector;

/// A simple, opinionated, newtype for the influxive_core::Metric type.
///
/// The reported timestamp for the metric will be the current time when the metric is created.
/// The name you choose will be transformed into `ws.instruments.custom.<name>`.
pub struct ReportMetric(Metric);

impl ReportMetric {
    pub fn new(name: &str) -> Self {
        Self(Metric::new(
            std::time::SystemTime::now(),
            format!("wt.custom.{}", name),
        ))
    }

    pub fn with_field<N, V>(mut self, name: N, value: V) -> Self
    where
        N: Into<StringType>,
        V: Into<influxive_core::DataType>,
    {
        self.0 = self.0.with_field(name, value);
        self
    }

    pub fn with_tag<N, V>(mut self, name: N, value: V) -> Self
    where
        N: Into<StringType>,
        V: Into<influxive_core::DataType>,
    {
        self.0 = self.0.with_tag(name, value);
        self
    }

    pub(crate) fn into_inner(self) -> Metric {
        self.0
    }
}

// TODO temporary, prefer to do without multiple reporter implementations
impl Clone for ReportMetric {
    fn clone(&self) -> Self {
        let mut new_inner = Metric::new(self.timestamp, self.name.clone());
        for (k, v) in &self.fields {
            new_inner = new_inner.with_field(k.clone(), v.clone());
        }
        for (k, v) in &self.tags {
            new_inner = new_inner.with_tag(k.clone(), v.clone());
        }
        Self(new_inner)
    }
}

impl Deref for ReportMetric {
    type Target = Metric;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait ReportCollector {
    fn add_operation(&mut self, operation_record: &OperationRecord);

    /// Record a custom metric that
    fn add_custom(&mut self, metric: ReportMetric);

    fn finalize(&self);
}
