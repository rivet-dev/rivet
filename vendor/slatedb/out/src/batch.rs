//! # Batch
//!
//! This module contains types for batch operations. A batch operation is a
//! collection of write operations (puts and/or deletes) that are applied
//! atomically to the database.

use crate::config::{MergeOptions, PutOptions};
use crate::db_common::extract_segment_prefix;
use crate::error::SlateDBError;
use crate::iter::{IterationOrder, RowEntryIterator};
use crate::merge_operator::{MergeOperatorIterator, MergeOperatorType};
use crate::prefix_extractor::PrefixExtractor;
use crate::types::{RowEntry, ValueDeletable};
use async_trait::async_trait;
use bytes::Bytes;
use smallvec::{smallvec, SmallVec};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::iter::Peekable;
use std::ops::RangeBounds;

/// A batch of write operations (puts, deletes, and merges). All operations in
/// the batch are applied atomically to the database. Put and delete operations
/// replace earlier operations for the same key, while merge operations are
/// preserved until a later put or delete replaces them.
///
/// # Examples
/// ```rust
/// # async fn run() -> Result<(), slatedb::Error> {
/// #     use std::sync::Arc;
/// #     use slatedb::object_store::memory::InMemory;
///     use slatedb::Db;
///     use slatedb::WriteBatch;
///
///     let object_store = Arc::new(InMemory::new());
///     let db = Db::open("path/to/db", object_store).await?;
///
///     let mut batch = WriteBatch::new();
///     batch.put(b"key1", b"value1");
///     batch.put(b"key2", b"value2");
///     batch.delete(b"key3");
///
///     db.write(batch).await;
///     Ok(())
/// # };
/// # tokio_test::block_on(run());
/// ```
///
/// Note that the `WriteBatch` has an unlimited size. This means that batch
/// writes can exceed `l0_sst_size_bytes` (when `WAL` is disabled). It also
/// means that WAL SSTs could get large if there's a large batch write.
#[derive(Clone, Debug)]
pub struct WriteBatch {
    pub(crate) ops: BTreeMap<Bytes, SmallVec<[WriteOp; 1]>>,
    pub(crate) op_count: usize,
    pub(crate) has_merge_ops: bool,
}

impl Default for WriteBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// A write operation in a batch.
#[derive(PartialEq, Clone)]
pub(crate) enum WriteOp {
    Put(Bytes, PutOptions),
    Delete,
    Merge(Bytes, MergeOptions),
}

impl std::fmt::Debug for WriteOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn trunc(bytes: &Bytes) -> String {
            if bytes.len() > 10 {
                format!("{:?}...", &bytes[..10])
            } else {
                format!("{:?}", bytes)
            }
        }

        match self {
            WriteOp::Put(value, options) => {
                let value = trunc(value);
                write!(f, "Put({value}, {:?})", options)
            }
            WriteOp::Delete => write!(f, "Delete"),
            WriteOp::Merge(value, options) => {
                let value = trunc(value);
                write!(f, "Merge({value}, {:?})", options)
            }
        }
    }
}

impl WriteOp {
    /// Convert WriteOp to RowEntry for queries
    pub(crate) fn to_row_entry(
        &self,
        key: &Bytes,
        seq: u64,
        create_ts: Option<i64>,
        expire_ts: Option<i64>,
    ) -> RowEntry {
        match self {
            WriteOp::Put(value, _options) => {
                // For queries, we don't need to compute expiration time here
                // since these are uncommitted writes. The expiration will be
                // computed when the batch is actually written.
                RowEntry::new(
                    key.clone(),
                    ValueDeletable::Value(value.clone()),
                    seq,
                    create_ts,
                    expire_ts,
                )
            }
            WriteOp::Delete => RowEntry::new(
                key.clone(),
                ValueDeletable::Tombstone,
                seq,
                create_ts,
                expire_ts,
            ),
            WriteOp::Merge(value, _options) => RowEntry::new(
                key.clone(),
                ValueDeletable::Merge(value.clone()),
                seq,
                create_ts,
                expire_ts,
            ),
        }
    }
}

impl WriteBatch {
    pub fn new() -> Self {
        WriteBatch {
            ops: BTreeMap::new(),
            op_count: 0,
            has_merge_ops: false,
        }
    }

    fn assert_kv<K, V>(&self, key: &K, value: &V)
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let key = key.as_ref();
        let value = value.as_ref();

        assert!(!key.is_empty(), "key cannot be empty");
        assert!(
            key.len() <= u32::MAX as usize,
            "key size must be <= u32::MAX"
        );
        assert!(
            value.len() <= u32::MAX as usize,
            "value size must be <= u32::MAX"
        );
    }

    /// Put a key-value pair into the batch. Keys must not be empty.
    ///
    /// # Panics
    /// - if the key is empty
    /// - if the key size is larger than u32::MAX
    /// - if the value size is larger than u32::MAX
    pub fn put<K, V>(&mut self, key: K, value: V)
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.put_with_options(key, value, &PutOptions::default())
    }

    /// Put a key-value pair into the batch. Keys must not be empty.
    ///
    /// # Panics
    /// - if the key is empty
    /// - if the key size is larger than u32::MAX
    /// - if the value size is larger than u32::MAX
    pub fn put_with_options<K, V>(&mut self, key: K, value: V, options: &PutOptions)
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.put_bytes_with_options(
            Bytes::copy_from_slice(key.as_ref()),
            Bytes::copy_from_slice(value.as_ref()),
            options,
        )
    }

    /// Put a key-value pair into the batch using owned [`Bytes`], avoiding
    /// the copies that [`WriteBatch::put`] performs via
    /// `Bytes::copy_from_slice`. Prefer this form when the caller already
    /// holds the data as [`Bytes`] (e.g. from a prior read, a zero-copy
    /// buffer pool, or a client that produces [`Bytes`] directly). Keys must
    /// not be empty.
    ///
    /// # Panics
    /// - if the key is empty
    /// - if the key size is larger than u32::MAX
    /// - if the value size is larger than u32::MAX
    pub fn put_bytes(&mut self, key: Bytes, value: Bytes) {
        self.put_bytes_with_options(key, value, &PutOptions::default())
    }

    /// Put a key-value pair into the batch using owned [`Bytes`] with custom
    /// options. See [`WriteBatch::put_bytes`] for why this form exists.
    ///
    /// # Panics
    /// - if the key is empty
    /// - if the key size is larger than u32::MAX
    /// - if the value size is larger than u32::MAX
    pub fn put_bytes_with_options(&mut self, key: Bytes, value: Bytes, options: &PutOptions) {
        self.assert_kv(&key, &value);

        // put will overwrite the existing key so we can safely
        // remove all previous entries.
        let previous = self
            .ops
            .insert(key, smallvec![WriteOp::Put(value, options.clone())]);
        self.op_count += 1;
        if let Some(previous) = previous {
            self.op_count -= previous.len();
        }
    }

    /// Merge a key-value pair into the batch. Keys must not be empty.
    pub fn merge<K, V>(&mut self, key: K, value: V)
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.merge_with_options(key, value, &MergeOptions::default());
    }

    /// Merge a key-value pair into the batch with custom options.
    pub fn merge_with_options<K, V>(&mut self, key: K, value: V, options: &MergeOptions)
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.assert_kv(&key, &value);

        let key = key.as_ref();
        let value = value.as_ref();
        let op = WriteOp::Merge(Bytes::copy_from_slice(value), options.clone());
        if let Some(ops) = self.ops.get_mut(key) {
            ops.push(op);
        } else {
            self.ops.insert(Bytes::copy_from_slice(key), smallvec![op]);
        }

        self.has_merge_ops = true;
        self.op_count += 1;
    }

    /// Delete a key-value pair into the batch. Keys must not be empty.
    pub fn delete<K: AsRef<[u8]>>(&mut self, key: K) {
        self.assert_kv(&key, &[]);

        let key = Bytes::copy_from_slice(key.as_ref());

        // delete will overwrite the existing key so we can safely
        // remove all previous entries.
        let previous = self.ops.insert(key, smallvec![WriteOp::Delete]);
        self.op_count += 1;
        if let Some(previous) = previous {
            self.op_count -= previous.len();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub(crate) fn has_merge_ops(&self) -> bool {
        self.has_merge_ops
    }

    pub(crate) fn op_count(&self) -> usize {
        self.op_count
    }

    pub(crate) fn keys(&self) -> HashSet<Bytes> {
        self.ops.keys().cloned().collect()
    }

    /// Converts a WriteBatch into a vector of RowEntry objects with
    /// seq and timestamp set, applying the merge operator to any
    /// mergeable entries.
    ///
    /// Also returning the size of keys and values in the batch
    /// after merge iterator and overwrites are collapsed.
    ///
    /// When `extractor` is `Some`, the same iteration also derives
    /// each entry's segment prefix (RFC-0024) and accumulates the
    /// distinct prefixes into a `BTreeSet`. Returns
    /// `EmptySegmentPrefix` if the extractor produces a zero-length
    /// or absent prefix for any key.
    ///
    /// When `extractor` is `None`, the returned prefix set is empty.
    #[allow(clippy::panic)]
    pub(crate) async fn extract_entries(
        &self,
        seq: u64,
        now: i64,
        default_ttl_millis: Option<u64>,
        merger: Option<MergeOperatorType>,
        extractor: Option<&dyn PrefixExtractor>,
    ) -> Result<(Vec<RowEntry>, BTreeSet<Bytes>, u64), SlateDBError> {
        let mut entries_bytes: u64 = 0;
        let mut it: Box<dyn RowEntryIterator> = Box::new(WriteBatchIterator::new(
            self,
            ..,
            IterationOrder::Ascending,
            seq,
            Some(now),
            default_ttl_millis,
        ));
        if self.has_merge_ops() {
            if let Some(ref merge_operator) = merger {
                it = Box::new(MergeOperatorIterator::new(
                    merge_operator.clone(),
                    it,
                    false,
                    None,
                ));
            } else {
                return Err(SlateDBError::MergeOperatorMissing);
            }
        }

        let mut entries: Vec<RowEntry> = Vec::new();
        let mut touched_segments: BTreeSet<Bytes> = BTreeSet::new();
        while let Some(entry) = it.next().await? {
            // Verify that user has not supplied merge entries with different TTLs.
            // This is not allowed. See #1581 for details.
            match entries.last() {
                Some(previous) if previous.key == entry.key => {
                    if previous.expire_ts == entry.expire_ts {
                        panic!("iterator emitted duplicate keys [key={:?}]", entry.key);
                    }
                    return Err(SlateDBError::IncompatibleMergeTtls {
                        key: entry.key.clone(),
                        previous_expire_ts: previous.expire_ts,
                        current_expire_ts: entry.expire_ts,
                    });
                }
                _ => {}
            }
            if let Some(ext) = extractor {
                touched_segments.insert(extract_segment_prefix(ext, &entry.key)?);
            }
            entries_bytes += (entry.key.len() + entry.value.len()) as u64;
            entries.push(entry);
        }
        Ok((entries, touched_segments, entries_bytes))
    }
}

#[cfg(feature = "bench-internal")]
pub mod benches {
    use super::WriteBatch;
    use crate::{Error, MergeOperator, PrefixExtractor, RowEntry};
    use bytes::Bytes;
    use std::collections::{BTreeSet, HashSet};
    use std::sync::Arc;

    pub fn keys(batch: &WriteBatch) -> HashSet<Bytes> {
        batch.keys()
    }

    pub async fn extract_entries(
        batch: &WriteBatch,
        seq: u64,
        now: i64,
        default_ttl_millis: Option<u64>,
        merger: Option<Arc<dyn MergeOperator + Send + Sync>>,
        extractor: Option<&dyn PrefixExtractor>,
    ) -> Result<(Vec<RowEntry>, BTreeSet<Bytes>, u64), Error> {
        batch
            .extract_entries(seq, now, default_ttl_millis, merger, extractor)
            .await
            .map_err(Into::into)
    }
}

/// Iterator over `WriteBatch` entries.
pub(crate) struct WriteBatchIterator {
    iter: Peekable<Box<dyn Iterator<Item = RowEntry> + Send + Sync>>,
    ordering: IterationOrder,
}

impl WriteBatchIterator {
    /// Converts a batch write operation into a [`RowEntry`] with the supplied
    /// sequence number and optional batch-local timestamp metadata.
    ///
    /// This wrapper exists because a [`WriteOp`] stores only the value-level
    /// operation payload while the key lives in the batch map. The iterator is
    /// where that key is available alongside the write options, current time,
    /// and default TTL, so it computes the effective expiration timestamp and
    /// passes both the key and timestamp metadata to [`WriteOp::to_row_entry`].
    fn write_op_to_row_entry(
        key: &Bytes,
        op: &WriteOp,
        seq: u64,
        now: Option<i64>,
        default_ttl_millis: Option<u64>,
    ) -> RowEntry {
        let expire_ts = match (op, now) {
            (WriteOp::Put(_, opts), Some(now)) => opts.expire_ts_from(default_ttl_millis, now),
            (WriteOp::Merge(_, opts), Some(now)) => opts.expire_ts_from(default_ttl_millis, now),
            _ => None,
        };
        op.to_row_entry(key, seq, now, expire_ts)
    }

    pub(crate) fn new(
        batch: &WriteBatch,
        range: impl RangeBounds<Bytes>,
        ordering: IterationOrder,
        seq: u64,
        now: Option<i64>,
        default_ttl_millis: Option<u64>,
    ) -> Self {
        let entries: Vec<RowEntry> = match ordering {
            IterationOrder::Ascending => batch
                .ops
                .range(range)
                .flat_map(|(key, ops)| ops.iter().rev().map(move |op| (key, op)))
                .map(|(key, op)| Self::write_op_to_row_entry(key, op, seq, now, default_ttl_millis))
                .collect(),
            IterationOrder::Descending => batch
                .ops
                .range(range)
                .rev()
                .flat_map(|(key, ops)| ops.iter().rev().map(move |op| (key, op)))
                .map(|(key, op)| Self::write_op_to_row_entry(key, op, seq, now, default_ttl_millis))
                .collect(),
        };

        let iter: Box<dyn Iterator<Item = RowEntry> + Send + Sync> = Box::new(entries.into_iter());

        Self {
            iter: iter.peekable(),
            ordering,
        }
    }
}

#[async_trait]
impl RowEntryIterator for WriteBatchIterator {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        Ok(())
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        Ok(self.iter.next())
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        while let Some(entry) = self.iter.peek() {
            if match self.ordering {
                IterationOrder::Ascending => entry.key.as_ref() < next_key,
                IterationOrder::Descending => entry.key.as_ref() > next_key,
            } {
                self.iter.next();
                // Keep in-memory seeking cooperative.
                tokio::task::coop::consume_budget().await;
            } else {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::bytes_range::BytesRange;
    use crate::config::Ttl;
    use crate::test_utils::assert_iterator;
    use crate::types::RowEntry;

    struct WriteOpTestCase {
        key: Vec<u8>,
        // None is a delete and options will be ignored
        value: Option<Vec<u8>>,
        options: PutOptions,
    }

    fn ops_for_key<'a>(batch: &'a WriteBatch, key: &[u8]) -> &'a [WriteOp] {
        let key = Bytes::copy_from_slice(key);
        batch.ops.get(&key).expect("missing key").as_slice()
    }

    fn only_op<'a>(batch: &'a WriteBatch, key: &[u8]) -> &'a WriteOp {
        let ops = ops_for_key(batch, key);
        assert_eq!(ops.len(), 1);
        &ops[0]
    }

    #[rstest]
    #[case(vec![WriteOpTestCase {
        key: b"key".to_vec(),
        value: Some(b"value".to_vec()),
        options: PutOptions::default(),
    }])]
    #[case(vec![WriteOpTestCase {
        key: b"key".to_vec(),
        value: None,
        options: PutOptions::default(),
    }])]
    #[should_panic(expected = "key cannot be empty")]
    #[case(vec![WriteOpTestCase {
        key: b"".to_vec(),
        value: Some(b"value".to_vec()),
        options: PutOptions::default(),
    }])]
    #[should_panic(expected = "key cannot be empty")]
    #[case(vec![WriteOpTestCase {
        key: b"".to_vec(),
        value: None,
        options: PutOptions::default(),
    }])]
    fn test_put_delete_batch(#[case] test_case: Vec<WriteOpTestCase>) {
        let mut batch = WriteBatch::new();
        let mut expected_ops: BTreeMap<Bytes, SmallVec<[WriteOp; 1]>> = BTreeMap::new();
        for test_case in test_case {
            if let Some(value) = test_case.value {
                batch.put_with_options(
                    test_case.key.as_slice(),
                    value.as_slice(),
                    &test_case.options,
                );
                expected_ops.insert(
                    Bytes::from(test_case.key.clone()),
                    smallvec![WriteOp::Put(Bytes::from(value), test_case.options)],
                );
            } else {
                batch.delete(test_case.key.as_slice());
                expected_ops.insert(
                    Bytes::from(test_case.key.clone()),
                    smallvec![WriteOp::Delete],
                );
            }
        }
        assert_eq!(batch.ops, expected_ops);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_accepts_u32_max_key_and_value_sizes() {
        let bytes = vec![0; u32::MAX as usize];
        WriteBatch::new().assert_kv(&bytes, &bytes);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    #[should_panic(expected = "key size must be <= u32::MAX")]
    fn test_put_rejects_key_larger_than_u32_max() {
        // Keep this separate from test_put_delete_batch: put_with_options copies
        // the key before validation. put_bytes rejects the owned, zero-filled
        // buffer without copying 4 GiB.
        let key = Bytes::from(vec![0; u32::MAX as usize + 1]);
        WriteBatch::new().put_bytes(key, Bytes::from_static(b"value"));
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    #[should_panic(expected = "value size must be <= u32::MAX")]
    fn test_put_rejects_value_larger_than_u32_max() {
        // Keep this separate from test_put_delete_batch: put_with_options copies
        // the value before validation. put_bytes rejects the owned, zero-filled
        // buffer without copying 4 GiB.
        let value = Bytes::from(vec![0; u32::MAX as usize + 1]);
        WriteBatch::new().put_bytes(Bytes::from_static(b"key"), value);
    }

    #[rstest]
    #[case(u16::MAX as usize)]
    #[case(u16::MAX as usize + 1)]
    #[case(1024 * 1024)]
    fn test_large_key_batch_operations(#[case] key_size: usize) {
        let key = vec![b'k'; key_size];
        let value = Bytes::from_static(b"value");
        let mut batch = WriteBatch::new();

        batch.put(&key, &value);
        assert_eq!(
            only_op(&batch, &key),
            &WriteOp::Put(value.clone(), PutOptions::default())
        );

        batch.delete(&key);
        assert_eq!(only_op(&batch, &key), &WriteOp::Delete);

        let mut batch = WriteBatch::new();
        batch.merge(&key, &value);
        assert_eq!(
            only_op(&batch, &key),
            &WriteOp::Merge(value, MergeOptions::default())
        );
    }

    #[test]
    fn test_writebatch_deduplication() {
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.put(b"key1", b"value2"); // Should overwrite previous

        assert_eq!(batch.ops.len(), 1); // Only one entry due to deduplication
        assert_eq!(batch.op_count(), 1);
        let op = only_op(&batch, b"key1");
        match op {
            WriteOp::Put(value, _) => assert_eq!(value.as_ref(), b"value2"),
            _ => panic!("Expected Put operation"),
        }
    }

    #[tokio::test]
    async fn test_writebatch_iterator_basic() {
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.put(b"key3", b"value3");
        batch.put(b"key2", b"value2");
        batch.delete(b"key4");

        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Ascending, u64::MAX, None, None);

        let expected = vec![
            RowEntry::new_value(b"key1", b"value1", u64::MAX),
            RowEntry::new_value(b"key2", b"value2", u64::MAX),
            RowEntry::new_value(b"key3", b"value3", u64::MAX),
            RowEntry::new(
                Bytes::from("key4"),
                ValueDeletable::Tombstone,
                u64::MAX,
                None,
                None,
            ),
        ];

        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn test_writebatch_iterator_range() {
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.put(b"key3", b"value3");
        batch.put(b"key5", b"value5");

        // Test range [key2, key4)
        let mut iter = WriteBatchIterator::new(
            &batch,
            BytesRange::from(Bytes::from_static(b"key2")..Bytes::from_static(b"key4")),
            IterationOrder::Ascending,
            u64::MAX,
            None,
            None,
        );

        let expected = vec![RowEntry::new_value(b"key3", b"value3", u64::MAX)];

        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn test_writebatch_iterator_descending() {
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.put(b"key3", b"value3");
        batch.put(b"key2", b"value2");

        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Descending, u64::MAX, None, None);

        let expected = vec![
            RowEntry::new_value(b"key3", b"value3", u64::MAX),
            RowEntry::new_value(b"key2", b"value2", u64::MAX),
            RowEntry::new_value(b"key1", b"value1", u64::MAX),
        ];

        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn test_writebatch_iterator_seek_ascending() {
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.put(b"key3", b"value3");
        batch.put(b"key5", b"value5");

        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Ascending, u64::MAX, None, None);

        // Seek to key3
        iter.seek(b"key3").await.unwrap();

        // Should get key3 and key5
        let expected = vec![
            RowEntry::new_value(b"key3", b"value3", u64::MAX),
            RowEntry::new_value(b"key5", b"value5", u64::MAX),
        ];

        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn test_writebatch_iterator_seek_descending() {
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.put(b"key3", b"value3");
        batch.put(b"key5", b"value5");

        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Descending, u64::MAX, None, None);

        // Seek to key3 (in descending, we want keys <= key3)
        iter.seek(b"key3").await.unwrap();

        // Should get key3 and key1 (in descending order)
        let expected = vec![
            RowEntry::new_value(b"key3", b"value3", u64::MAX),
            RowEntry::new_value(b"key1", b"value1", u64::MAX),
        ];

        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn test_writebatch_iterator_empty_batch() {
        let batch = WriteBatch::new();
        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Ascending, u64::MAX, None, None);

        let result = iter.next().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_writebatch_iterator_seek_to_nonexistent() {
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.put(b"key3", b"value3");

        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Ascending, u64::MAX, None, None);

        // Seek to key2 (doesn't exist)
        iter.seek(b"key2").await.unwrap();

        // Should get key3 (next available)
        let result = iter.next().await.unwrap();
        assert!(result.is_some());
        let entry = result.unwrap();
        assert_eq!(entry.key, Bytes::from_static(b"key3"));
        assert_eq!(
            entry.value,
            ValueDeletable::Value(Bytes::from_static(b"value3"))
        );
    }

    #[tokio::test]
    async fn test_writebatch_iterator_seek_beyond_end() {
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.put(b"key3", b"value3");

        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Ascending, u64::MAX, None, None);

        // Seek beyond maximum key
        iter.seek(b"key9").await.unwrap();

        // Should be exhausted
        let result = iter.next().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_writebatch_iterator_multiple_tombstones() {
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.delete(b"key2");
        batch.put(b"key3", b"value3");
        batch.delete(b"key4");

        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Ascending, u64::MAX, None, None);

        let expected = vec![
            RowEntry::new_value(b"key1", b"value1", u64::MAX),
            RowEntry::new(
                Bytes::from_static(b"key2"),
                ValueDeletable::Tombstone,
                u64::MAX,
                None,
                None,
            ),
            RowEntry::new_value(b"key3", b"value3", u64::MAX),
            RowEntry::new(
                Bytes::from_static(b"key4"),
                ValueDeletable::Tombstone,
                u64::MAX,
                None,
                None,
            ),
        ];

        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn test_writebatch_iterator_seek_before_first() {
        let mut batch = WriteBatch::new();
        batch.put(b"key2", b"value2");
        batch.put(b"key3", b"value3");

        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Ascending, u64::MAX, None, None);

        // Seek before first key
        iter.seek(b"key1").await.unwrap();

        // Should get key2 (first available)
        let result = iter.next().await.unwrap();
        assert!(result.is_some());
        let entry = result.unwrap();
        assert_eq!(entry.key, Bytes::from_static(b"key2"));
    }

    #[tokio::test]
    async fn test_writebatch_iterator_range_with_tombstones() {
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.delete(b"key2");
        batch.put(b"key3", b"value3");
        batch.delete(b"key4");
        batch.put(b"key5", b"value5");

        // Range [key2, key4) should include tombstones
        let mut iter = WriteBatchIterator::new(
            &batch,
            BytesRange::from(Bytes::from_static(b"key2")..Bytes::from_static(b"key4")),
            IterationOrder::Ascending,
            u64::MAX,
            None,
            None,
        );

        let expected = vec![
            RowEntry::new(
                Bytes::from_static(b"key2"),
                ValueDeletable::Tombstone,
                u64::MAX,
                None,
                None,
            ),
            RowEntry::new_value(b"key3", b"value3", u64::MAX),
        ];

        assert_iterator(&mut iter, expected).await;
    }

    #[test]
    fn should_create_merge_operation_with_default_options() {
        // Given: an empty WriteBatch
        let mut batch = WriteBatch::new();

        // When: adding a merge operation
        batch.merge(b"key1", b"value1");

        // Then: the batch should contain one merge operation with default options
        assert_eq!(batch.ops.len(), 1);
        let op = only_op(&batch, b"key1");
        match op {
            WriteOp::Merge(value, options) => {
                assert_eq!(value.as_ref(), b"value1");
                assert_eq!(options, &MergeOptions::default());
            }
            _ => panic!("Expected Merge operation"),
        }
    }

    #[test]
    fn should_create_merge_operation_with_custom_options() {
        // Given: an empty WriteBatch and custom merge options
        let mut batch = WriteBatch::new();
        let merge_options = MergeOptions {
            ttl: Ttl::ExpireAfterMillis(3_600_000), // 1 hour
        };

        // When: adding a merge operation with custom options
        batch.merge_with_options(b"key1", b"value1", &merge_options);

        // Then: the batch should contain one merge operation with the custom options
        assert_eq!(batch.ops.len(), 1);
        let op = only_op(&batch, b"key1");
        match op {
            WriteOp::Merge(value, options) => {
                assert_eq!(value.as_ref(), b"value1");
                assert_eq!(options.ttl, Ttl::ExpireAfterMillis(3_600_000));
            }
            _ => panic!("Expected Merge operation"),
        }
    }

    #[test]
    fn should_allow_multiple_merges_for_same_key() {
        // Given: an empty WriteBatch
        let mut batch = WriteBatch::new();

        // When: adding multiple merge operations for the same key
        batch.merge(b"key1", b"value1");
        batch.merge(b"key1", b"value2");
        batch.merge(b"key1", b"value3");

        // Then: all merge operations should be preserved (not deduplicated)
        assert_eq!(batch.ops.len(), 1);
        assert_eq!(batch.op_count(), 3);

        // And: all three operations should exist for the key in insertion order
        let ops = ops_for_key(&batch, b"key1");
        let op1 = &ops[0];
        let op2 = &ops[1];
        let op3 = &ops[2];

        match (op1, op2, op3) {
            (WriteOp::Merge(v1, _), WriteOp::Merge(v2, _), WriteOp::Merge(v3, _)) => {
                assert_eq!(v1.as_ref(), b"value1");
                assert_eq!(v2.as_ref(), b"value2");
                assert_eq!(v3.as_ref(), b"value3");
            }
            _ => panic!("Expected all Merge operations"),
        }
    }

    #[test]
    fn should_allow_merges_for_different_keys() {
        // Given: an empty WriteBatch
        let mut batch = WriteBatch::new();

        // When: adding merge operations for different keys
        batch.merge(b"key1", b"value1");
        batch.merge(b"key2", b"value2");
        batch.merge(b"key3", b"value3");

        // Then: all merge operations should be preserved
        assert_eq!(batch.ops.len(), 3);
        assert_eq!(batch.op_count(), 3);

        // And: all keys should exist
        assert!(batch.ops.contains_key(&Bytes::from_static(b"key1")));
        assert!(batch.ops.contains_key(&Bytes::from_static(b"key2")));
        assert!(batch.ops.contains_key(&Bytes::from_static(b"key3")));
    }

    #[test]
    fn should_track_merge_presence() {
        let mut batch = WriteBatch::new();
        assert!(!batch.has_merge_ops());

        batch.merge(b"key1", b"value1");

        assert!(batch.has_merge_ops());
    }

    #[test]
    fn should_remember_merge_presence_when_put_overwrites_merge_ops() {
        let mut batch = WriteBatch::new();
        batch.merge(b"key1", b"value1");
        batch.merge(b"key1", b"value2");
        assert!(batch.has_merge_ops());

        batch.put(b"key1", b"final");

        assert!(batch.has_merge_ops());
        assert_eq!(batch.op_count(), 1);
        match only_op(&batch, b"key1") {
            WriteOp::Put(value, _) => {
                assert_eq!(value.as_ref(), b"final");
            }
            _ => panic!("Expected Put operation"),
        }
    }

    #[test]
    fn should_remember_merge_presence_when_delete_overwrites_merge_ops() {
        let mut batch = WriteBatch::new();
        batch.merge(b"key1", b"value1");
        batch.merge(b"key2", b"value2");
        assert!(batch.has_merge_ops());

        batch.delete(b"key1");

        assert!(batch.has_merge_ops());
        assert_eq!(batch.ops.len(), 2);
        assert_eq!(batch.op_count(), 2);

        match only_op(&batch, b"key1") {
            WriteOp::Delete => {}
            _ => panic!("Expected Delete operation"),
        }

        match only_op(&batch, b"key2") {
            WriteOp::Merge(value, _) => {
                assert_eq!(value.as_ref(), b"value2");
            }
            _ => panic!("Expected Merge operation"),
        }
    }

    #[test]
    fn should_preserve_both_put_and_merge_when_merge_comes_after_put() {
        // Given: a WriteBatch with a put operation
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"put_value");

        // When: adding a merge operation for the same key
        batch.merge(b"key1", b"merge_value");

        // Then: both operations should remain (merge accumulates on top of put)
        assert_eq!(batch.ops.len(), 1);
        assert_eq!(batch.op_count(), 2);
        let ops = ops_for_key(&batch, b"key1");

        // Verify the put operation exists
        let put_op = &ops[0];
        match put_op {
            WriteOp::Put(value, _) => {
                assert_eq!(value.as_ref(), b"put_value");
            }
            _ => panic!("Expected Put operation"),
        }

        // Verify the merge operation exists
        let merge_op = &ops[1];
        match merge_op {
            WriteOp::Merge(value, _) => {
                assert_eq!(value.as_ref(), b"merge_value");
            }
            _ => panic!("Expected Merge operation"),
        }
    }

    #[test]
    fn should_preserve_both_delete_and_merge_when_merge_comes_after_delete() {
        // Given: a WriteBatch with a delete operation
        let mut batch = WriteBatch::new();
        batch.delete(b"key1");

        // When: adding a merge operation for the same key
        batch.merge(b"key1", b"merge_value");

        // Then: both operations should remain (merge accumulates on top of delete)
        assert_eq!(batch.ops.len(), 1);
        assert_eq!(batch.op_count(), 2);
        let ops = ops_for_key(&batch, b"key1");

        // Verify the delete operation exists
        let delete_op = &ops[0];
        match delete_op {
            WriteOp::Delete => {}
            _ => panic!("Expected Delete operation"),
        }

        // Verify the merge operation exists
        let merge_op = &ops[1];
        match merge_op {
            WriteOp::Merge(value, _) => {
                assert_eq!(value.as_ref(), b"merge_value");
            }
            _ => panic!("Expected Merge operation"),
        }
    }

    #[test]
    fn should_deduplicate_merge_when_put_is_added_for_same_key() {
        // Given: a WriteBatch with a merge operation
        let mut batch = WriteBatch::new();
        batch.merge(b"key1", b"merge_value");

        // When: adding a put operation for the same key
        batch.put(b"key1", b"put_value");

        // Then: only the put operation should remain (merge is deduplicated by put)
        assert_eq!(batch.ops.len(), 1);
        assert_eq!(batch.op_count(), 1);

        let op = only_op(&batch, b"key1");
        match op {
            WriteOp::Put(value, _) => {
                assert_eq!(value.as_ref(), b"put_value");
            }
            _ => panic!("Expected Put operation"),
        }
    }

    #[test]
    fn should_deduplicate_merge_when_delete_is_added_for_same_key() {
        // Given: a WriteBatch with a merge operation
        let mut batch = WriteBatch::new();
        batch.merge(b"key1", b"merge_value");

        // When: adding a delete operation for the same key
        batch.delete(b"key1");

        // Then: only the delete operation should remain (merge is deduplicated by delete)
        assert_eq!(batch.ops.len(), 1);
        assert_eq!(batch.op_count(), 1);

        let op = only_op(&batch, b"key1");
        match op {
            WriteOp::Delete => {}
            _ => panic!("Expected Delete operation"),
        }
    }

    #[tokio::test]
    async fn should_iterate_over_mixed_operations_including_merges() {
        // Given: a WriteBatch with mixed operations including multiple merges
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.merge(b"key2", b"merge1");
        batch.merge(b"key2", b"merge2");
        batch.delete(b"key3");

        // When: creating an iterator
        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Ascending, u64::MAX, None, None);

        // Then: the iterator should return all operations in order
        let expected = vec![
            RowEntry::new_value(b"key1", b"value1", u64::MAX),
            RowEntry::new(
                Bytes::from_static(b"key2"),
                ValueDeletable::Merge(Bytes::from_static(b"merge2")),
                u64::MAX,
                None,
                None,
            ),
            RowEntry::new(
                Bytes::from_static(b"key2"),
                ValueDeletable::Merge(Bytes::from_static(b"merge1")),
                u64::MAX,
                None,
                None,
            ),
            RowEntry::new(
                Bytes::from_static(b"key3"),
                ValueDeletable::Tombstone,
                u64::MAX,
                None,
                None,
            ),
        ];

        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_iterate_over_multiple_merges_for_same_key() {
        // Given: a WriteBatch with multiple merges for the same key
        let mut batch = WriteBatch::new();
        batch.merge(b"key1", b"merge1");
        batch.merge(b"key1", b"merge2");
        batch.merge(b"key1", b"merge3");

        // When: creating an iterator
        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Ascending, u64::MAX, None, None);

        // Then: the iterator should return all merge operations
        let expected = vec![
            RowEntry::new(
                Bytes::from_static(b"key1"),
                ValueDeletable::Merge(Bytes::from_static(b"merge3")),
                u64::MAX,
                None,
                None,
            ),
            RowEntry::new(
                Bytes::from_static(b"key1"),
                ValueDeletable::Merge(Bytes::from_static(b"merge2")),
                u64::MAX,
                None,
                None,
            ),
            RowEntry::new(
                Bytes::from_static(b"key1"),
                ValueDeletable::Merge(Bytes::from_static(b"merge1")),
                u64::MAX,
                None,
                None,
            ),
        ];

        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_iterate_over_merges_in_descending_order() {
        // Given: a WriteBatch with merges for different keys
        let mut batch = WriteBatch::new();
        batch.merge(b"key1", b"merge1");
        batch.merge(b"key2", b"merge2");
        batch.merge(b"key1", b"merge3");

        // When: creating a descending iterator
        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Descending, u64::MAX, None, None);

        // Then: the iterator should return operations in descending order
        let expected = vec![
            RowEntry::new(
                Bytes::from_static(b"key2"),
                ValueDeletable::Merge(Bytes::from_static(b"merge2")),
                u64::MAX,
                None,
                None,
            ),
            RowEntry::new(
                Bytes::from_static(b"key1"),
                ValueDeletable::Merge(Bytes::from_static(b"merge3")),
                u64::MAX,
                None,
                None,
            ),
            RowEntry::new(
                Bytes::from_static(b"key1"),
                ValueDeletable::Merge(Bytes::from_static(b"merge1")),
                u64::MAX,
                None,
                None,
            ),
        ];

        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_iterate_over_merges_in_range() {
        // Given: a WriteBatch with merges for different keys
        let mut batch = WriteBatch::new();
        batch.merge(b"key1", b"merge1");
        batch.merge(b"key3", b"merge3");
        batch.merge(b"key5", b"merge5");

        // When: creating an iterator with a range filter
        let mut iter = WriteBatchIterator::new(
            &batch,
            BytesRange::from(Bytes::from_static(b"key2")..Bytes::from_static(b"key4")),
            IterationOrder::Ascending,
            u64::MAX,
            None,
            None,
        );

        // Then: the iterator should only return merges within the range
        let expected = vec![RowEntry::new(
            Bytes::from_static(b"key3"),
            ValueDeletable::Merge(Bytes::from_static(b"merge3")),
            u64::MAX,
            None,
            None,
        )];

        assert_iterator(&mut iter, expected).await;
    }

    #[tokio::test]
    async fn should_seek_to_merge_operations() {
        // Given: a WriteBatch with merges for different keys
        let mut batch = WriteBatch::new();
        batch.merge(b"key1", b"merge1");
        batch.merge(b"key3", b"merge3");
        batch.merge(b"key5", b"merge5");

        // When: creating an iterator and seeking to key3
        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Ascending, u64::MAX, None, None);
        iter.seek(b"key3").await.unwrap();

        // Then: the iterator should return key3 and key5
        let expected = vec![
            RowEntry::new(
                Bytes::from_static(b"key3"),
                ValueDeletable::Merge(Bytes::from_static(b"merge3")),
                u64::MAX,
                None,
                None,
            ),
            RowEntry::new(
                Bytes::from_static(b"key5"),
                ValueDeletable::Merge(Bytes::from_static(b"merge5")),
                u64::MAX,
                None,
                None,
            ),
        ];

        assert_iterator(&mut iter, expected).await;
    }

    // Tests for extract_entries
    struct StringConcatMergeOperator;

    impl crate::merge_operator::MergeOperator for StringConcatMergeOperator {
        fn merge(
            &self,
            _key: &Bytes,
            existing_value: Option<Bytes>,
            operand: Bytes,
        ) -> Result<Bytes, crate::merge_operator::MergeOperatorError> {
            match existing_value {
                Some(base) => {
                    let mut merged = base.to_vec();
                    merged.extend_from_slice(&operand);
                    Ok(Bytes::from(merged))
                }
                None => Ok(operand),
            }
        }
    }

    #[tokio::test]
    async fn should_preserve_interleaved_merge_order_by_key() {
        let mut batch = WriteBatch::new();
        batch.merge(b"key1", b"a");
        batch.merge(b"key2", b"x");
        batch.merge(b"key1", b"b");

        let mut iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Ascending, u64::MAX, None, None);

        let expected = vec![
            RowEntry::new(
                Bytes::from_static(b"key1"),
                ValueDeletable::Merge(Bytes::from_static(b"b")),
                u64::MAX,
                None,
                None,
            ),
            RowEntry::new(
                Bytes::from_static(b"key1"),
                ValueDeletable::Merge(Bytes::from_static(b"a")),
                u64::MAX,
                None,
                None,
            ),
            RowEntry::new(
                Bytes::from_static(b"key2"),
                ValueDeletable::Merge(Bytes::from_static(b"x")),
                u64::MAX,
                None,
                None,
            ),
        ];
        assert_iterator(&mut iter, expected).await;

        let merge_operator =
            Some(std::sync::Arc::new(StringConcatMergeOperator) as MergeOperatorType);
        let (result, _, _) = batch
            .extract_entries(100, 1000, None, merge_operator, None)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, Bytes::from_static(b"key1"));
        assert_eq!(
            result[0].value,
            ValueDeletable::Merge(Bytes::from_static(b"ab"))
        );
        assert_eq!(result[1].key, Bytes::from_static(b"key2"));
        assert_eq!(
            result[1].value,
            ValueDeletable::Merge(Bytes::from_static(b"x"))
        );
    }

    #[tokio::test]
    async fn should_extract_entries_no_merges() {
        // Given: a WriteBatch with no merge operations
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.put(b"key2", b"value2");
        batch.delete(b"key3");

        // When: extracting entries
        let (result, _, _) = batch
            .extract_entries(100, 1000, None, None, None)
            .await
            .unwrap();

        // Then: should return entries for all operations without merging
        let mut entries = result.into_iter().collect::<Vec<_>>();
        entries.sort_by_key(|e| e.key.clone());

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].key, Bytes::from_static(b"key1"));
        assert_eq!(
            entries[0].value,
            ValueDeletable::Value(Bytes::from_static(b"value1"))
        );
        assert_eq!(entries[1].key, Bytes::from_static(b"key2"));
        assert_eq!(
            entries[1].value,
            ValueDeletable::Value(Bytes::from_static(b"value2"))
        );
        assert_eq!(entries[2].key, Bytes::from_static(b"key3"));
        assert_eq!(entries[2].value, ValueDeletable::Tombstone);
    }

    #[tokio::test]
    async fn should_error_extracting_entries_with_merges_without_merge_operator() {
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.merge(b"key2", b"merge1");

        let err = batch
            .extract_entries(100, 1000, None, None, None)
            .await
            .unwrap_err();
        assert!(matches!(err, SlateDBError::MergeOperatorMissing));
    }

    #[tokio::test]
    async fn should_extract_entries_multiple_merges() {
        // Given: a WriteBatch with multiple merge operations for the same key
        let mut batch = WriteBatch::new();
        batch.merge(b"key1", b"merge1");
        batch.merge(b"key1", b"merge2");
        batch.merge(b"key1", b"merge3");

        // When: extracting entries with a merge operator
        let merge_operator =
            Some(std::sync::Arc::new(StringConcatMergeOperator) as MergeOperatorType);
        let (result, _, _) = batch
            .extract_entries(100, 1000, None, merge_operator, None)
            .await
            .unwrap();

        // Then: should return merged entry
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, Bytes::from_static(b"key1"));
        assert_eq!(
            result[0].value,
            ValueDeletable::Merge(Bytes::from_static(b"merge1merge2merge3"))
        );
    }

    #[tokio::test]
    async fn should_extract_same_key_merges_with_same_effective_ttl() {
        let mut batch = WriteBatch::new();
        batch.merge_with_options(
            b"key1",
            b"a",
            &MergeOptions {
                ttl: Ttl::ExpireAfterMillis(3600),
            },
        );
        batch.merge_with_options(
            b"key1",
            b"b",
            &MergeOptions {
                ttl: Ttl::ExpireAtMillis(4600),
            },
        );

        let merge_operator =
            Some(std::sync::Arc::new(StringConcatMergeOperator) as MergeOperatorType);
        let (result, _, _) = batch
            .extract_entries(100, 1000, None, merge_operator, None)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, Bytes::from_static(b"key1"));
        assert_eq!(
            result[0].value,
            ValueDeletable::Merge(Bytes::from_static(b"ab"))
        );
        assert_eq!(result[0].expire_ts, Some(4600));
    }

    #[tokio::test]
    async fn should_error_extracting_same_key_merges_with_different_ttls() {
        let mut batch = WriteBatch::new();
        batch.merge_with_options(
            b"key1",
            b"a",
            &MergeOptions {
                ttl: Ttl::ExpireAfterMillis(3600),
            },
        );
        batch.merge_with_options(
            b"key1",
            b"b",
            &MergeOptions {
                ttl: Ttl::ExpireAfterMillis(7200),
            },
        );

        let merge_operator =
            Some(std::sync::Arc::new(StringConcatMergeOperator) as MergeOperatorType);
        let err = batch
            .extract_entries(100, 1000, None, merge_operator, None)
            .await
            .unwrap_err();

        match err {
            SlateDBError::IncompatibleMergeTtls {
                key,
                previous_expire_ts,
                current_expire_ts,
            } => {
                assert_eq!(key, Bytes::from_static(b"key1"));
                assert_eq!(previous_expire_ts, Some(8200));
                assert_eq!(current_expire_ts, Some(4600));
            }
            other => panic!("expected IncompatibleMergeTtls, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn should_allow_different_keys_with_different_merge_ttls() {
        let mut batch = WriteBatch::new();
        batch.merge_with_options(
            b"key1",
            b"a",
            &MergeOptions {
                ttl: Ttl::ExpireAfterMillis(3600),
            },
        );
        batch.merge_with_options(
            b"key2",
            b"b",
            &MergeOptions {
                ttl: Ttl::ExpireAfterMillis(7200),
            },
        );

        let merge_operator =
            Some(std::sync::Arc::new(StringConcatMergeOperator) as MergeOperatorType);
        let (result, _, _) = batch
            .extract_entries(100, 1000, None, merge_operator, None)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, Bytes::from_static(b"key1"));
        assert_eq!(result[0].expire_ts, Some(4600));
        assert_eq!(result[1].key, Bytes::from_static(b"key2"));
        assert_eq!(result[1].expire_ts, Some(8200));
    }

    #[tokio::test]
    async fn should_error_extracting_put_then_merge_with_different_ttls() {
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"base");
        batch.merge_with_options(
            b"key1",
            b"a",
            &MergeOptions {
                ttl: Ttl::ExpireAfterMillis(7200),
            },
        );

        let merge_operator =
            Some(std::sync::Arc::new(StringConcatMergeOperator) as MergeOperatorType);
        let err = batch
            .extract_entries(100, 1000, None, merge_operator, None)
            .await
            .unwrap_err();

        assert!(matches!(err, SlateDBError::IncompatibleMergeTtls { .. }));
    }

    #[tokio::test]
    async fn should_error_extracting_delete_then_merge_with_different_ttls() {
        let mut batch = WriteBatch::new();
        batch.delete(b"key1");
        batch.merge_with_options(
            b"key1",
            b"a",
            &MergeOptions {
                ttl: Ttl::ExpireAfterMillis(7200),
            },
        );

        let merge_operator =
            Some(std::sync::Arc::new(StringConcatMergeOperator) as MergeOperatorType);
        let err = batch
            .extract_entries(100, 1000, None, merge_operator, None)
            .await
            .unwrap_err();

        assert!(matches!(err, SlateDBError::IncompatibleMergeTtls { .. }));
    }

    #[tokio::test]
    async fn should_extract_entries_delete_then_merge() {
        // Given: a WriteBatch with a delete followed by merges
        let mut batch = WriteBatch::new();
        batch.delete(b"key1");
        batch.merge(b"key1", b"merge1");
        batch.merge(b"key1", b"merge2");

        // When: extracting entries with a merge operator
        let merge_operator =
            Some(std::sync::Arc::new(StringConcatMergeOperator) as MergeOperatorType);
        let (result, _, _) = batch
            .extract_entries(100, 1000, None, merge_operator, None)
            .await
            .unwrap();

        // Then: should return merged entry (delete gets merged with merges)
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, Bytes::from_static(b"key1"));
        // The delete (tombstone) gets converted to empty bytes, then merged
        assert_eq!(
            result[0].value,
            ValueDeletable::Value(Bytes::from_static(b"merge1merge2"))
        );
    }

    #[tokio::test]
    async fn should_extract_entries_value_then_merge() {
        // Given: a WriteBatch with a put followed by merges
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value");
        batch.merge(b"key1", b"merge1");
        batch.merge(b"key1", b"merge2");

        // When: extracting entries with a merge operator
        let merge_operator =
            Some(std::sync::Arc::new(StringConcatMergeOperator) as MergeOperatorType);
        let (result, _, _) = batch
            .extract_entries(100, 1000, None, merge_operator, None)
            .await
            .unwrap();

        // Then: should return merged entry
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, Bytes::from_static(b"key1"));
        // Value gets merged with merges
        assert_eq!(
            result[0].value,
            ValueDeletable::Value(Bytes::from_static(b"valuemerge1merge2"))
        );
    }
}
