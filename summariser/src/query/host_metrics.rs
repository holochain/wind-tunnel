use std::time::Duration;
use strum::VariantNames;
use strum_macros::EnumString;

/// A trait to describe how to source table data from InfluxDB for a type.
pub trait InfluxSourced {
    /// A list of key-value tag pairs to filter by when sourcing data for this type.
    ///
    /// Note that tags are indexed in InfluxDB, where fields are not. Please check the docs before
    /// adding tags to this list and consider filtering locally if you need to work with
    /// non-indexed values.
    fn filter_tags(&self) -> Vec<(&str, String)> {
        Vec::with_capacity(0)
    }

    /// A list of fields or tags to select for this type.
    ///
    /// These will become the columns in the resulting table.
    fn select(&self) -> &[&str];
}

/// Host metric measurement.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostMetricMeasurement {
    /// Telegraf CPU metrics -> https://docs.influxdata.com/telegraf/v1/input-plugins/cpu/
    Cpu(CpuFieldSet),
    /// Telegraf Memory metrics -> https://docs.influxdata.com/telegraf/v1/input-plugins/mem/
    Mem(MemFieldSet),
    /// Telegraf Network metrics -> https://docs.influxdata.com/telegraf/v1/input-plugins/net/
    Net(NetFieldSet),
    /// Telegraf Disk metrics -> https://docs.influxdata.com/telegraf/v1/input-plugins/disk/
    Disk(DiskFieldSet),
    /// Telegraf Disk IO metrics -> https://docs.influxdata.com/telegraf/v1/input-plugins/diskio/
    DiskIo(DiskIoFieldSet),
    /// Telegraf System metrics -> https://docs.influxdata.com/telegraf/v1/input-plugins/system/
    System(SystemFieldSet),
    /// Linux Pressure Stall Information metrics -> https://docs.influxdata.com/telegraf/v1/input-plugins/kernel
    Pressure(PressureFieldSet),
    /// Process metrics sourced from telegraf's `inputs.procstat` plugin -> https://docs.influxdata.com/telegraf/v1/input-plugins/procstat/
    Procstat(ProcstatFieldSet),
}

impl HostMetricMeasurement {
    /// Get the measurement name for this measurement category.
    ///
    /// This value can be used as a table name in InfluxDB queries.
    pub fn measurement(&self) -> &'static str {
        match self {
            HostMetricMeasurement::Cpu(_) => "cpu",
            HostMetricMeasurement::Mem(_) => "mem",
            HostMetricMeasurement::Net(_) => "net",
            HostMetricMeasurement::Disk(_) => "disk",
            HostMetricMeasurement::DiskIo(_) => "diskio",
            HostMetricMeasurement::System(_) => "system",
            HostMetricMeasurement::Pressure(_) => "pressure",
            HostMetricMeasurement::Procstat(_) => "procstat",
        }
    }
}

impl InfluxSourced for HostMetricMeasurement {
    fn filter_tags(&self) -> Vec<(&str, String)> {
        match self {
            HostMetricMeasurement::Cpu(f) => f.filter_tags(),
            HostMetricMeasurement::Mem(f) => f.filter_tags(),
            HostMetricMeasurement::Net(f) => f.filter_tags(),
            HostMetricMeasurement::Disk(f) => f.filter_tags(),
            HostMetricMeasurement::DiskIo(f) => f.filter_tags(),
            HostMetricMeasurement::System(f) => f.filter_tags(),
            HostMetricMeasurement::Pressure(f) => f.filter_tags(),
            HostMetricMeasurement::Procstat(f) => f.filter_tags(),
        }
    }

    fn select(&self) -> &[&str] {
        match self {
            HostMetricMeasurement::Cpu(f) => f.select(),
            HostMetricMeasurement::Mem(f) => f.select(),
            HostMetricMeasurement::Net(f) => f.select(),
            HostMetricMeasurement::Disk(f) => f.select(),
            HostMetricMeasurement::DiskIo(f) => f.select(),
            HostMetricMeasurement::System(f) => f.select(),
            HostMetricMeasurement::Pressure(f) => f.select(),
            HostMetricMeasurement::Procstat(f) => f.select(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CpuFieldSet {
    Default,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    strum_macros::VariantNames,
    strum_macros::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum CpuField {
    Host,
    UsageUser,
    UsageSystem,
}

impl InfluxSourced for CpuFieldSet {
    fn filter_tags(&self) -> Vec<(&str, String)> {
        match self {
            CpuFieldSet::Default => vec![("cpu", "cpu-total".to_string())],
        }
    }

    fn select(&self) -> &[&str] {
        match self {
            CpuFieldSet::Default => CpuField::VARIANTS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemFieldSet {
    Default,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    strum_macros::VariantNames,
    strum_macros::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum MemField {
    Host,
    UsedPercent,
    AvailablePercent,
    Used,
    Total,
    Available,
    SwapFree,
    SwapTotal,
}

impl InfluxSourced for MemFieldSet {
    fn select(&self) -> &[&str] {
        match self {
            MemFieldSet::Default => MemField::VARIANTS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetFieldSet {
    Default,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    strum_macros::VariantNames,
    strum_macros::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum NetField {
    Host,
    Interface,
    BytesRecv,
    BytesSent,
    PacketsRecv,
    PacketsSent,
}

impl InfluxSourced for NetFieldSet {
    fn select(&self) -> &[&str] {
        match self {
            NetFieldSet::Default => NetField::VARIANTS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiskFieldSet {
    Default,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    strum_macros::VariantNames,
    strum_macros::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum DiskField {
    Host,
    Path,
    UsedPercent,
}

impl InfluxSourced for DiskFieldSet {
    fn select(&self) -> &[&str] {
        match self {
            DiskFieldSet::Default => DiskField::VARIANTS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiskIoFieldSet {
    Default,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    strum_macros::VariantNames,
    strum_macros::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum DiskIoField {
    Host,
    Path,
    Name,
    ReadBytes,
    WriteBytes,
}

impl InfluxSourced for DiskIoFieldSet {
    fn select(&self) -> &[&str] {
        match self {
            DiskIoFieldSet::Default => DiskIoField::VARIANTS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemFieldSet {
    Default,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    strum_macros::VariantNames,
    strum_macros::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum SystemField {
    Host,
    Load1,
    Load5,
    Load15,
    NCpus,
}

impl InfluxSourced for SystemFieldSet {
    fn select(&self) -> &[&str] {
        match self {
            SystemFieldSet::Default => SystemField::VARIANTS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PressureFieldSet {
    CpuSome,
    MemSome,
    MemFull,
    IoSome,
    IoFull,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    strum_macros::Display,
    EnumString,
    strum_macros::VariantNames,
    strum_macros::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum PressureField {
    Avg10,
    Avg60,
    Avg300,
}

impl InfluxSourced for PressureFieldSet {
    fn filter_tags(&self) -> Vec<(&str, String)> {
        match self {
            PressureFieldSet::CpuSome => vec![
                ("resource", "cpu".to_string()),
                ("type", "some".to_string()),
            ],
            PressureFieldSet::MemSome => vec![
                ("resource", "memory".to_string()),
                ("type", "some".to_string()),
            ],
            PressureFieldSet::MemFull => vec![
                ("resource", "memory".to_string()),
                ("type", "full".to_string()),
            ],
            PressureFieldSet::IoSome => {
                vec![("resource", "io".to_string()), ("type", "some".to_string())]
            }
            PressureFieldSet::IoFull => {
                vec![("resource", "io".to_string()), ("type", "full".to_string())]
            }
        }
    }

    fn select(&self) -> &[&str] {
        // All pressure variants use the same fields: avg10, avg60, avg300
        PressureField::VARIANTS
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProcstatFieldSet {
    Default { pattern: String },
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    EnumString,
    strum_macros::VariantNames,
    strum_macros::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum ProcstatField {
    /// Hostname tag — used for per-host cpu_usage normalization
    Host,
    /// CPU usage percentage for the process (unbounded: 100% per core)
    CpuUsage,
    /// Proportional set size in bytes
    MemoryPss,
    /// Number of threads
    NumThreads,
    /// Number of open file descriptors
    NumFds,
}

impl InfluxSourced for ProcstatFieldSet {
    fn filter_tags(&self) -> Vec<(&str, String)> {
        match self {
            ProcstatFieldSet::Default { pattern } => vec![("pattern", pattern.clone())],
        }
    }

    fn select(&self) -> &[&str] {
        match self {
            ProcstatFieldSet::Default { .. } => ProcstatField::VARIANTS,
        }
    }
}

/// Filter to select the host metrics
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SelectFilter {
    /// Select ONLY by run_id
    RunId(String),
    /// Select by the time interval
    TimeInterval {
        /// UNIX epoch the test started at
        started_at: i64,
        /// The test duration
        duration: Duration,
        /// Run id
        run_id: String,
    },
}
