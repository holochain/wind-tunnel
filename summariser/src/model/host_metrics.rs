use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::{CounterRateStats, StandardTimingsStats};

/// Host metrics model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HostMetrics {
    /// [`CpuMetrics`] for the host; load is averaged among CPU cores
    pub cpu: CpuMetrics,
    /// [`MemMetrics`] for the host
    pub memory: MemMetrics,
    /// [`NetMetrics`] aggregated by the network interface name
    pub network: NetMetrics,
}

/// CPU metrics model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CpuMetrics {
    pub usage_user: StandardTimingsStats,
    pub usage_system: StandardTimingsStats,
}

/// RAM metrics model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemMetrics {
    pub active: StandardTimingsStats,
    pub available: StandardTimingsStats,
    pub available_percent: StandardTimingsStats,
    pub free: StandardTimingsStats,
    pub inactive: StandardTimingsStats,
    pub swap_free: StandardTimingsStats,
    pub swap_total: StandardTimingsStats,
    pub total: StandardTimingsStats,
    pub used: StandardTimingsStats,
    pub used_percent: StandardTimingsStats,
}

/// Network metrics model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetMetrics {
    pub bytes_recv: BTreeMap<String, CounterRateStats>,
    pub bytes_sent: BTreeMap<String, CounterRateStats>,
    pub packets_recv: BTreeMap<String, CounterRateStats>,
    pub packets_sent: BTreeMap<String, CounterRateStats>,
}
