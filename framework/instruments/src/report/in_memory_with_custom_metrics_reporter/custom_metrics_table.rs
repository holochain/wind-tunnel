use crate::report::ReportMetric;
use std::collections::{BTreeMap, HashSet};
use tabled::settings::Style;
use tabled::Table;
use tabled::Tabled;

#[derive(Tabled)]
pub struct MetricTableRow {
    #[tabled(rename = "#")]
    pub index: usize,
    #[tabled(rename = "Time")]
    pub timestamp: String,
    #[tabled(rename = "Fields")]
    pub fields: FieldsWrapper,
    #[tabled(rename = "Tags")]
    pub tags: TagsWrapper,
}

// Wrapper types to implement Display for Vec<String> without violating orphan rules
#[derive(Debug, Clone)]
pub struct FieldsWrapper(pub Vec<String>);

#[derive(Debug, Clone)]
pub struct TagsWrapper(pub Vec<String>);

impl std::fmt::Display for FieldsWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join(", "))
    }
}

impl std::fmt::Display for TagsWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join(", "))
    }
}

pub struct CustomMetricsTableBuilder;

impl CustomMetricsTableBuilder {
    /// Main entry point for printing custom metrics
    pub fn print_custom_metrics(metrics: &[ReportMetric]) {
        if metrics.is_empty() {
            return;
        }

        println!("\nCustom Metrics");

        // Group metrics by name for better organization
        let grouped_metrics = Self::group_metrics_by_name(metrics);

        for (metric_name, metrics) in grouped_metrics {
            Self::print_metric_group(&metric_name, &metrics);
        }
    }

    pub fn group_metrics_by_name(metrics: &[ReportMetric]) -> BTreeMap<String, Vec<&ReportMetric>> {
        let mut grouped = BTreeMap::new();

        for metric in metrics {
            grouped
                .entry(metric.name.clone().into_string())
                .or_insert_with(Vec::new)
                .push(metric);
        }

        grouped
    }

    fn print_metric_group(metric_name: &str, metrics: &[&ReportMetric]) {
        println!("\n📊 {} ({})", metric_name, metrics.len());

        if metrics.len() == 1 {
            Self::print_single_metric(metrics[0]);
        } else {
            Self::print_multiple_metrics(metrics);
        }
    }

    fn print_single_metric(metric: &ReportMetric) {
        // For single metrics, show a detailed view
        if !metric.fields.is_empty() {
            println!("  Fields:");
            for (key, value) in &metric.fields {
                println!(
                    "    {}: {}",
                    key.clone().into_string(),
                    Self::format_data_type(value)
                );
            }
        }

        if !metric.tags.is_empty() {
            println!("  Tags:");
            for (key, value) in &metric.tags {
                println!(
                    "    {}: {}",
                    key.clone().into_string(),
                    Self::format_data_type(value)
                );
            }
        }

        println!("  Timestamp: {}", Self::format_timestamp(metric.timestamp));
    }

    fn print_multiple_metrics(metrics: &[&ReportMetric]) {
        // For multiple metrics, create a table with common fields as columns
        let common_fields = Self::find_common_fields(metrics);
        let common_tags = Self::find_common_tags(metrics);

        if !common_fields.is_empty() || !common_tags.is_empty() {
            let table_data = Self::create_table_data(
                metrics,
                &common_fields,
                &common_tags,
                Self::format_data_type,
                Self::format_timestamp,
            );
            let mut table = Table::new(table_data);
            table.with(Style::modern());
            println!("{}", table);
        } else {
            // Fallback to individual display if no common structure
            for (i, metric) in metrics.iter().enumerate() {
                println!("  Entry {}:", i + 1);
                Self::print_single_metric(metric);
            }
        }
    }

    pub fn find_common_fields(metrics: &[&ReportMetric]) -> Vec<String> {
        if metrics.is_empty() {
            return Vec::new();
        }

        let first_fields: HashSet<_> = metrics[0]
            .fields
            .iter()
            .map(|(k, _)| k.clone().into_string())
            .collect();

        metrics
            .iter()
            .skip(1)
            .fold(first_fields, |acc, metric| {
                let metric_fields: HashSet<_> = metric
                    .fields
                    .iter()
                    .map(|(k, _)| k.clone().into_string())
                    .collect();
                acc.intersection(&metric_fields).cloned().collect()
            })
            .into_iter()
            .collect()
    }

    pub fn find_common_tags(metrics: &[&ReportMetric]) -> Vec<String> {
        if metrics.is_empty() {
            return Vec::new();
        }

        let first_tags: HashSet<_> = metrics[0]
            .tags
            .iter()
            .map(|(k, _)| k.clone().into_string())
            .collect();

        metrics
            .iter()
            .skip(1)
            .fold(first_tags, |acc, metric| {
                let metric_tags: HashSet<_> = metric
                    .tags
                    .iter()
                    .map(|(k, _)| k.clone().into_string())
                    .collect();
                acc.intersection(&metric_tags).cloned().collect()
            })
            .into_iter()
            .collect()
    }

    pub fn create_table_data<F1, F2>(
        metrics: &[&ReportMetric],
        common_fields: &[String],
        common_tags: &[String],
        format_data_type: F1,
        format_timestamp: F2,
    ) -> Vec<MetricTableRow>
    where
        F1: Fn(&influxive_core::DataType) -> String,
        F2: Fn(std::time::SystemTime) -> String,
    {
        metrics
            .iter()
            .enumerate()
            .map(|(i, metric)| {
                let mut row = MetricTableRow {
                    index: i + 1,
                    timestamp: format_timestamp(metric.timestamp),
                    fields: FieldsWrapper(Vec::new()),
                    tags: TagsWrapper(Vec::new()),
                };

                for field_name in common_fields {
                    if let Some((_, value)) = metric
                        .fields
                        .iter()
                        .find(|(k, _)| k.clone().into_string() == *field_name)
                    {
                        row.fields
                            .0
                            .push(format!("{}: {}", field_name, format_data_type(value)));
                    }
                }

                for tag_name in common_tags {
                    if let Some((_, value)) = metric
                        .tags
                        .iter()
                        .find(|(k, _)| k.clone().into_string() == *tag_name)
                    {
                        row.tags
                            .0
                            .push(format!("{}: {}", tag_name, format_data_type(value)));
                    }
                }

                row
            })
            .collect()
    }

    fn format_data_type(data_type: &influxive_core::DataType) -> String {
        match data_type {
            influxive_core::DataType::Bool(b) => b.to_string(),
            influxive_core::DataType::F64(f) => format!("{:.3}", f),
            influxive_core::DataType::I64(i) => i.to_string(),
            influxive_core::DataType::U64(u) => u.to_string(),
            influxive_core::DataType::String(s) => s.clone().into_string(),
        }
    }

    fn format_timestamp(timestamp: std::time::SystemTime) -> String {
        match timestamp.duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => {
                let secs = duration.as_secs();
                let dt = chrono::DateTime::from_timestamp(secs as i64, 0)
                    .unwrap_or(chrono::DateTime::UNIX_EPOCH);
                dt.format("%H:%M:%S").to_string()
            }
            Err(_) => "Invalid timestamp".to_string(),
        }
    }
}
