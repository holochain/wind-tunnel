//! The mem op store implementation for WindTunnel.

use bytes::Bytes;
use kitsune2_api::{
    AgentId, BoxFut, Builder, Config, DhtArc, DynOpStore, K2Error, K2Result, MetaOp, Op, OpId,
    OpStore, OpStoreFactory, SpaceId, StoredOp, Timestamp,
};
use serde::{Deserialize, Serialize};
use sha3::Digest;
use std::collections::HashMap;
use std::sync::Arc;
use time_slice_hash_store::TimeSliceHashStore;
use tokio::sync::RwLock;
use wind_tunnel_instruments::prelude::{ReportMetric, Reporter};

mod time_slice_hash_store;

#[cfg(test)]
mod test;

#[derive(Debug)]
pub(super) struct WtOpStoreFactory {
    op_store: DynWtOpStore,
}

impl OpStoreFactory for WtOpStoreFactory {
    fn create(
        &self,
        _builder: Arc<Builder>,
        _space: SpaceId,
    ) -> BoxFut<'static, K2Result<DynOpStore>> {
        // A handle to the op store is required for direct manipulation with custom methods
        // beyond the trait ones. Hence the factory is created with an op store and this `create`
        // method, which is called when the space is created, will simply return it.
        let op_store: DynOpStore = self.op_store.clone();
        Box::pin(async move { Ok(op_store) })
    }

    fn default_config(&self, _config: &mut Config) -> K2Result<()> {
        Ok(())
    }

    fn validate_config(&self, _config: &Config) -> K2Result<()> {
        Ok(())
    }
}

impl WtOpStoreFactory {
    pub fn new(op_store: DynWtOpStore) -> Self {
        Self { op_store }
    }
}

/// A WindTunnel op which holds a string message as data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WtOp {
    /// The creation timestamp of this op.
    pub created_at: Timestamp,
    /// The data of the op.
    pub op_data: Vec<u8>,
}

impl WtOp {
    /// Create a new [MemoryOp].
    pub fn new(timestamp: Timestamp, payload: Vec<u8>) -> Self {
        Self {
            created_at: timestamp,
            op_data: payload,
        }
    }

    /// Compute the op id for this op.
    pub fn compute_op_id(&self) -> OpId {
        if cfg!(test) {
            // Note that this produces predictable op ids for testing purposes.
            // It is simply the first 32 bytes of the op data.
            let mut value = self.op_data.as_slice()[..32.min(self.op_data.len())].to_vec();
            value.resize(32, 0);
            OpId::from(bytes::Bytes::from(value))
        } else {
            // For WindTunnel scenarios a common hashing algorithm computes the op id.
            let mut hasher = sha3::Sha3_256::new();
            hasher.update(self.op_data.clone());
            OpId::from(Bytes::copy_from_slice(&hasher.finalize()))
        }
    }
}

impl From<Bytes> for WtOp {
    fn from(value: Bytes) -> Self {
        serde_json::from_slice(&value).expect("failed to deserialize MemoryOp from bytes")
    }
}

impl From<WtOp> for Bytes {
    fn from(value: WtOp) -> Self {
        serde_json::to_vec(&value)
            .expect("failed to serialize MemoryOp to bytes")
            .into()
    }
}

/// This is the storage record for an op with computed fields.
///
/// Test data should create [MemoryOp]s and not be aware of this type.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WtOpRecord {
    /// The id (hash) of the op
    pub op_id: OpId,
    /// The creation timestamp of this op
    pub created_at: Timestamp,
    /// The timestamp at which this op was stored by us
    pub stored_at: Timestamp,
    /// The data for the op
    pub op_data: Vec<u8>,
}

impl From<Bytes> for WtOpRecord {
    fn from(value: Bytes) -> Self {
        let inner: WtOp = value.into();
        Self {
            op_id: inner.compute_op_id(),
            created_at: inner.created_at,
            stored_at: Timestamp::now(),
            op_data: inner.op_data,
        }
    }
}

impl From<WtOp> for StoredOp {
    fn from(value: WtOp) -> Self {
        StoredOp {
            op_id: value.compute_op_id(),
            created_at: value.created_at,
        }
    }
}

impl From<Op> for WtOp {
    fn from(value: Op) -> Self {
        value.data.into()
    }
}

/// An in-memory op store for WindTunnel.
#[derive(Debug)]
pub(crate) struct WtOpStore {
    agent_id: AgentId,
    inner: RwLock<WtOpStoreInner>,
    reporter: Arc<Reporter>,
}

/// WtOpStore trait object.
pub(crate) type DynWtOpStore = Arc<WtOpStore>;

impl WtOpStore {
    pub fn new(agent_id: AgentId, reporter: Arc<Reporter>) -> Self {
        Self {
            agent_id,
            inner: Default::default(),
            reporter,
        }
    }

    pub async fn store_ops(&self, ops: Vec<WtOp>) -> anyhow::Result<Vec<OpId>> {
        let mut inner_lock = self.inner.write().await;
        let mut inserted_op_ids = Vec::new();
        for op in ops {
            let op_record = WtOpRecord::from(Bytes::from(op));
            if !inner_lock.op_list.contains_key(&op_record.op_id) {
                inserted_op_ids.push(op_record.op_id.clone());
                inner_lock
                    .op_list
                    .insert(op_record.op_id.clone(), op_record);
            }
        }
        Ok(inserted_op_ids)
    }
}

impl std::ops::Deref for WtOpStore {
    type Target = RwLock<WtOpStoreInner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// WindTunnel op store inner state.
#[derive(Debug, Default)]
pub(crate) struct WtOpStoreInner {
    op_list: HashMap<OpId, WtOpRecord>,
    time_slice_hashes: TimeSliceHashStore,
}

impl OpStore for WtOpStore {
    fn process_incoming_ops(&self, op_list: Vec<Bytes>) -> BoxFut<'_, K2Result<Vec<OpId>>> {
        Box::pin(async move {
            let ops_to_add = op_list
                .iter()
                .map(|op| -> serde_json::Result<(OpId, WtOpRecord)> {
                    let op = WtOpRecord::from(op.clone());
                    Ok((op.op_id.clone(), op))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    K2Error::other_src("Failed to deserialize op data, are you using `WtOp`s?", e)
                })?;

            let mut op_ids = Vec::with_capacity(ops_to_add.len());
            let mut lock = self.write().await;
            for (op_id, record) in ops_to_add {
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    lock.op_list.entry(op_id.clone())
                {
                    entry.insert(record);
                    op_ids.push(op_id);
                }
            }

            // After inserting incoming ops into the store, the number of inserted ops is reported.
            let amount = op_ids.len();
            log::info!("{} ops have come in to {}", amount, self.agent_id);
            if amount > 0 {
                // Kitsune is calling this method with empty vectors. Work around it by not reporting
                // such events.
                self.reporter.add_custom(
                    ReportMetric::new("heard_messages")
                        .with_tag("agent_id", self.agent_id.to_string())
                        .with_field("num_messages", amount as u32),
                );
            }

            Ok(op_ids)
        })
    }

    fn retrieve_op_hashes_in_time_slice(
        &self,
        arc: DhtArc,
        start: Timestamp,
        end: Timestamp,
    ) -> BoxFut<'_, K2Result<(Vec<OpId>, u32)>> {
        Box::pin(async move {
            let self_lock = self.read().await;

            let mut used_bytes = 0;
            let mut candidate_ops = self_lock
                .op_list
                .iter()
                .filter(|(_, op)| {
                    let loc = op.op_id.loc();
                    op.created_at >= start && op.created_at < end && arc.contains(loc)
                })
                .collect::<Vec<_>>();
            candidate_ops.sort_by_key(|a| a.1.created_at);

            Ok((
                candidate_ops
                    .iter()
                    .map(|(op_id, record)| {
                        used_bytes += record.op_data.len() as u32;
                        (*op_id).clone()
                    })
                    .collect(),
                used_bytes,
            ))
        })
    }

    fn retrieve_ops(&self, op_ids: Vec<OpId>) -> BoxFut<'_, K2Result<Vec<MetaOp>>> {
        Box::pin(async move {
            let self_lock = self.read().await;
            Ok(op_ids
                .iter()
                .filter_map(|op_id| {
                    self_lock.op_list.get(op_id).map(|op| MetaOp {
                        op_id: op.op_id.clone(),
                        op_data: WtOp {
                            created_at: op.created_at,
                            op_data: op.op_data.clone(),
                        }
                        .into(),
                    })
                })
                .collect())
        })
    }

    fn retrieve_op_ids_bounded(
        &self,
        arc: DhtArc,
        start: Timestamp,
        limit_bytes: u32,
    ) -> BoxFut<'_, K2Result<(Vec<OpId>, u32, Timestamp)>> {
        Box::pin(async move {
            let new_start = Timestamp::now();

            let self_lock = self.read().await;

            // Capture all ops that are within the arc and after the start time
            let mut candidate_ops = self_lock
                .op_list
                .values()
                .filter(|op| arc.contains(op.op_id.loc()) && op.stored_at >= start)
                .collect::<Vec<_>>();

            // Sort the ops by the time they were stored
            candidate_ops.sort_by(|a, b| a.stored_at.cmp(&b.stored_at));

            // Now take as many ops as we can up to the limit
            let mut total_bytes = 0;
            let mut last_op_timestamp = None;
            let op_ids = candidate_ops
                .into_iter()
                .take_while(|op| {
                    let data_len = op.op_data.len() as u32;
                    if total_bytes + data_len <= limit_bytes {
                        total_bytes += data_len;
                        true
                    } else {
                        last_op_timestamp = Some(op.stored_at);
                        false
                    }
                })
                .map(|op| op.op_id.clone())
                .collect();

            Ok((
                op_ids,
                total_bytes,
                if let Some(ts) = last_op_timestamp {
                    ts
                } else {
                    new_start
                },
            ))
        })
    }

    /// Store the combined hash of a time slice.
    ///
    /// The `slice_id` is the index of the time slice. This is a 0-based index. So for a given
    /// time period being used to slice time, the first `slice_hash` at `slice_id` 0 would
    /// represent the combined hash of all known ops in the time slice `[0, period)`. Then `slice_id`
    /// 1 would represent the combined hash of all known ops in the time slice `[period, 2*period)`.
    fn store_slice_hash(
        &self,
        arc: DhtArc,
        slice_index: u64,
        slice_hash: bytes::Bytes,
    ) -> BoxFut<'_, K2Result<()>> {
        Box::pin(async move {
            self.write()
                .await
                .time_slice_hashes
                .insert(arc, slice_index, slice_hash)
        })
    }

    /// Retrieve the count of time slice hashes stored.
    ///
    /// Note that this is not the total number of hashes of a time slice at a unique `slice_id`.
    /// This value is the count, based on the highest stored id, starting from time slice id 0 and counting up to the highest stored id. In other words it is the id of the most recent time slice plus 1.
    ///
    /// This value is easier to compare between peers because it ignores sync progress. A simple
    /// count cannot tell the difference between a peer that has synced the first 4 time slices,
    /// and a peer who has synced the first 3 time slices and created one recent one. However,
    /// using the highest stored id shows the difference to be 4 and say 300 respectively.
    /// Equally, the literal count is more useful if the DHT contains a large amount of data and
    /// a peer might allocate a recent full slice before completing its initial sync. That situation
    /// could be created by a configuration that chooses small time-slices. However, in the general
    /// case, the highest stored id is more useful.
    fn slice_hash_count(&self, arc: DhtArc) -> BoxFut<'_, K2Result<u64>> {
        // +1 to convert from a 0-based index to a count
        Box::pin(async move {
            Ok(self
                .read()
                .await
                .time_slice_hashes
                .highest_stored_id(&arc)
                .map(|id| id + 1)
                .unwrap_or_default())
        })
    }

    /// Retrieve the hash of a time slice.
    ///
    /// This must be the same value provided by the caller to `store_slice_hash` for the same `slice_id`.
    /// If `store_slice_hash` has been called multiple times for the same `slice_id`, the most recent value is returned.
    /// If the caller has never provided a value for this `slice_id`, return `None`.
    fn retrieve_slice_hash(
        &self,
        arc: DhtArc,
        slice_index: u64,
    ) -> BoxFut<'_, K2Result<Option<bytes::Bytes>>> {
        Box::pin(async move { Ok(self.read().await.time_slice_hashes.get(&arc, slice_index)) })
    }

    /// Retrieve the hashes of all time slices.
    fn retrieve_slice_hashes(&self, arc: DhtArc) -> BoxFut<'_, K2Result<Vec<(u64, bytes::Bytes)>>> {
        Box::pin(async move {
            let self_lock = self.read().await;
            Ok(self_lock.time_slice_hashes.get_all(&arc))
        })
    }
}
