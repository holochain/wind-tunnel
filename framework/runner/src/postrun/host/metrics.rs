use std::{fmt, str::FromStr};

use influxive_core::DataType;
use serde::{Deserialize, Serialize};
use wind_tunnel_instruments::prelude::ReportMetric;

/// Name for [`HostMetrics`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostMetricsName {
    Cpu,
    Disk,
    Diskio,
    Kernel,
    Mem,
    Net,
    Netstat,
    Processes,
    Swap,
    System,
}

impl fmt::Display for HostMetricsName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostMetricsName::Cpu => write!(f, "cpu"),
            HostMetricsName::Disk => write!(f, "disk"),
            HostMetricsName::Diskio => write!(f, "diskio"),
            HostMetricsName::Kernel => write!(f, "kernel"),
            HostMetricsName::Mem => write!(f, "mem"),
            HostMetricsName::Net => write!(f, "net"),
            HostMetricsName::Netstat => write!(f, "netstat"),
            HostMetricsName::Processes => write!(f, "processes"),
            HostMetricsName::Swap => write!(f, "swap"),
            HostMetricsName::System => write!(f, "system"),
        }
    }
}

/// Wrapper of metrics for the host system.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct HostMetrics {
    pub fields: serde_json::Value,
    pub name: HostMetricsName,
    pub tags: serde_json::Value,
    pub timestamp: u64,
}

impl FromStr for HostMetrics {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl HostMetrics {
    /// Returns the associated [`ReportMetric`] for this host metrics.
    pub fn report_metric(&self) -> ReportMetric {
        let mut metric = ReportMetric::new(&format!("wt.host.{}", self.name));
        // add fields
        for (key, value) in self.fields.as_object().unwrap_or(&serde_json::Map::new()) {
            metric = metric.with_field(key.clone(), Self::value_to_data_type(value));
        }
        // add tags
        for (key, value) in self.tags.as_object().unwrap_or(&serde_json::Map::new()) {
            metric = metric.with_tag(key.clone(), Self::value_to_data_type(value));
        }

        metric
    }

    /// Converts a [`serde_json::Value`] to a [`DataType`].
    fn value_to_data_type(value: &serde_json::Value) -> DataType {
        match value {
            serde_json::Value::Bool(b) => DataType::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    DataType::I64(i)
                } else if let Some(f) = n.as_f64() {
                    DataType::F64(f)
                } else {
                    DataType::String(n.to_string().into())
                }
            }
            serde_json::Value::String(s) => DataType::String(s.to_string().into()),
            _ => DataType::String(value.to_string().into()),
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{io::BufRead as _, path::Path};

    use super::*;

    const JSON_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/host_metrics.json");
    const METRIC_STR: &str = r#"{"fields":{"run_id":"123","active":7981232128,"available":26517172224,"available_percent":81.04687482395181,"buffered":10010624,"cached":6127505408,"commit_limit":47816503296,"committed_as":35352698880,"dirty":30318592,"free":21104721920,"high_free":0,"high_total":0,"huge_page_size":2097152,"huge_pages_free":0,"huge_pages_total":0,"inactive":2375311360,"low_free":0,"low_total":0,"mapped":1431642112,"page_tables":69566464,"shared":253870080,"slab":546750464,"sreclaimable":272355328,"sunreclaim":274395136,"swap_cached":0,"swap_free":31457345536,"swap_total":31457345536,"total":32718315520,"used":5476077568,"used_percent":16.737040036956035,"vmalloc_chunk":0,"vmalloc_total":35184372087808,"vmalloc_used":263839744,"write_back":0,"write_back_tmp":0},"name":"mem","tags":{"host":"msi-manjaro"},"timestamp":1753861050}"#;

    fn load_metrics_from_json() -> anyhow::Result<Vec<HostMetrics>> {
        let mut metrics = Vec::new();
        let file = std::fs::File::open(Path::new(JSON_PATH))?;
        let reader = std::io::BufReader::new(file);
        let lines = reader.lines();

        for line in lines {
            let metric: HostMetrics = serde_json::from_str(&(line?))?;
            metrics.push(metric);
        }
        Ok(metrics)
    }

    #[test]
    fn test_should_parse_metrics_from_str() {
        let metrics: HostMetrics = METRIC_STR.parse().expect("Failed to parse metrics");
        assert_eq!(metrics.name, HostMetricsName::Mem);
    }

    #[test]
    fn test_should_parse_host_metrics_json() {
        let metrics = load_metrics_from_json().expect("Failed to parse host metrics");

        // should have 487 lines
        assert_eq!(metrics.len(), 487);
        let first_metric = &metrics[0];
        assert_eq!(first_metric.name, HostMetricsName::Mem);
        let active = first_metric
            .fields
            .get("active")
            .and_then(|v| v.as_u64())
            .unwrap();

        assert_eq!(active, 17853476864);
        // should have default tags
        let host = first_metric
            .tags
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(host, "msi-manjaro");
    }

    #[test]
    fn test_should_report_host_metrics() {
        let metrics = load_metrics_from_json().expect("Failed to parse host metrics");

        let first_metric = &metrics[0];
        let report_metric = first_metric.report_metric();
        // verify tags (should have only host tag)
        let (tag_name, _) = &report_metric.tags[0];
        assert_eq!(tag_name.clone().into_string(), "host");
        // verify fields (should have active field)
        let (field_name, _field_value) = &report_metric.fields[0];
        assert_eq!(field_name.clone().into_string(), "active");
    }
}
