use std::{fmt, str::FromStr};

/// Name for [`HostMetrics`].
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl FromStr for HostMetricsName {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cpu" => Ok(HostMetricsName::Cpu),
            "disk" => Ok(HostMetricsName::Disk),
            "diskio" => Ok(HostMetricsName::Diskio),
            "kernel" => Ok(HostMetricsName::Kernel),
            "mem" => Ok(HostMetricsName::Mem),
            "net" => Ok(HostMetricsName::Net),
            "netstat" => Ok(HostMetricsName::Netstat),
            "processes" => Ok(HostMetricsName::Processes),
            "swap" => Ok(HostMetricsName::Swap),
            "system" => Ok(HostMetricsName::System),
            _ => Err("Unknown host metrics name"),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_convert_str_to_host_metrics_name() {
        assert_eq!(
            HostMetricsName::from_str("cpu").unwrap(),
            HostMetricsName::Cpu
        );
        assert_eq!(
            HostMetricsName::from_str("disk").unwrap(),
            HostMetricsName::Disk
        );
        assert_eq!(
            HostMetricsName::from_str("diskio").unwrap(),
            HostMetricsName::Diskio
        );
        assert_eq!(
            HostMetricsName::from_str("kernel").unwrap(),
            HostMetricsName::Kernel
        );
        assert_eq!(
            HostMetricsName::from_str("mem").unwrap(),
            HostMetricsName::Mem
        );
        assert_eq!(
            HostMetricsName::from_str("net").unwrap(),
            HostMetricsName::Net
        );
        assert_eq!(
            HostMetricsName::from_str("netstat").unwrap(),
            HostMetricsName::Netstat
        );
        assert_eq!(
            HostMetricsName::from_str("processes").unwrap(),
            HostMetricsName::Processes
        );
        assert_eq!(
            HostMetricsName::from_str("swap").unwrap(),
            HostMetricsName::Swap
        );
        assert_eq!(
            HostMetricsName::from_str("system").unwrap(),
            HostMetricsName::System
        );
        assert_eq!(
            HostMetricsName::from_str("unknown"),
            Err("Unknown host metrics name")
        );
    }

    #[test]
    fn test_should_display_host_metrics_name() {
        assert_eq!(HostMetricsName::Cpu.to_string(), "cpu");
        assert_eq!(HostMetricsName::Disk.to_string(), "disk");
        assert_eq!(HostMetricsName::Diskio.to_string(), "diskio");
        assert_eq!(HostMetricsName::Kernel.to_string(), "kernel");
        assert_eq!(HostMetricsName::Mem.to_string(), "mem");
        assert_eq!(HostMetricsName::Net.to_string(), "net");
        assert_eq!(HostMetricsName::Netstat.to_string(), "netstat");
        assert_eq!(HostMetricsName::Processes.to_string(), "processes");
        assert_eq!(HostMetricsName::Swap.to_string(), "swap");
        assert_eq!(HostMetricsName::System.to_string(), "system");
    }
}
