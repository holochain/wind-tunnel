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
    fn filter_tags(&self) -> &[(&str, &str)] {
        &[]
    }

    /// A list of fields or tags to select for this type.
    ///
    /// These will become the columns in the resulting table.
    fn select(&self) -> &[&str];
}

/// Host metric measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Holochain process metrics sourced from telegraf's `inputs.procstat` plugin with `pattern = "holochain"` -> https://docs.influxdata.com/telegraf/v1/input-plugins/procstat/
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
    fn filter_tags(&self) -> &[(&str, &str)] {
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
    UsageUser,
    UsageSystem,
}

impl InfluxSourced for CpuFieldSet {
    fn select(&self) -> &[&str] {
        match self {
            CpuFieldSet::Default => &CpuField::VARIANTS,
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
            MemFieldSet::Default => &MemField::VARIANTS,
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
            NetFieldSet::Default => &NetField::VARIANTS,
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
#[allow(dead_code)]
pub enum DiskField {
    Path,
    UsedPercent,
}

impl InfluxSourced for DiskFieldSet {
    fn select(&self) -> &[&str] {
        match self {
            DiskFieldSet::Default => &DiskField::VARIANTS,
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
    Path,
    Name,
    ReadBytes,
    WriteBytes,
}

impl InfluxSourced for DiskIoFieldSet {
    fn select(&self) -> &[&str] {
        match self {
            DiskIoFieldSet::Default => &DiskIoField::VARIANTS,
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
            SystemFieldSet::Default => &SystemField::VARIANTS,
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
    fn filter_tags(&self) -> &[(&str, &str)] {
        match self {
            PressureFieldSet::CpuSome => &[("resource", "cpu"), ("type", "some")],
            PressureFieldSet::MemSome => &[("resource", "memory"), ("type", "some")],
            PressureFieldSet::MemFull => &[("resource", "memory"), ("type", "full")],
            PressureFieldSet::IoSome => &[("resource", "io"), ("type", "some")],
            PressureFieldSet::IoFull => &[("resource", "io"), ("type", "full")],
        }
    }

    fn select(&self) -> &[&str] {
        // All pressure variants use the same fields: avg10, avg60, avg300
        &PressureField::VARIANTS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcstatFieldSet {
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
pub enum ProcstatField {
    /// CPU usage percentage for the process
    CpuUsage,
    /// Proportional set size in bytes
    MemoryPss,
    /// Number of threads
    NumThreads,
    /// Number of open file descriptors
    NumFds,
}

impl InfluxSourced for ProcstatFieldSet {
    fn filter_tags(&self) -> &[(&str, &str)] {
        match self {
            ProcstatFieldSet::Default => &[("pattern", "holochain")],
        }
    }

    fn select(&self) -> &[&str] {
        match self {
            ProcstatFieldSet::Default => &ProcstatField::VARIANTS,
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
