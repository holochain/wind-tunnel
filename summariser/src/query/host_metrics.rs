use std::fmt;

/// Kind of host metric measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostMetricMeasurement {
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

impl HostMetricMeasurement {
    /// Iter all [`HostMetricMeasurement`] variants.
    pub fn all() -> impl Iterator<Item = Self> {
        [
            Self::Cpu,
            Self::Disk,
            Self::Diskio,
            Self::Kernel,
            Self::Mem,
            Self::Net,
            Self::Netstat,
            Self::Processes,
            Self::Swap,
            Self::System,
        ]
        .into_iter()
    }
}

impl fmt::Display for HostMetricMeasurement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostMetricMeasurement::Cpu => write!(f, "cpu"),
            HostMetricMeasurement::Disk => write!(f, "disk"),
            HostMetricMeasurement::Diskio => write!(f, "diskio"),
            HostMetricMeasurement::Kernel => write!(f, "kernel"),
            HostMetricMeasurement::Mem => write!(f, "mem"),
            HostMetricMeasurement::Net => write!(f, "net"),
            HostMetricMeasurement::Netstat => write!(f, "netstat"),
            HostMetricMeasurement::Processes => write!(f, "processes"),
            HostMetricMeasurement::Swap => write!(f, "swap"),
            HostMetricMeasurement::System => write!(f, "system"),
        }
    }
}
/// Get SELECT query for host metrics.
///
/// Given a [`HostMetricMeasurement`], it returns the select for all the fields associated to the measurement collected.
#[inline(always)]
pub fn host_metrics_select(measurement: HostMetricMeasurement, run_id: &str) -> String {
    format!(
        r#"SELECT * FROM
        "windtunnel"."autogen"."{measurement}"
        WHERE run_id = '{run_id}'
    "#,
        run_id = run_id
    )
}
