mod metrics_report;
mod summary_report;

use crate::OperationRecord;
pub use metrics_report::MetricsReportCollector;
pub use summary_report::SummaryReportCollector;

pub trait ReportCollector {
    fn add_operation(&mut self, operation_record: &OperationRecord);

    fn finalize(&self);
}
