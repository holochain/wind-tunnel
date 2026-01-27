mod custom_metrics_table;

use crate::OperationRecord;
use crate::report::in_memory_with_custom_metrics_reporter::custom_metrics_table::CustomMetricsTableBuilder;
use crate::report::{InMemoryReporter, ReportCollector, ReportMetric};

/// A very basic reporter that is useful while developing scenarios. It keeps all of the operations
/// and custom metrics in memory and prints a summary of the operations at the end of the run.
pub struct InMemoryWithCustomMetricsReporter {
    in_memory_reporter: InMemoryReporter,
    custom_metrics: Vec<ReportMetric>,
}

impl InMemoryWithCustomMetricsReporter {
    pub fn new() -> Self {
        Self {
            in_memory_reporter: InMemoryReporter::new(),
            custom_metrics: Vec::new(),
        }
    }

    fn print_summary_of_operations(&self) {
        // Print operations summary table
        self.in_memory_reporter.print_summary_of_operations();

        // Print custom metrics
        CustomMetricsTableBuilder::print_custom_metrics(&self.custom_metrics);
    }
}

impl ReportCollector for InMemoryWithCustomMetricsReporter {
    fn add_operation(&mut self, operation_record: &OperationRecord) {
        self.in_memory_reporter.add_operation(operation_record);
    }

    fn add_custom(&mut self, metric: crate::report::ReportMetric) {
        self.custom_metrics.push(metric);
    }

    fn finalize(&self) {
        self.print_summary_of_operations();
    }
}
