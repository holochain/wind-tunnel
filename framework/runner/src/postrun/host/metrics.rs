mod fields;
mod tags;

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use wind_tunnel_instruments::prelude::ReportMetric;

pub use self::fields::HostMetricsFields;
use self::fields::ReportFields as _;
pub use self::tags::HostMetricsTags;
use self::tags::ReportTags as _;

const DEFAULT_RUN_ID: &str = "N/A";

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
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct HostMetrics {
    pub fields: HostMetricsFields,
    pub name: HostMetricsName,
    pub run_id: Option<String>,
    pub tags: HostMetricsTags,
    pub timestamp: u64,
}

impl FromStr for HostMetrics {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl<'de> Deserialize<'de> for HostMetrics {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize the host metrics from a JSON object with a generic structure.
        #[derive(Deserialize)]
        struct HostMetricsRaw {
            name: HostMetricsName,
            timestamp: u64,
            fields: serde_json::Value,
            tags: serde_json::Value,
        }

        let HostMetricsRaw {
            fields,
            name,
            tags,
            timestamp,
        } = HostMetricsRaw::deserialize(deserializer)?;

        // get run_id from fields
        let run_id = fields.get("run_id").and_then(|v| v.as_str()).and_then(|s| {
            if s == DEFAULT_RUN_ID {
                None
            } else {
                Some(s.to_string())
            }
        });

        let fields = match name {
            HostMetricsName::Cpu => HostMetricsFields::Cpu(
                serde_json::from_value(fields).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::Disk => HostMetricsFields::Disk(
                serde_json::from_value(fields).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::Diskio => HostMetricsFields::Diskio(
                serde_json::from_value(fields).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::Kernel => HostMetricsFields::Kernel(
                serde_json::from_value(fields).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::Mem => HostMetricsFields::Mem(
                serde_json::from_value(fields).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::Net => HostMetricsFields::Net(
                serde_json::from_value(fields).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::Netstat => HostMetricsFields::Netstat(
                serde_json::from_value(fields).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::Processes => HostMetricsFields::Processes(
                serde_json::from_value(fields).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::Swap => HostMetricsFields::Swap(
                serde_json::from_value(fields).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::System => HostMetricsFields::System(
                serde_json::from_value(fields).map_err(serde::de::Error::custom)?,
            ),
        };

        let tags = match name {
            HostMetricsName::Cpu => HostMetricsTags::Cpu(
                serde_json::from_value(tags).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::Disk => HostMetricsTags::Disk(
                serde_json::from_value(tags).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::Diskio => HostMetricsTags::Diskio(
                serde_json::from_value(tags).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::Kernel
            | HostMetricsName::Mem
            | HostMetricsName::Netstat
            | HostMetricsName::Processes
            | HostMetricsName::Swap
            | HostMetricsName::System => HostMetricsTags::Default(
                serde_json::from_value(tags).map_err(serde::de::Error::custom)?,
            ),
            HostMetricsName::Net => HostMetricsTags::Net(
                serde_json::from_value(tags).map_err(serde::de::Error::custom)?,
            ),
        };

        Ok(HostMetrics {
            fields,
            name,
            run_id,
            tags,
            timestamp,
        })
    }
}

impl HostMetrics {
    /// Returns the associated [`ReportMetric`] for this host metrics.
    pub fn report_metric(&self) -> ReportMetric {
        let metric = ReportMetric::new(&format!("wt.host.{}", self.name));
        // if run_id is set, add it to the metric
        let metric = if let Some(run_id) = &self.run_id {
            metric.with_field("run_id", run_id.clone())
        } else {
            metric
        };
        // add fields based on the fields of the host metrics
        let metric = self.fields.report_fields(metric);
        // add tags based on the tags of the host metrics
        self.tags.report_tags(metric)
    }
}

#[cfg(test)]
mod tests {

    use std::{io::BufRead as _, path::Path};

    use super::*;

    const JSON_PATH: &str = "tests/host_metrics.json";
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
        assert!(matches!(metrics.fields, HostMetricsFields::Mem(_)));
        assert!(matches!(metrics.tags, HostMetricsTags::Default(_)));
    }

    #[test]
    fn test_should_parse_host_metrics_json() {
        let metrics = load_metrics_from_json().expect("Failed to parse host metrics");

        // should have 487 lines
        assert_eq!(metrics.len(), 487);
        let first_metric = &metrics[0];
        assert_eq!(first_metric.name, HostMetricsName::Mem);
        let HostMetricsFields::Mem(mem_metrics) = &first_metric.fields else {
            panic!("Expected MemMetrics, found {:?}", first_metric.fields);
        };
        assert!(first_metric.run_id.is_none());
        assert_eq!(mem_metrics.active, 17853476864);
        // should have default tags
        let HostMetricsTags::Default(default_tags) = &first_metric.tags else {
            panic!("Expected DefaultTags, found {:?}", first_metric.tags);
        };
        assert_eq!(default_tags.host, "msi-manjaro");

        // verify all fields and tags are decoded
        let cpu_metric = metrics
            .iter()
            .find(|m| m.name == HostMetricsName::Cpu)
            .expect("No CPU metrics found");
        assert!(matches!(cpu_metric.tags, HostMetricsTags::Cpu(_)));
        assert!(matches!(cpu_metric.fields, HostMetricsFields::Cpu(_)));
        let disk_metric = metrics
            .iter()
            .find(|m| m.name == HostMetricsName::Disk)
            .expect("No Disk metrics found");
        assert!(matches!(disk_metric.tags, HostMetricsTags::Disk(_)));
        assert!(matches!(disk_metric.fields, HostMetricsFields::Disk(_)));
        let diskio_metric = metrics
            .iter()
            .find(|m| m.name == HostMetricsName::Diskio)
            .expect("No DiskIO metrics found");
        assert!(matches!(diskio_metric.tags, HostMetricsTags::Diskio(_)));
        assert!(matches!(diskio_metric.fields, HostMetricsFields::Diskio(_)));
        let kernel_metric = metrics
            .iter()
            .find(|m| m.name == HostMetricsName::Kernel)
            .expect("No Kernel metrics found");
        assert!(matches!(kernel_metric.tags, HostMetricsTags::Default(_)));
        assert!(matches!(kernel_metric.fields, HostMetricsFields::Kernel(_)));
        let net_metric = metrics
            .iter()
            .find(|m| m.name == HostMetricsName::Net)
            .expect("No Net metrics found");
        assert!(matches!(net_metric.tags, HostMetricsTags::Net(_)));
        assert!(matches!(net_metric.fields, HostMetricsFields::Net(_)));
        let netstat_metric = metrics
            .iter()
            .find(|m| m.name == HostMetricsName::Netstat)
            .expect("No Netstat metrics found");
        assert!(matches!(netstat_metric.tags, HostMetricsTags::Default(_)));
        assert!(matches!(
            netstat_metric.fields,
            HostMetricsFields::Netstat(_)
        ));
        let processes_metric = metrics
            .iter()
            .find(|m| m.name == HostMetricsName::Processes)
            .expect("No Processes metrics found");
        assert!(matches!(processes_metric.tags, HostMetricsTags::Default(_)));

        assert!(matches!(
            processes_metric.fields,
            HostMetricsFields::Processes(_)
        ));
        let swap_metric = metrics
            .iter()
            .find(|m| m.name == HostMetricsName::Swap)
            .expect("No Swap metrics found");
        assert!(matches!(swap_metric.tags, HostMetricsTags::Default(_)));
        assert!(matches!(swap_metric.fields, HostMetricsFields::Swap(_)));
        let system_metric = metrics
            .iter()
            .find(|m| m.name == HostMetricsName::System)
            .expect("No System metrics found");
        assert!(matches!(system_metric.tags, HostMetricsTags::Default(_)));
        assert!(matches!(system_metric.fields, HostMetricsFields::System(_)));
    }

    #[test]
    fn test_host_metrics_should_have_run_id() {
        let metrics: HostMetrics = METRIC_STR.parse().expect("Failed to parse metrics");
        assert_eq!(metrics.run_id, Some("123".to_string()));
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

    #[test]
    fn test_should_report_host_metrics_with_run_id() {
        let metrics = load_metrics_from_json().expect("Failed to parse host metrics");

        let metric_with_run_id = &metrics[1];
        let report_metric = metric_with_run_id.report_metric();
        // verify tags (should have only host tag)
        let (tag_name, _) = &report_metric.tags[0];
        assert_eq!(tag_name.clone().into_string(), "host");
        // verify fields (should have active field)
        let (field_name, _field_value) = &report_metric.fields[0];
        assert_eq!(field_name.clone().into_string(), "run_id");
    }
}
