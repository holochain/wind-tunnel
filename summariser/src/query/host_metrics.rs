use std::{fmt, time::Duration};

pub const TAG_INTERFACE: &str = "interface";

/// A trait to return values to select for a type.
pub trait Values {
    /// Get the values to select for this type.
    fn values(&self) -> &[&'static str];
}

/// A trait to return the column name for a type.
pub trait Column {
    /// Get the column name for this type.
    fn column(&self) -> &'static str;
}

/// Host metric field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostMetricField {
    Cpu(CpuField),
    Mem(MemField),
    Net(NetField),
}

impl fmt::Display for HostMetricField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{measurement}.{column}",
            measurement = self.measurement(),
            column = self.column()
        )
    }
}

impl Values for HostMetricField {
    fn values(&self) -> &[&'static str] {
        match self {
            HostMetricField::Cpu(f) => f.values(),
            HostMetricField::Mem(f) => f.values(),
            HostMetricField::Net(f) => f.values(),
        }
    }
}

impl Column for HostMetricField {
    fn column(&self) -> &'static str {
        match self {
            HostMetricField::Cpu(f) => f.column(),
            HostMetricField::Mem(f) => f.column(),
            HostMetricField::Net(f) => f.column(),
        }
    }
}

impl HostMetricField {
    /// Get the measurement name for this field category to be used as the table name
    pub fn measurement(&self) -> &'static str {
        match self {
            HostMetricField::Cpu(_) => "cpu",
            HostMetricField::Mem(_) => "mem",
            HostMetricField::Net(_) => "net",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CpuField {
    UsageUser,
    UsageSystem,
}

impl Values for CpuField {
    fn values(&self) -> &[&'static str] {
        match self {
            CpuField::UsageUser => &["usage_user"],
            CpuField::UsageSystem => &["usage_system"],
        }
    }
}

impl Column for CpuField {
    fn column(&self) -> &'static str {
        match self {
            CpuField::UsageUser => "usage_user",
            CpuField::UsageSystem => "usage_system",
        }
    }
}

impl fmt::Display for CpuField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.column())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemField {
    Active,
    Available,
    AvailablePercent,
    Free,
    Inactive,
    SwapFree,
    SwapTotal,
    Total,
    Used,
    UsedPercent,
}

impl Values for MemField {
    fn values(&self) -> &[&'static str] {
        match self {
            MemField::Active => &["active"],
            MemField::Available => &["available"],
            MemField::AvailablePercent => &["available_percent"],
            MemField::Free => &["free"],
            MemField::Inactive => &["inactive"],
            MemField::SwapFree => &["swap_free"],
            MemField::SwapTotal => &["swap_total"],
            MemField::Total => &["total"],
            MemField::Used => &["used"],
            MemField::UsedPercent => &["used_percent"],
        }
    }
}

impl Column for MemField {
    fn column(&self) -> &'static str {
        match self {
            MemField::Active => "active",
            MemField::Available => "available",
            MemField::AvailablePercent => "available_percent",
            MemField::Free => "free",
            MemField::Inactive => "inactive",
            MemField::SwapFree => "swap_free",
            MemField::SwapTotal => "swap_total",
            MemField::Total => "total",
            MemField::Used => "used",
            MemField::UsedPercent => "used_percent",
        }
    }
}

impl fmt::Display for MemField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.column())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetField {
    BytesRecv,
    BytesSent,
    PacketsRecv,
    PacketsSent,
}

impl fmt::Display for NetField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.column())
    }
}

impl Column for NetField {
    fn column(&self) -> &'static str {
        match self {
            NetField::BytesRecv => "bytes_recv",
            NetField::BytesSent => "bytes_sent",
            NetField::PacketsRecv => "packets_recv",
            NetField::PacketsSent => "packets_sent",
        }
    }
}

impl Values for NetField {
    fn values(&self) -> &[&'static str] {
        match self {
            NetField::BytesRecv => &[TAG_INTERFACE, "bytes_recv"],
            NetField::BytesSent => &[TAG_INTERFACE, "bytes_sent"],
            NetField::PacketsRecv => &[TAG_INTERFACE, "packets_recv"],
            NetField::PacketsSent => &[TAG_INTERFACE, "packets_sent"],
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
