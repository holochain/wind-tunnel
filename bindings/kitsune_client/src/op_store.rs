use bytes::Bytes;
use kitsune2::default_builder;
use kitsune2_api::{
    AgentId, BoxFut, Builder, Config, DhtArc, DynOpStore, K2Result, MetaOp, OpId, OpStore,
    OpStoreFactory, SpaceId, Timestamp,
};
use std::sync::Arc;
use wind_tunnel_instruments::{
    prelude::{ReportMetric, Reporter},
    OperationRecord,
};

#[derive(Debug)]
pub(super) struct WtOpStoreFactory {
    agent_id: AgentId,
    reporter: Arc<Reporter>,
}

impl OpStoreFactory for WtOpStoreFactory {
    fn create(
        &self,
        _builder: Arc<Builder>,
        space: SpaceId,
    ) -> BoxFut<'static, K2Result<DynOpStore>> {
        let agent_id = self.agent_id.clone();
        let reporter = self.reporter.clone();
        Box::pin(async move {
            // Create a memory op store. The passed in builder cannot be used for this, because
            // it would call this create function recursively ad infinitum.
            let builder = Arc::new(default_builder().with_default_config()?);
            let op_store = builder.op_store.create(builder.clone(), space).await?;
            let out: DynOpStore = Arc::new(WtOpStore::new(op_store, agent_id, reporter));
            Ok(out)
        })
    }

    fn default_config(&self, _config: &mut Config) -> K2Result<()> {
        Ok(())
    }

    fn validate_config(&self, _config: &Config) -> K2Result<()> {
        Ok(())
    }
}

impl WtOpStoreFactory {
    pub fn new(agent_id: AgentId, reporter: Arc<Reporter>) -> Self {
        Self { agent_id, reporter }
    }
}

#[derive(Debug)]
struct WtOpStore {
    inner: DynOpStore,
    reporter: Arc<Reporter>,
    agent_id: AgentId,
}

impl OpStore for WtOpStore {
    fn process_incoming_ops(&self, op_list: Vec<Bytes>) -> BoxFut<'_, K2Result<Vec<OpId>>> {
        Box::pin(async {
            let inserted_op_ids = self.inner.process_incoming_ops(op_list).await?;
            let amount = inserted_op_ids.len();
            log::info!("{} ops have come in to {}", amount, self.agent_id);
            if amount > 0 {
                let operation_record = OperationRecord::new("incoming_message".to_string());
                let wt_result = Result::<(), ()>::Ok(());
                wind_tunnel_instruments::report_operation(
                    self.reporter.clone(),
                    operation_record,
                    &wt_result,
                );
                self.reporter.add_custom(
                    ReportMetric::new("number_of_incoming_ops")
                        .with_tag("agent_id", self.agent_id.to_string())
                        .with_field("number_of_incoming_ops", amount as u32),
                );
            }
            Ok(inserted_op_ids)
        })
    }

    fn retrieve_ops(&self, op_ids: Vec<OpId>) -> BoxFut<'_, K2Result<Vec<MetaOp>>> {
        self.inner.retrieve_ops(op_ids)
    }

    fn retrieve_op_ids_bounded(
        &self,
        arc: DhtArc,
        start: Timestamp,
        limit_bytes: u32,
    ) -> BoxFut<'_, K2Result<(Vec<OpId>, u32, Timestamp)>> {
        self.inner.retrieve_op_ids_bounded(arc, start, limit_bytes)
    }

    fn retrieve_op_hashes_in_time_slice(
        &self,
        arc: DhtArc,
        start: Timestamp,
        end: Timestamp,
    ) -> BoxFut<'_, K2Result<(Vec<OpId>, u32)>> {
        self.inner.retrieve_op_hashes_in_time_slice(arc, start, end)
    }

    fn store_slice_hash(
        &self,
        arc: DhtArc,
        slice_index: u64,
        slice_hash: Bytes,
    ) -> BoxFut<'_, K2Result<()>> {
        self.inner.store_slice_hash(arc, slice_index, slice_hash)
    }

    fn slice_hash_count(&self, arc: DhtArc) -> BoxFut<'_, K2Result<u64>> {
        self.inner.slice_hash_count(arc)
    }

    fn retrieve_slice_hashes(&self, arc: DhtArc) -> BoxFut<'_, K2Result<Vec<(u64, Bytes)>>> {
        self.inner.retrieve_slice_hashes(arc)
    }

    fn retrieve_slice_hash(
        &self,
        arc: DhtArc,
        slice_index: u64,
    ) -> BoxFut<'_, K2Result<Option<Bytes>>> {
        self.inner.retrieve_slice_hash(arc, slice_index)
    }
}

impl WtOpStore {
    pub fn new(op_store: DynOpStore, agent_id: AgentId, reporter: Arc<Reporter>) -> Self {
        Self {
            inner: op_store,
            agent_id,
            reporter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WtOpStore;
    use crate::tests::test_reporter;
    use bytes::Bytes;
    use kitsune2::default_builder;
    use kitsune2_api::{AgentId, DhtArc, Id, OpStore, SpaceId, Timestamp};
    use kitsune2_core::factories::MemoryOp;
    use std::{sync::Arc, time::Duration};

    async fn test_op_store() -> WtOpStore {
        let builder = Arc::new(default_builder().with_default_config().unwrap());
        let timestamp = Timestamp::now();
        let space_id = SpaceId(Id(Bytes::copy_from_slice(
            timestamp.as_micros().to_string().as_bytes(),
        )));
        let agent_id = AgentId(Id(Bytes::copy_from_slice(
            timestamp.as_micros().to_string().as_bytes(),
        )));
        let inner_op_store = builder
            .op_store
            .create(builder.clone(), space_id)
            .await
            .unwrap();
        let reporter = test_reporter();
        WtOpStore::new(inner_op_store, agent_id, reporter)
    }

    // This test does not assert anything, it just serves to experiment with the
    // reporter.
    #[tokio::test]
    async fn reporter() {
        let op_store = test_op_store().await;

        for _ in 0..5 {
            let timestamp = Timestamp::now();
            let op = MemoryOp {
                created_at: timestamp,
                op_data: format!("data_{}", timestamp.as_micros()).into(),
            };
            let _ = op_store
                .process_incoming_ops(vec![op.clone().into()])
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        op_store.reporter.finalize();
    }

    #[test]
    fn happy_op_to_bytes() {
        let op = MemoryOp {
            created_at: Timestamp::now(),
            op_data: vec![0],
        };
        let bytes = Bytes::from(op.clone());
        let decoded_op = MemoryOp::from(bytes);
        assert_eq!(op, decoded_op);
    }

    #[tokio::test]
    async fn process_incoming_ops_and_retrieve() {
        let op_store = test_op_store().await;
        let op_1 = MemoryOp {
            created_at: Timestamp::from_micros(0),
            op_data: vec![1],
        };
        let op_2 = MemoryOp {
            created_at: Timestamp::from_micros(0),
            op_data: vec![2],
        };
        let op_list = vec![Bytes::from(op_1.clone()), Bytes::from(op_2.clone())];

        op_store
            .process_incoming_ops(op_list.clone())
            .await
            .unwrap();

        let op_ids = vec![op_1.compute_op_id(), op_2.compute_op_id()];
        let ops = op_store.retrieve_ops(op_ids).await.unwrap();
        assert_eq!(
            ops.into_iter().map(|op| op.op_data).collect::<Vec<_>>(),
            op_list
        );
    }

    #[tokio::test]
    async fn op_hashes_in_time_slice() {
        let op_store = test_op_store().await;
        let included_op_1 = MemoryOp {
            created_at: Timestamp::from_micros(10),
            op_data: vec![1],
        };
        let included_op_id_1 = included_op_1.compute_op_id();
        let included_op_2 = MemoryOp {
            created_at: Timestamp::from_micros(0),
            op_data: vec![2],
        };
        let included_op_id_2 = included_op_2.compute_op_id();
        let excluded_op_1 = MemoryOp {
            created_at: Timestamp::from_micros(100),
            op_data: vec![3],
        };
        let excluded_op_2 = MemoryOp {
            created_at: Timestamp::from_micros(0),
            op_data: vec![101],
        };

        let _ = op_store
            .process_incoming_ops(vec![
                included_op_1.clone().into(),
                included_op_2.clone().into(),
                excluded_op_1.into(),
                excluded_op_2.into(),
            ])
            .await
            .unwrap();

        let arc = DhtArc::Arc(0, 100);
        let start = Timestamp::from_micros(0);
        let end = Timestamp::from_micros(100);
        let (op_hashes, bytes) = op_store
            .retrieve_op_hashes_in_time_slice(arc, start, end)
            .await
            .unwrap();

        let expected_op_hashes = vec![included_op_id_2, included_op_id_1];
        let expected_bytes = (included_op_1.op_data.len() + included_op_2.op_data.len()) as u32;
        assert_eq!((op_hashes, bytes), (expected_op_hashes, expected_bytes));
    }

    #[tokio::test]
    async fn bounded_op_ids() {
        let op_store = test_op_store().await;
        let included_op_1 = MemoryOp {
            created_at: Timestamp::from_micros(0),
            op_data: vec![1; 9],
        };
        let included_op_id_1 = included_op_1.compute_op_id();
        let excess_op_1 = MemoryOp {
            created_at: Timestamp::from_micros(0),
            op_data: vec![1; 3],
        };
        let excluded_op_1 = MemoryOp {
            created_at: Timestamp::from_micros(0),
            op_data: vec![1],
        };
        let excluded_op_2 = MemoryOp {
            created_at: Timestamp::from_micros(0),
            op_data: vec![255],
        };

        // Store op to be excluded first.
        let _ = op_store
            .process_incoming_ops(vec![excluded_op_1.into()])
            .await
            .unwrap();
        // Then wait some and store the rest.
        tokio::time::sleep(Duration::from_millis(10)).await;
        let timestamp = Timestamp::now();
        let _ = op_store
            .process_incoming_ops(vec![
                included_op_1.clone().into(),
                excess_op_1.clone().into(),
                excluded_op_2.into(),
            ])
            .await
            .unwrap();

        let (op_ids, bytes, next_timestamp) = op_store
            .retrieve_op_ids_bounded(DhtArc::Arc(0, 100), timestamp, 10)
            .await
            .unwrap();
        assert_eq!(op_ids, vec![included_op_id_1]);
        assert_eq!(bytes, included_op_1.op_data.len() as u32);
        assert!(next_timestamp > timestamp);
    }

    #[tokio::test]
    async fn insert_empty_hash() {
        let op_store = test_op_store().await;
        let arc = DhtArc::Arc(10, 100);

        let insert_empty_hash_result = op_store.store_slice_hash(arc, 0, Bytes::new()).await;
        assert!(insert_empty_hash_result.is_err());
    }

    #[tokio::test]
    async fn insert_and_retrieve_time_slice_hash() {
        let op_store = test_op_store().await;
        let arc = DhtArc::Arc(10, 100);

        let slice_hash_0 = Bytes::from_static(b"slice_hash_0");
        op_store
            .store_slice_hash(arc, 0, slice_hash_0.clone())
            .await
            .unwrap();

        let slice_hash_count = op_store.slice_hash_count(arc).await.unwrap();
        assert_eq!(slice_hash_count, 1);
        let slice_hashes = op_store.retrieve_slice_hashes(arc).await.unwrap();
        assert_eq!(slice_hashes, vec![(0, slice_hash_0.clone())]);

        let actual_slice_hash_0 = op_store.retrieve_slice_hash(arc, 0).await.unwrap();
        assert_eq!(actual_slice_hash_0, Some(slice_hash_0));

        let retrieve_non_existing_arc_hash = op_store
            .retrieve_slice_hash(DhtArc::Arc(1, 10), 0)
            .await
            .unwrap();
        assert!(retrieve_non_existing_arc_hash.is_none());
    }

    #[tokio::test]
    async fn highest_stored_slice_hash() {
        let op_store = test_op_store().await;
        let arc = DhtArc::Arc(10, 100);
        let slice_hash_count = op_store.slice_hash_count(arc).await.unwrap();
        assert_eq!(slice_hash_count, 0);

        let highest_slice_hash = Bytes::from_static(b"highest_slice_hash");
        op_store
            .store_slice_hash(arc, 20, highest_slice_hash.clone())
            .await
            .unwrap();

        let slice_hash_count = op_store.slice_hash_count(arc).await.unwrap();
        assert_eq!(slice_hash_count, 21);

        op_store
            .store_slice_hash(arc, 2, Bytes::from_static(b"lower_slice_hash"))
            .await
            .unwrap();

        let slice_hash_count = op_store.slice_hash_count(arc).await.unwrap();
        assert_eq!(slice_hash_count, 21);
    }
}
