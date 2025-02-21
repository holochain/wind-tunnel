use kitsune2_api::{DhtArc, K2Error, K2Result};
use std::collections::{BTreeMap, HashMap};

/// In-memory store for time slice hashes.
///
/// Empty hashes do not need to be stored, and will be rejected with an error if you try to insert
/// one. Hashes that are stored, are stored sparsely, and are indexed by the slice id.
///
/// It is valid to look up a time slice which has not had a hash stored, and you will get a `None`
/// response. Otherwise, you will get exactly what was most recently stored for that slice id.
#[derive(Debug, Default)]
#[cfg_attr(test, derive(Clone))]
pub struct TimeSliceHashStore {
    inner: HashMap<DhtArc, BTreeMap<u64, bytes::Bytes>>,
}

impl TimeSliceHashStore {
    /// Insert a hash at the given slice id.
    pub fn insert(&mut self, arc: DhtArc, slice_id: u64, hash: bytes::Bytes) -> K2Result<()> {
        // This doesn't need to be supported. If we receive an empty hash
        // for a slice id after we've already stored a non-empty hash for
        // that slice id, then the caller has done something wrong.
        // Alternatively, if we've computed an empty hash for a time slice, then we don't
        // need to store that.
        if hash.is_empty() {
            return Err(K2Error::other("Cannot insert empty combined hash"));
        }

        self.inner.entry(arc).or_default().insert(slice_id, hash);

        Ok(())
    }

    pub fn get(&self, arc: &DhtArc, slice_id: u64) -> Option<bytes::Bytes> {
        self.inner
            .get(arc)
            .and_then(|by_arc| by_arc.get(&slice_id))
            .cloned()
    }

    pub fn get_all(&self, arc: &DhtArc) -> Vec<(u64, bytes::Bytes)> {
        self.inner
            .get(arc)
            .map(|by_arc| {
                by_arc
                    .iter()
                    .map(|(id, hash)| (*id, hash.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn highest_stored_id(&self, arc: &DhtArc) -> Option<u64> {
        self.inner
            .get(arc)
            .and_then(|by_arc| by_arc.iter().last().map(|(id, _)| *id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_empty() {
        let store = TimeSliceHashStore::default();

        assert_eq!(None, store.highest_stored_id(&DhtArc::Arc(0, 0)));
        assert!(store.inner.is_empty());
    }

    #[test]
    fn insert_empty_hash_into_empty() {
        let mut store = TimeSliceHashStore::default();

        let e = store
            .insert(DhtArc::Arc(0, 2), 100, bytes::Bytes::new())
            .unwrap_err();
        assert_eq!(
            "Cannot insert empty combined hash (src: None)",
            e.to_string()
        );
    }

    #[test]
    fn insert_single_hash_into_empty() {
        let mut store = TimeSliceHashStore::default();

        let arc_constraint = DhtArc::Arc(0, 2);
        store
            .insert(arc_constraint, 100, vec![1, 2, 3].into())
            .unwrap();

        assert_eq!(1, store.inner.len());
        assert_eq!(
            bytes::Bytes::from_static(&[1, 2, 3]),
            store.get(&arc_constraint, 100).unwrap()
        );
        assert_eq!(Some(100), store.highest_stored_id(&arc_constraint));
    }

    #[test]
    fn insert_many_sparse() {
        let mut store = TimeSliceHashStore::default();

        let arc_constraint = DhtArc::Arc(0, 2);
        store
            .insert(arc_constraint, 100, vec![1, 2, 3].into())
            .unwrap();
        store
            .insert(arc_constraint, 105, vec![2, 3, 4].into())
            .unwrap();
        store
            .insert(arc_constraint, 115, vec![3, 4, 5].into())
            .unwrap();

        assert_eq!(1, store.inner.len());
        assert_eq!(3, store.inner[&arc_constraint].len());
        assert_eq!(
            bytes::Bytes::from_static(&[1, 2, 3]),
            store.get(&arc_constraint, 100).unwrap()
        );
        assert_eq!(
            bytes::Bytes::from_static(&[2, 3, 4]),
            store.get(&arc_constraint, 105).unwrap()
        );
        assert_eq!(
            bytes::Bytes::from_static(&[3, 4, 5]),
            store.get(&arc_constraint, 115).unwrap()
        );
        assert_eq!(Some(115), store.highest_stored_id(&arc_constraint));
    }

    #[test]
    fn insert_many_in_sequence() {
        let mut store = TimeSliceHashStore::default();

        let arc_constraint = DhtArc::Arc(0, 2);
        store
            .insert(arc_constraint, 100, vec![1, 2, 3].into())
            .unwrap();
        store
            .insert(arc_constraint, 101, vec![2, 3, 4].into())
            .unwrap();
        store
            .insert(arc_constraint, 102, vec![3, 4, 5].into())
            .unwrap();

        assert_eq!(1, store.inner.len());
        assert_eq!(3, store.inner[&arc_constraint].len());

        assert_eq!(
            bytes::Bytes::from_static(&[1, 2, 3]),
            store.get(&arc_constraint, 100).unwrap()
        );
        assert_eq!(
            bytes::Bytes::from_static(&[2, 3, 4]),
            store.get(&arc_constraint, 101).unwrap()
        );
        assert_eq!(
            bytes::Bytes::from_static(&[3, 4, 5]),
            store.get(&arc_constraint, 102).unwrap()
        );
        assert_eq!(Some(102), store.highest_stored_id(&arc_constraint));
    }

    #[test]
    fn overwrite_existing_hash() {
        let mut store = TimeSliceHashStore::default();

        let arc_constraint = DhtArc::Arc(0, 2);
        store
            .insert(arc_constraint, 100, vec![1, 2, 3].into())
            .unwrap();
        assert_eq!(1, store.inner.len());

        store
            .insert(arc_constraint, 100, vec![2, 3, 4].into())
            .unwrap();
        assert_eq!(1, store.inner.len());
    }

    #[test]
    fn overlapping_arcs_are_kept_separate() {
        let mut store = TimeSliceHashStore::default();

        let arc_constraint_1 = DhtArc::Arc(0, 2);

        // Twice the size of arc_constraint_1, starting at the same point
        let arc_constraint_2 = DhtArc::Arc(0, 4);

        store
            .insert(arc_constraint_1, 100, vec![1, 2, 3].into())
            .unwrap();

        store
            .insert(arc_constraint_2, 100, vec![2, 3, 4].into())
            .unwrap();

        assert_eq!(2, store.inner.len());
        assert_eq!(Some(100), store.highest_stored_id(&arc_constraint_1));
        assert_eq!(vec![1, 2, 3], store.get(&arc_constraint_1, 100).unwrap());

        assert_eq!(Some(100), store.highest_stored_id(&arc_constraint_2));
        assert_eq!(vec![2, 3, 4], store.get(&arc_constraint_2, 100).unwrap());
    }

    #[test]
    fn update_with_multiple_arcs() {
        let mut store = TimeSliceHashStore::default();

        let arc_constraint_1 = DhtArc::Arc(0, 2);
        let arc_constraint_2 = DhtArc::Arc(2, 4);

        store
            .insert(arc_constraint_1, 0, vec![1, 2, 3].into())
            .unwrap();
        store
            .insert(arc_constraint_1, 1, vec![2, 3, 4].into())
            .unwrap();
        store
            .insert(arc_constraint_2, 0, vec![3, 2, 1].into())
            .unwrap();
        store
            .insert(arc_constraint_2, 1, vec![4, 3, 2].into())
            .unwrap();

        assert_eq!(2, store.inner.len());
        assert_eq!(Some(1), store.highest_stored_id(&arc_constraint_1));
        assert_eq!(Some(1), store.highest_stored_id(&arc_constraint_2));

        store
            .insert(arc_constraint_1, 0, vec![1, 2, 5].into())
            .unwrap();
        store
            .insert(arc_constraint_2, 1, vec![5, 3, 2].into())
            .unwrap();

        assert_eq!(2, store.inner.len());
        assert_eq!(Some(1), store.highest_stored_id(&arc_constraint_1));
        assert_eq!(vec![1, 2, 5], store.get(&arc_constraint_1, 0).unwrap());
        assert_eq!(vec![2, 3, 4], store.get(&arc_constraint_1, 1).unwrap());

        assert_eq!(Some(1), store.highest_stored_id(&arc_constraint_2));
        assert_eq!(vec![3, 2, 1], store.get(&arc_constraint_2, 0).unwrap());
        assert_eq!(vec![5, 3, 2], store.get(&arc_constraint_2, 1).unwrap());
    }
}
