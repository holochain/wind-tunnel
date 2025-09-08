use serde::{Deserialize, Serialize};

/// String enum of all types of databases in Holochain as specified by [`DbKind`] in `holochain_sqlite`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub enum HolochainDatabaseKind {
    Authored,
    Dht,
    Cache,
    Conductor,
    Wasm,
    #[strum(to_string = "peer_meta_store")]
    PeerMetaStore,
}

/// String enum of all workflows in Holochain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, strum::Display)]
pub enum HolochainWorkflowKind {
    #[strum(to_string = "app_validation_consumer")]
    AppValidation,
    #[strum(to_string = "countersigning_consumer")]
    Countersigning,
    #[strum(to_string = "integrate_dht_ops_consumer")]
    IntegrateDhtOps,
    #[strum(to_string = "publish_dht_ops_consumer")]
    PublishDhtOps,
    #[strum(to_string = "sys_validation_consumer")]
    SystemValidation,
    #[strum(to_string = "validation_receipt_consumer")]
    ValidationReceipt,
    #[strum(to_string = "witnessing_consumer")]
    Witnessing,
}
