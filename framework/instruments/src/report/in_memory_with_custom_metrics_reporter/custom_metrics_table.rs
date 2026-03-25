use crate::report::ReportMetric;
use std::collections::BTreeMap;
use tabled::Table;
use tabled::Tabled;
use tabled::settings::Style;

#[derive(Tabled)]
pub struct MetricTableRow {
    #[tabled(rename = "#")]
    pub index: usize,
    #[tabled(rename = "Value")]
    pub value: String,
    #[tabled(rename = "Tags")]
    pub tags: TagsWrapper,
}

// Wrapper type to implement Display for Vec<String> without violating orphan rules
#[derive(Debug, Clone)]
pub struct TagsWrapper(pub Vec<String>);

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
                .entry(metric.name.clone())
                .or_insert_with(Vec::new)
                .push(metric);
        }

        grouped
    }

    fn print_metric_group(metric_name: &str, metrics: &[&ReportMetric]) {
        println!("\n{} ({})", metric_name, metrics.len());

        if metrics.len() == 1 {
            Self::print_single_metric(metrics[0]);
        } else {
            Self::print_multiple_metrics(metrics);
        }
    }

    fn print_single_metric(metric: &ReportMetric) {
        println!("  Value: {:.3}", metric.value);
        println!("  Kind: {:?}", metric.kind);

        if !metric.tags.is_empty() {
            println!("  Tags:");
            for (key, value) in &metric.tags {
                println!("    {key}: {value}");
            }
        }
    }

    fn print_multiple_metrics(metrics: &[&ReportMetric]) {
        let table_data: Vec<MetricTableRow> = metrics
            .iter()
            .enumerate()
            .map(|(i, metric)| MetricTableRow {
                index: i + 1,
                value: format!("{:.3}", metric.value),
                tags: TagsWrapper(
                    metric
                        .tags
                        .iter()
                        .map(|(k, v)| format!("{k}: {v}"))
                        .collect(),
                ),
            })
            .collect();

        let mut table = Table::new(table_data);
        table.with(Style::modern());
        println!("{table}");
    }
}
