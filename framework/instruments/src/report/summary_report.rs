use std::collections::HashMap;
use crate::report::ReportCollector;
use crate::OperationRecord;
use itertools::Itertools;

pub struct SummaryReportCollector {
    operation_records: Vec<OperationRecord>,
}

impl SummaryReportCollector {
    pub fn new() -> Self {
        Self {
            operation_records: Vec::new(),
        }
    }
}

impl ReportCollector for SummaryReportCollector {
    fn add_operation(&mut self, operation_record: &OperationRecord) {
        self.operation_records.push(operation_record.clone());
    }

    fn finalize(&self) {
        let total_operations = self.operation_records.len();
        let total_duration_millis = self
            .operation_records
            .iter()
            .map(|record| record.duration().unwrap().as_millis())
            .sum::<u128>();

        println!("Total operations: {}", total_operations);
        println!("Total operations time: {}ms", total_duration_millis);

        self.operation_records.iter().filter(|op| !op.is_error).max_by(|a, b| a.duration().unwrap().cmp(&b.duration().unwrap())).map(|record| {
            println!("Slowest successful operation: {:?}", record);
        });

        self.operation_records.iter().filter(|op| !op.is_error).min_by(|a, b| a.duration().unwrap().cmp(&b.duration().unwrap())).map(|record| {
            println!("Fastest successful operation: {:?}", record);
        });

        self.operation_records.iter().filter(|op| op.is_error).max_by(|a, b| a.duration().unwrap().cmp(&b.duration().unwrap())).map(|record| {
            println!("Slowest unsuccessful operation: {:?}", record);
        });

        self.operation_records.iter().filter(|op| op.is_error).min_by(|a, b| a.duration().unwrap().cmp(&b.duration().unwrap())).map(|record| {
            println!("Fastest unsuccessful operation: {:?}", record);
        });

        let grouped = self.operation_records.iter().fold(HashMap::new(), |mut acc, record| {
            match acc.entry(record.operation_id.clone()) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(vec![record.clone()]);
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().push(record.clone());
                }
            }
            acc
        });

        for (operation_id, operations) in grouped {
            let total_ops_for_id = operations.len();
            let total_duration_millis_for_id = operations.iter().map(|record| record.duration().unwrap().as_millis()).sum::<u128>();

            println!();
            println!("Operation ID: {}", operation_id);
            println!("Total operations: {}", total_ops_for_id);
            println!("Total operations time: {}ms", total_duration_millis_for_id);
            println!("Average time: {}ms", total_duration_millis_for_id as f64 / total_ops_for_id as f64);

            operations.iter().filter(|op| !op.is_error).max_by(|a, b| a.duration().unwrap().cmp(&b.duration().unwrap())).map(|record| {
                println!("Slowest successful: {:?}", record);
            });

            operations.iter().filter(|op| !op.is_error).min_by(|a, b| a.duration().unwrap().cmp(&b.duration().unwrap())).map(|record| {
                println!("Fastest successful: {:?}", record);
            });

            operations.iter().filter(|op| op.is_error).max_by(|a, b| a.duration().unwrap().cmp(&b.duration().unwrap())).map(|record| {
                println!("Slowest unsuccessful: {:?}", record);
            });

            operations.iter().filter(|op| op.is_error).min_by(|a, b| a.duration().unwrap().cmp(&b.duration().unwrap())).map(|record| {
                println!("Fastest unsuccessful: {:?}", record);
            });
        }
    }
}
