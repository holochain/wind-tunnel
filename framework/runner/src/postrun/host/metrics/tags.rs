use serde::{Deserialize, Serialize};
use wind_tunnel_instruments::prelude::ReportMetric;

/// Trait for reporting tags of host metrics.
pub trait ReportTags {
    /// Returns the associated [`ReportMetric`] for this host metrics tags.
    fn report_tags(&self, metric: ReportMetric) -> ReportMetric;
}

/// [`super::HostMetrics`] tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum HostMetricsTags {
    /// Tags for [`super::HostMetricsName::Cpu`].
    Cpu(CpuMetricsTags),
    /// Tags for those without specific tags.
    Default(DefaultHostMetricsTags),
    /// Tags for [`super::HostMetricsName::Disk`].
    Disk(DiskMetricsTags),
    /// Tags for [`super::HostMetricsName::Diskio`].
    Diskio(DiskioMetricsTags),
    /// Tags for [`super::HostMetricsName::Net`].
    Net(NetHostMetricsTags),
}

impl ReportTags for HostMetricsTags {
    fn report_tags(&self, metric: ReportMetric) -> ReportMetric {
        match self {
            Self::Default(tags) => tags.report_tags(metric),
            Self::Cpu(tags) => tags.report_tags(metric),
            Self::Disk(tags) => tags.report_tags(metric),
            Self::Diskio(tags) => tags.report_tags(metric),
            Self::Net(tags) => tags.report_tags(metric),
        }
    }
}

/// Default tags for [`super::HostMetrics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DefaultHostMetricsTags {
    pub host: String,
}

impl ReportTags for DefaultHostMetricsTags {
    fn report_tags(&self, metric: ReportMetric) -> ReportMetric {
        metric.with_tag("host", self.host.clone())
    }
}

/// Tags for [`super::HostMetricsName::Cpu`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CpuMetricsTags {
    pub cpu: String,
    pub host: String,
}

impl ReportTags for CpuMetricsTags {
    fn report_tags(&self, metric: ReportMetric) -> ReportMetric {
        metric
            .with_tag("cpu", self.cpu.clone())
            .with_tag("host", self.host.clone())
    }
}

/// Tags for [`super::HostMetricsName::Disk`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DiskMetricsTags {
    pub device: String,
    pub fstype: String,
    pub host: String,
    pub label: Option<String>,
    pub mode: String,
    pub path: String,
}

impl ReportTags for DiskMetricsTags {
    fn report_tags(&self, metric: ReportMetric) -> ReportMetric {
        let metric = metric
            .with_tag("device", self.device.clone())
            .with_tag("fstype", self.fstype.clone())
            .with_tag("host", self.host.clone())
            .with_tag("mode", self.mode.clone())
            .with_tag("path", self.path.clone());
        if let Some(label) = &self.label {
            metric.with_tag("label", label.clone())
        } else {
            metric
        }
    }
}

/// Tags for [`super::HostMetricsName::Diskio`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DiskioMetricsTags {
    pub host: String,
    pub name: String,
    pub wwid: Option<String>,
}

impl ReportTags for DiskioMetricsTags {
    fn report_tags(&self, metric: ReportMetric) -> ReportMetric {
        let metric = metric
            .with_tag("host", self.host.clone())
            .with_tag("name", self.name.clone());
        if let Some(wwid) = &self.wwid {
            metric.with_tag("wwid", wwid.clone())
        } else {
            metric
        }
    }
}

/// Tags for [`super::HostMetricsName::Net`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NetHostMetricsTags {
    pub host: String,
    pub interface: String,
}

impl ReportTags for NetHostMetricsTags {
    fn report_tags(&self, metric: ReportMetric) -> ReportMetric {
        metric
            .with_tag("host", self.host.clone())
            .with_tag("interface", self.interface.clone())
    }
}
