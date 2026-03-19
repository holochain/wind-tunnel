use std::time::Duration;
use strum::VariantNames;
use strum_macros::EnumString;

/// How to aggregate a field when downsampling via GROUP BY time().
#[derive(Debug, Clone, Copy)]
pub enum FieldAggregation {
    /// Use MEAN() — appropriate for gauges (CPU %, load averages, etc.)
    Mean,
    /// Use LAST() — appropriate for monotonic counters (bytes_recv, read_bytes, etc.)
    Last,
    /// This is a tag — include in GROUP BY, not in SELECT aggregation.
    Tag,
}

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

    /// How each selected field should be aggregated when downsampling.
    /// Must return one entry per item in `select()`, in the same order.
    /// Default: all fields use Mean (backward-compatible).
    fn aggregations(&self) -> Vec<FieldAggregation> {
        vec![FieldAggregation::Mean; self.select().len()]
    }

    /// The time interval for GROUP BY time() downsampling.
    /// None means no downsampling (fetch raw data).
    fn downsample_interval(&self) -> Option<&'static str> {
        None
    }
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

    fn aggregations(&self) -> Vec<FieldAggregation> {
        match self {
            HostMetricMeasurement::Cpu(f) => f.aggregations(),
            HostMetricMeasurement::Mem(f) => f.aggregations(),
            HostMetricMeasurement::Net(f) => f.aggregations(),
            HostMetricMeasurement::Disk(f) => f.aggregations(),
            HostMetricMeasurement::DiskIo(f) => f.aggregations(),
            HostMetricMeasurement::System(f) => f.aggregations(),
            HostMetricMeasurement::Pressure(f) => f.aggregations(),
            HostMetricMeasurement::Procstat(f) => f.aggregations(),
        }
    }

    fn downsample_interval(&self) -> Option<&'static str> {
        match self {
            HostMetricMeasurement::Cpu(f) => f.downsample_interval(),
            HostMetricMeasurement::Mem(f) => f.downsample_interval(),
            HostMetricMeasurement::Net(f) => f.downsample_interval(),
            HostMetricMeasurement::Disk(f) => f.downsample_interval(),
            HostMetricMeasurement::DiskIo(f) => f.downsample_interval(),
            HostMetricMeasurement::System(f) => f.downsample_interval(),
            HostMetricMeasurement::Pressure(f) => f.downsample_interval(),
            HostMetricMeasurement::Procstat(f) => f.downsample_interval(),
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

    fn aggregations(&self) -> Vec<FieldAggregation> {
        match self {
            CpuFieldSet::Default => vec![
                FieldAggregation::Tag,  // host
                FieldAggregation::Mean, // usage_user
                FieldAggregation::Mean, // usage_system
            ],
        }
    }

    fn downsample_interval(&self) -> Option<&'static str> {
        Some("30s")
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

    fn aggregations(&self) -> Vec<FieldAggregation> {
        match self {
            MemFieldSet::Default => vec![
                FieldAggregation::Tag,  // host
                FieldAggregation::Mean, // used_percent
                FieldAggregation::Mean, // available_percent
                FieldAggregation::Last, // used
                FieldAggregation::Last, // total
                FieldAggregation::Last, // available
                FieldAggregation::Last, // swap_free
                FieldAggregation::Last, // swap_total
            ],
        }
    }

    fn downsample_interval(&self) -> Option<&'static str> {
        Some("30s")
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

    fn aggregations(&self) -> Vec<FieldAggregation> {
        match self {
            NetFieldSet::Default => vec![
                FieldAggregation::Tag,  // host
                FieldAggregation::Tag,  // interface
                FieldAggregation::Last, // bytes_recv (counter)
                FieldAggregation::Last, // bytes_sent (counter)
                FieldAggregation::Last, // packets_recv (counter)
                FieldAggregation::Last, // packets_sent (counter)
            ],
        }
    }

    fn downsample_interval(&self) -> Option<&'static str> {
        Some("30s")
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

    fn aggregations(&self) -> Vec<FieldAggregation> {
        match self {
            DiskFieldSet::Default => vec![
                FieldAggregation::Tag,  // host
                FieldAggregation::Tag,  // path
                FieldAggregation::Mean, // used_percent
            ],
        }
    }

    fn downsample_interval(&self) -> Option<&'static str> {
        Some("30s")
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

    fn aggregations(&self) -> Vec<FieldAggregation> {
        match self {
            DiskIoFieldSet::Default => vec![
                FieldAggregation::Tag,  // host
                FieldAggregation::Tag,  // path
                FieldAggregation::Tag,  // name
                FieldAggregation::Last, // read_bytes (counter)
                FieldAggregation::Last, // write_bytes (counter)
            ],
        }
    }

    fn downsample_interval(&self) -> Option<&'static str> {
        Some("30s")
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

    fn aggregations(&self) -> Vec<FieldAggregation> {
        match self {
            SystemFieldSet::Default => vec![
                FieldAggregation::Tag,  // host
                FieldAggregation::Mean, // load1
                FieldAggregation::Mean, // load5
                FieldAggregation::Mean, // load15
                FieldAggregation::Last, // n_cpus (constant per host)
            ],
        }
    }

    fn downsample_interval(&self) -> Option<&'static str> {
        Some("30s")
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

    fn aggregations(&self) -> Vec<FieldAggregation> {
        vec![
            FieldAggregation::Mean,
            FieldAggregation::Mean,
            FieldAggregation::Mean,
        ]
    }

    fn downsample_interval(&self) -> Option<&'static str> {
        Some("30s")
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

    fn aggregations(&self) -> Vec<FieldAggregation> {
        match self {
            ProcstatFieldSet::Default { .. } => vec![
                FieldAggregation::Tag,  // host
                FieldAggregation::Mean, // cpu_usage
                FieldAggregation::Last, // memory_pss
                FieldAggregation::Mean, // num_threads
                FieldAggregation::Mean, // num_fds
            ],
        }
    }

    fn downsample_interval(&self) -> Option<&'static str> {
        Some("30s")
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
