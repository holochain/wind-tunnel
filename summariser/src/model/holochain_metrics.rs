use crate::model::{CounterStats, GaugeStats, StandardTimingsStats};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Display;
use strum::{EnumIter, IntoEnumIterator};

/// Holochain metrics model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HolochainMetrics {
    /// holochain_p2p request duration metric per dna_hash
    pub p2p_request_duration: BTreeMap<String, StandardTimingsStats>,
    /// post_commit() zome callback duration metric per cell_id
    pub post_commit_duration: BTreeMap<String, StandardTimingsStats>,
    /// Conductor workflow duration metric per workflow type and dna_hash
    pub workflow_duration: BTreeMap<String, BTreeMap<String, StandardTimingsStats>>,
    /// Cascade query duration metric
    pub cascade_duration: Option<StandardTimingsStats>,
    /// Holochain database metrics per database type
    pub database: BTreeMap<String, HolochainDatabaseMetrics>,
    /// Wasm usage metric per cell_id
    pub wasm_usage: BTreeMap<String, CounterStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HolochainDatabaseMetrics {
    pub utilization: GaugeStats,
    pub connection_use_time: Option<StandardTimingsStats>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, EnumIter, strum::Display,
)]
#[strum(serialize_all = "lowercase")]
pub enum HolochainDatabaseKind {
    Authored,
    Conductor,
    Dht,
    Wasm,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, EnumIter, strum::Display,
)]
pub enum HolochainWorkflowKind {
    #[strum(to_string = "app_validation_consumer")]
    AppValidation,
    #[strum(to_string = "integrate_dht_ops_consumer")]
    IntegrateDhtOps,
    #[strum(to_string = "publish_dht_ops_consumer")]
    PublishDhtOps,
    #[strum(to_string = "sys_validation_consumer")]
    SystemValidation,
    #[strum(to_string = "validation_receipt_consumer")]
    ValidationReceipt,
}

/// Configuration for HolochainMetrics processing
#[derive(Debug, Clone, PartialEq)]
pub struct HolochainMetricsConfig {
    /// Enable processing of p2p_request_duration metrics
    pub p2p_request_duration: bool,
    /// Enable processing of post_commit_duration metrics
    pub post_commit_duration: bool,
    /// Enable processing of certain workflow_duration metrics
    pub workflows: StrumVariantSelector<HolochainWorkflowKind>,
    /// Enable processing of cascade_duration metrics
    pub cascade_duration: bool,
    /// Enable processing of certain database metrics
    pub databases: StrumVariantSelector<HolochainDatabaseKind>,
    /// Enable processing of wasm_usage metrics
    pub wasm_usage: bool,
}

/// Enum for selecting specific, all, or no variants of a string enum
#[derive(Debug, Clone, PartialEq)]
pub enum StrumVariantSelector<T>
where
    T: Ord + IntoEnumIterator + Display,
{
    /// Process all variants
    All,
    /// Process no variants
    None,
    /// Process only specific variants
    Specific(BTreeSet<T>),
}

impl<T> StrumVariantSelector<T>
where
    T: Ord + IntoEnumIterator + Display,
{
    /// Create a new selector that processes all variants
    pub fn all() -> Self {
        Self::All
    }

    /// Create a new selector that processes no variants
    pub fn none() -> Self {
        Self::None
    }

    /// Create a new selector that processes specific variants
    pub fn specific(variants: impl IntoIterator<Item = T>) -> Self {
        Self::Specific(variants.into_iter().collect())
    }

    /// Get an iterator of the selected variants as strings
    pub fn into_str_iter(self) -> impl Iterator<Item = String> {
        let strings = match self {
            Self::All => T::iter().map(|variant| variant.to_string()).collect(),
            Self::None => vec![],
            Self::Specific(set) => set.iter().map(|variant| variant.to_string()).collect(),
        };
        strings.into_iter()
    }
}

impl Default for HolochainMetricsConfig {
    fn default() -> Self {
        Self::none()
    }
}

impl HolochainMetricsConfig {
    /// Create a new configuration with all metrics enabled
    pub fn all() -> Self {
        Self {
            p2p_request_duration: true,
            post_commit_duration: true,
            workflows: StrumVariantSelector::All,
            cascade_duration: true,
            databases: StrumVariantSelector::All,
            wasm_usage: true,
        }
    }

    /// Create a new configuration with all metrics disabled
    pub fn none() -> Self {
        Self {
            p2p_request_duration: false,
            post_commit_duration: false,
            workflows: StrumVariantSelector::None,
            cascade_duration: false,
            databases: StrumVariantSelector::None,
            wasm_usage: false,
        }
    }
}

impl HolochainMetricsConfig {
    pub fn with_p2p_request_duration(mut self, enabled: bool) -> Self {
        self.p2p_request_duration = enabled;
        self
    }

    pub fn with_post_commit_duration(mut self, enabled: bool) -> Self {
        self.post_commit_duration = enabled;
        self
    }

    pub fn with_workflows(mut self, config: StrumVariantSelector<HolochainWorkflowKind>) -> Self {
        self.workflows = config;
        self
    }

    pub fn with_cascade_duration(mut self, enabled: bool) -> Self {
        self.cascade_duration = enabled;
        self
    }

    pub fn with_databases(mut self, config: StrumVariantSelector<HolochainDatabaseKind>) -> Self {
        self.databases = config;
        self
    }

    pub fn with_wasm_usage(mut self, enabled: bool) -> Self {
        self.wasm_usage = enabled;
        self
    }
}
