use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::{CounterStats, GaugeStats};

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
    pub usage_user: GaugeStats,
    pub usage_system: GaugeStats,
}

/// RAM metrics model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemMetrics {
    pub active: GaugeStats,
    pub available: GaugeStats,
    pub available_percent: GaugeStats,
    pub free: GaugeStats,
    pub inactive: GaugeStats,
    pub swap_free: GaugeStats,
    pub swap_total: GaugeStats,
    pub total: GaugeStats,
    pub used: GaugeStats,
    pub used_percent: GaugeStats,
}

/// Network metrics model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetMetrics {
    pub bytes_recv: BTreeMap<String, CounterStats>,
    pub bytes_sent: BTreeMap<String, CounterStats>,
    pub packets_recv: BTreeMap<String, CounterStats>,
    pub packets_sent: BTreeMap<String, CounterStats>,
}
