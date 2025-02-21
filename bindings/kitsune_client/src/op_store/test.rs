use super::WtOpStore;
use crate::tests::test_reporter;
use bytes::Bytes;
use kitsune2_api::{AgentId, DhtArc, Id, OpStore, Timestamp};
use kitsune2_core::factories::MemoryOp;
use std::time::Duration;

async fn test_op_store() -> WtOpStore {
    let timestamp = Timestamp::now();
    let agent_id = AgentId(Id(Bytes::copy_from_slice(
        timestamp.as_micros().to_string().as_bytes(),
    )));
    let reporter = test_reporter();
    WtOpStore::new(agent_id, reporter)
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
    // Then wait some to produce a different `stored_at` timestamp and store the rest.
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
