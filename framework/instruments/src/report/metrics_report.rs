use crate::report::ReportCollector;
use crate::OperationRecord;

pub struct MetricsReportCollector {}

impl MetricsReportCollector {
    pub fn new() -> Self {
        Self {}
    }
}

impl ReportCollector for MetricsReportCollector {
    fn add_operation(&mut self, _operation_record: &OperationRecord) {
        todo!()
    }

    fn finalize(&self) {
        todo!()
    }
}
