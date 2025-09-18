use serde::{Deserialize, Serialize};

/// String enum of all types of databases in Holochain as specified by `DbKind` in `holochain_sqlite`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum HolochainDatabaseKind {
    Authored,
    Dht,
    Cache,
    Conductor,
    Wasm,
    PeerMetaStore,
}

/// String enum of all workflows in Holochain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, strum::Display)]
#[strum(serialize_all = "snake_case", suffix = "_consumer")]
pub enum HolochainWorkflowKind {
    AppValidation,
    Countersigning,
    IntegrateDhtOps,
    PublishDhtOps,
    SysValidation,
    ValidationReceipt,
    Witnessing,
}
