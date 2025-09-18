mod custom_metrics_table;
mod operations_table;

use crate::report::in_memory_reporter::custom_metrics_table::CustomMetricsTableBuilder;
use crate::report::in_memory_reporter::operations_table::OperationRow;
use crate::report::{ReportCollector, ReportMetric};
use crate::OperationRecord;
use std::collections::HashMap;
use tabled::settings::Style;
use tabled::Table;

/// A very basic reporter that is useful while developing scenarios. It keeps all of the operations
/// and custom metrics in memory and prints a summary of the operations at the end of the run.
pub struct InMemoryReporter {
    operation_records: Vec<OperationRecord>,
    custom_metrics: Vec<ReportMetric>,
}

impl InMemoryReporter {
    pub fn new() -> Self {
        Self {
            operation_records: Vec::new(),
            custom_metrics: Vec::new(),
        }
    }

    fn print_summary_of_operations(&self) {
        println!("\nSummary of operations");
        let rows = self
            .operation_records
            .iter()
            .fold(HashMap::new(), |mut acc, record| {
                match acc.entry(record.operation_id.clone()) {
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(vec![record.clone()]);
                    }
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        entry.get_mut().push(record.clone());
                    }
                }
                acc
            })
            .into_iter()
            .map(|(operation_id, operations)| {
                let total_operations = operations.len();
                let total_duration_micro = operations
                    .iter()
                    .map(|record| record.duration().unwrap().as_micros())
                    .sum::<u128>();

                OperationRow {
                    operation_id,
                    total_operations,
                    total_duration_ms: total_duration_micro as f64 / 1000.0,
                    avg_time_ms: (total_duration_micro as f64 / total_operations as f64) / 1000.0,
                    min_time_ms: operations
                        .iter()
                        .filter(|op| !op.is_error)
                        .min_by(|a, b| a.duration().unwrap().cmp(&b.duration().unwrap()))
                        .unwrap()
                        .elapsed
                        .unwrap()
                        .as_micros() as f64
                        / 1000.0,
                    max_time_ms: operations
                        .iter()
                        .filter(|op| !op.is_error)
                        .max_by(|a, b| a.duration().unwrap().cmp(&b.duration().unwrap()))
                        .unwrap()
                        .elapsed
                        .unwrap()
                        .as_micros() as f64
                        / 1000.0,
                }
            })
            .collect::<Vec<_>>();

        let mut table = Table::new(rows);
        table.with(Style::modern());

        println!("{}", table);

        // Print custom metrics using the dedicated module
        CustomMetricsTableBuilder::print_custom_metrics(&self.custom_metrics);
    }
}

impl ReportCollector for InMemoryReporter {
    fn add_operation(&mut self, operation_record: &OperationRecord) {
        self.operation_records.push(operation_record.clone());
    }

    fn add_custom(&mut self, metric: crate::report::ReportMetric) {
        self.custom_metrics.push(metric);
    }

    fn finalize(&self) {
        self.print_summary_of_operations();
    }
}
