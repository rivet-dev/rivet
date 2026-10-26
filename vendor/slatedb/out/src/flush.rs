use crate::db::DbInner;
use crate::db_state::{SsTableHandle, SsTableId};
use crate::error::SlateDBError;
#[cfg(test)]
use crate::format::sst::EncodedSsTable;
use crate::iter::RowEntryIterator;
use crate::mem_table::KVTable;
use crate::merge_operator::{MergeOperatorIterator, MergeOperatorRequiredIterator};
use crate::oracle::Oracle;
use crate::reader::DbStateReader;
use crate::retention_iterator::RetentionIterator;
use crate::tablestore::EncodedSsTableWriter;
use bytes::Bytes;
use log::warn;
use std::collections::BTreeMap;
use std::sync::Arc;
use ulid::Ulid;

/// Best-effort cleanup after a writer fails partway through.
///
/// Tells object storage to discard any part of this SST already
/// uploaded. If the abort call itself fails, only logs a warning. The
/// caller returns its original error either way.
async fn abort_writer(writer: &mut EncodedSsTableWriter) {
    if let Err(e) = writer.abort().await {
        warn!("failed to abort sst writer after error [error={:?}]", e);
    }
}

/// One uploaded SST from a memtable flush, tagged with the segment it
/// belongs to (RFC-0024). An empty `prefix` denotes the compatibility-encoded
/// `prefix=""` segment whose state lives in the manifest's top-level tree.
#[derive(Clone)]
pub(crate) struct SegmentedSstHandle {
    pub(crate) prefix: Bytes,
    pub(crate) sst_handle: SsTableHandle,
    pub(crate) encoded_bytes: u64,
}

impl DbInner {
    /// Write this memtable's post-retention entries to object storage.
    ///
    /// Hands the SST to the writer one block at a time, so peak memory
    /// is bounded by the writer's buffer (10 MiB) rather than the whole
    /// encoded SST. An SST that fits in that buffer still stays fully
    /// buffered until close; the saving is for SSTs larger than it.
    ///
    /// With no segment extractor configured, writes one SST. With one
    /// configured, writes one SST per segment prefix that received at
    /// least one entry.
    pub(crate) async fn stream_imm_ssts(
        &self,
        imm_table: Arc<KVTable>,
        min_retention_seq: Option<u64>,
        segment_sst_ids: &BTreeMap<Bytes, Ulid>,
    ) -> Result<Vec<SegmentedSstHandle>, SlateDBError> {
        if self.segment_extractor.is_none() {
            let sst_id = segment_sst_ids
                .get(&Bytes::new())
                .copied()
                .map(SsTableId::from)
                .ok_or(SlateDBError::InvalidDBState)?;
            return Ok(self
                .stream_imm_sst(imm_table, min_retention_seq, sst_id)
                .await?
                .into_iter()
                .map(|(sst_handle, encoded_bytes)| SegmentedSstHandle {
                    prefix: Bytes::new(),
                    sst_handle,
                    encoded_bytes,
                })
                .collect());
        }
        let touched = imm_table.touched_segments();
        if touched.is_empty() {
            if imm_table.is_empty() {
                return Ok(Vec::new());
            }
            return Err(SlateDBError::InvalidDBState);
        }
        self.stream_imm_segment_ssts(imm_table, touched, min_retention_seq, segment_sst_ids)
            .await
    }

    /// Write one SST for the whole memtable, with no segment extractor.
    ///
    /// Each finished block is handed to the writer as it is produced.
    /// The writer buffers blocks and only starts uploading once its
    /// buffer fills, so a small SST is held in memory until close.
    ///
    /// On any error, aborts this SST's upload before returning the
    /// error. This discards any blocks already sent to object storage
    /// for this attempt.
    async fn stream_imm_sst(
        &self,
        imm_table: Arc<KVTable>,
        min_retention_seq: Option<u64>,
        sst_id: SsTableId,
    ) -> Result<Option<(SsTableHandle, u64)>, SlateDBError> {
        let mut writer = self.table_store.table_writer(sst_id, Some(Bytes::new()));
        let mut iter = match self.iter_imm_table(imm_table, min_retention_seq).await {
            Ok(iter) => iter,
            Err(e) => {
                abort_writer(&mut writer).await;
                return Err(e);
            }
        };
        let mut any = false;
        loop {
            match iter.next().await {
                Ok(Some(entry)) => {
                    if let Err(e) = writer.add(entry).await {
                        abort_writer(&mut writer).await;
                        return Err(e);
                    }
                    any = true;
                    tokio::task::coop::consume_budget().await;
                }
                Ok(None) => break,
                Err(e) => {
                    abort_writer(&mut writer).await;
                    return Err(e);
                }
            }
        }
        if !any {
            return Ok(None);
        }
        Ok(Some(writer.close().await?))
    }

    /// Write one SST per touched segment, block by block.
    ///
    /// Walks the memtable's entries and the segment prefixes together.
    /// Both are sorted, so every entry for one prefix arrives before
    /// the next prefix starts. Closes the current segment's SST and
    /// opens the next one each time an entry's key no longer matches
    /// the current prefix.
    ///
    /// On error, aborts only the segment being written when the error
    /// happened. Segments already closed and uploaded earlier in this
    /// call stay uploaded.
    async fn stream_imm_segment_ssts(
        &self,
        imm_table: Arc<KVTable>,
        touched_segments: std::collections::BTreeSet<Bytes>,
        min_retention_seq: Option<u64>,
        segment_sst_ids: &BTreeMap<Bytes, Ulid>,
    ) -> Result<Vec<SegmentedSstHandle>, SlateDBError> {
        let mut entries = self.iter_imm_table(imm_table, min_retention_seq).await?;
        let mut seg_iter = touched_segments.into_iter();
        let mut current_prefix = seg_iter
            .next()
            .expect("touched_segments non-empty in this branch");
        let mut current_id = segment_sst_ids
            .get(&current_prefix)
            .copied()
            .ok_or(SlateDBError::InvalidDBState)?;
        let mut current_writer = self
            .table_store
            .table_writer(SsTableId::from(current_id), Some(current_prefix.clone()));
        let mut current_has_entry = false;
        let mut out: Vec<SegmentedSstHandle> = Vec::new();
        loop {
            let entry = match entries.next().await {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(e) => {
                    abort_writer(&mut current_writer).await;
                    return Err(e);
                }
            };
            // entry's key no longer matches current_prefix: close this
            // segment's SST (if it received any entries) and move on
            // to the next prefix.
            while !entry.key.starts_with(current_prefix.as_ref()) {
                if current_has_entry {
                    let (sst_handle, encoded_bytes) = current_writer.close().await?;
                    out.push(SegmentedSstHandle {
                        prefix: current_prefix,
                        sst_handle,
                        encoded_bytes,
                    });
                }
                current_prefix = seg_iter.next().expect(
                    "entry key has no matching prefix in touched_segments — \
                         extractor output inconsistent with recorded set",
                );
                current_id = segment_sst_ids
                    .get(&current_prefix)
                    .copied()
                    .ok_or(SlateDBError::InvalidDBState)?;
                current_writer = self
                    .table_store
                    .table_writer(SsTableId::from(current_id), Some(current_prefix.clone()));
                current_has_entry = false;
            }
            if let Err(e) = current_writer.add(entry).await {
                abort_writer(&mut current_writer).await;
                return Err(e);
            }
            current_has_entry = true;
            tokio::task::coop::consume_budget().await;
        }
        if current_has_entry {
            let (sst_handle, encoded_bytes) = current_writer.close().await?;
            out.push(SegmentedSstHandle {
                prefix: current_prefix,
                sst_handle,
                encoded_bytes,
            });
        }
        Ok(out)
    }

    /// Write `encoded_sst` to object storage at `id` and advance the
    /// monotonic durable tick from `imm_table`.
    #[cfg(test)]
    pub(crate) async fn upload_sst(
        &self,
        id: &SsTableId,
        encoded_sst: &EncodedSsTable,
        segment: Bytes,
    ) -> Result<SsTableHandle, SlateDBError> {
        let handle = self
            .table_store
            .write_sst(id, encoded_sst, Some(segment))
            .await?;
        Ok(handle)
    }

    /// Test helper: build L0 SSTs from `imm_table` via the segment-aware
    /// path ([`Self::stream_imm_ssts`]) and upload each one with a freshly
    /// allocated [`db_state::SsTableId`]. Returns the resulting
    /// handles in the same order as the segments. Without an extractor the
    /// result is at most one handle; an empty Vec means retention pruned
    /// every entry.
    #[cfg(test)]
    pub(crate) async fn flush_l0_for_test(
        &self,
        imm_table: Arc<KVTable>,
    ) -> Result<Vec<SsTableHandle>, SlateDBError> {
        use crate::utils::IdGenerator;
        // Tests that construct an `imm_table` outside the write path
        // must call `KVTable::record_touched_segments` themselves
        // before dispatching here.
        let mut prefixes = imm_table.touched_segments();
        if prefixes.is_empty() {
            prefixes.insert(Bytes::new());
        }
        let segment_sst_ids: BTreeMap<Bytes, Ulid> = prefixes
            .into_iter()
            .map(|p| (p, self.rand.rng().gen_ulid(self.system_clock.as_ref())))
            .collect();
        let min_retention_seq = self.compute_min_retention_seq();
        let segments = self
            .stream_imm_ssts(imm_table, min_retention_seq, &segment_sst_ids)
            .await?;
        Ok(segments.into_iter().map(|s| s.sst_handle).collect())
    }

    /// Compute the retention boundary for this flush.
    ///
    /// The boundary is the lowest sequence number this flush must keep
    /// a version for. It is the minimum of three values: the durable
    /// watermark, the oldest open snapshot's sequence, and the oldest
    /// open transaction's sequence.
    ///
    /// Remote readers (`DurabilityLevel::Remote`) cap visibility at the
    /// durable watermark, so at least one version at or below the
    /// boundary must survive for each key. Otherwise a remote reader
    /// skips a newer non-durable version and falls back to an even
    /// older value.
    ///
    /// The read is not atomic. A snapshot or transaction may open or
    /// close between the reads of the three inputs. Taking the minimum
    /// still yields a safe boundary, so the race is acceptable.
    ///
    /// Call this once per flush attempt. Reuse the same value on every
    /// retry of that attempt. The three inputs can change between
    /// retries; reading a fresh value on each retry could make each
    /// retry keep a different set of entries.
    pub(crate) fn compute_min_retention_seq(&self) -> Option<u64> {
        let durable_seq = self.oracle.last_remote_persisted_seq();
        [
            Some(durable_seq),
            self.snapshot_manager.min_active_seq(),
            self.txn_manager.min_active_seq(),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    async fn iter_imm_table(
        &self,
        imm_table: Arc<KVTable>,
        min_retention_seq: Option<u64>,
    ) -> Result<RetentionIterator<Box<dyn RowEntryIterator>>, SlateDBError> {
        let state = self.state.read().view();

        let merge_iter = if let Some(merge_operator) = self.flush_merge_operator.clone() {
            Box::new(MergeOperatorIterator::new(
                merge_operator,
                imm_table.iter(),
                false,
                min_retention_seq,
            ))
        } else {
            Box::new(MergeOperatorRequiredIterator::new(imm_table.iter()))
                as Box<dyn RowEntryIterator>
        };
        let mut iter = RetentionIterator::new(
            merge_iter,
            None,
            min_retention_seq,
            false,
            imm_table.last_tick(),
            self.system_clock.clone(),
            Arc::new(state.core().sequence_tracker.clone()),
            None,
        )
        .await?;
        iter.init().await?;
        Ok(iter)
    }
}

#[cfg(test)]
mod tests {
    use crate::block_iterator::BlockIteratorLatest;
    use crate::db::Db;
    use crate::db_state::SsTableHandle;
    use crate::error::SlateDBError;
    use crate::error::SlateDBError::MergeOperatorMissing;
    use crate::iter::RowEntryIterator;
    use crate::mem_table::WritableKVTable;
    use crate::merge_operator::{MERGE_OPERATOR_FLUSH_PATH, MERGE_OPERATOR_READ_PATH};
    use crate::object_store::memory::InMemory;
    use crate::test_utils::{
        lookup_merge_operator_operands, FixedThreeBytePrefixExtractor, StringConcatMergeOperator,
    };
    use crate::types::{RowEntry, ValueDeletable};
    use bytes::Bytes;
    use rstest::rstest;
    use slatedb_common::metrics::test_recorder_helper;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use ulid::Ulid;

    fn preallocate_ids(prefixes: impl IntoIterator<Item = Bytes>) -> BTreeMap<Bytes, Ulid> {
        prefixes.into_iter().map(|p| (p, Ulid::new())).collect()
    }

    async fn setup_test_db_with_merge_operator() -> Db {
        setup_test_db(true).await
    }

    async fn setup_test_db_without_merge_operator() -> Db {
        setup_test_db(false).await
    }

    async fn setup_test_db(set_merge_operator: bool) -> Db {
        let object_store: Arc<dyn object_store::ObjectStore> = Arc::new(InMemory::new());
        let builder = Db::builder("/tmp/test_flush_unsegmented_sst", object_store);
        let builder = if set_merge_operator {
            builder.with_merge_operator(Arc::new(StringConcatMergeOperator))
        } else {
            builder
        };
        builder.build().await.unwrap()
    }

    async fn verify_sst(
        db: &Db,
        sst_handle: &SsTableHandle,
        entries: &[(Bytes, u64, ValueDeletable)],
    ) {
        let index = db
            .inner
            .table_store
            .read_index(
                sst_handle,
                true,
                Some(Bytes::new()),
                &crate::reader::ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        let block_count = index.borrow().block_meta().len();
        let blocks = db
            .inner
            .table_store
            .read_blocks(sst_handle, 0..block_count, Some(Bytes::new()))
            .await
            .unwrap();
        let mut found_entries = Vec::new();
        for block in blocks {
            let mut block_iter = BlockIteratorLatest::new_ascending(block);
            block_iter.init().await.unwrap();

            while let Some(entry) = block_iter.next().await.unwrap() {
                found_entries.push((entry.key.clone(), entry.seq, entry.value.clone()));
            }
        }
        assert_eq!(entries.len(), found_entries.len());
        for i in 0..found_entries.len() {
            let (actual_key, actual_seq, actual_value) = &found_entries[i];
            let (expected_key, expected_seq, expected_value) = &entries[i];
            assert_eq!(expected_key, actual_key);
            assert_eq!(expected_seq, actual_seq);
            assert_eq!(expected_value, actual_value);
        }
    }

    struct FlushImmTableTestCase {
        min_active_seq: u64,
        row_entries: Vec<RowEntry>,
        expected_entries: Vec<(Bytes, u64, ValueDeletable)>,
    }

    #[rstest]
    #[case::flush_empty_table(FlushImmTableTestCase {
        min_active_seq: 0,
        row_entries: vec![],
        expected_entries: vec![],
    })]
    #[case::flush_single_entry(FlushImmTableTestCase {
        min_active_seq: 0,
        row_entries: vec![
            RowEntry::new_value(b"key1", b"value1", 1),
        ],
        expected_entries: vec![
            (Bytes::from("key1"), 1, ValueDeletable::Value(Bytes::from("value1"))),
        ],
    })]
    #[case::flush_multiple_unique_keys(FlushImmTableTestCase {
        min_active_seq: 0,
        row_entries: vec![
            RowEntry::new_value(b"key1", b"value1", 1),
            RowEntry::new_value(b"key2", b"value2", 2),
            RowEntry::new_value(b"key3", b"value3", 3),
        ],
        expected_entries: vec![
            (Bytes::from("key1"), 1, ValueDeletable::Value(Bytes::from("value1"))),
            (Bytes::from("key2"), 2, ValueDeletable::Value(Bytes::from("value2"))),
            (Bytes::from("key3"), 3, ValueDeletable::Value(Bytes::from("value3"))),
        ],
    })]
    #[case::flush_all_seqs(FlushImmTableTestCase {
        min_active_seq: 0,
        row_entries: vec![
            RowEntry::new_value(&Bytes::from("key"), b"value1", 1),
            RowEntry::new_value(&Bytes::from("key"), b"value3", 3),
            RowEntry::new_value(&Bytes::from("key"), b"value2", 2),
        ],
        expected_entries: vec![
            (Bytes::from("key"), 3, ValueDeletable::Value(Bytes::from("value3"))),
            (Bytes::from("key"), 2, ValueDeletable::Value(Bytes::from("value2"))),
            (Bytes::from("key"), 1, ValueDeletable::Value(Bytes::from("value1"))),
        ],
    })]
    #[case::flush_some_highest_seqs(FlushImmTableTestCase {
        min_active_seq: 2,
        row_entries: vec![
            RowEntry::new_value(&Bytes::from("key"), b"value1", 1),
            RowEntry::new_value(&Bytes::from("key"), b"value3", 3),
            RowEntry::new_value(&Bytes::from("key"), b"value2", 2),
        ],
        expected_entries: vec![
            (Bytes::from("key"), 3, ValueDeletable::Value(Bytes::from("value3"))),
            (Bytes::from("key"), 2, ValueDeletable::Value(Bytes::from("value2"))),
        ],
    })]
    #[case::flush_only_highest_seq(FlushImmTableTestCase {
        min_active_seq: 3,
        row_entries: vec![
            RowEntry::new_value(&Bytes::from("key"), b"value1", 1),
            RowEntry::new_value(&Bytes::from("key"), b"value3", 3),
            RowEntry::new_value(&Bytes::from("key"), b"value2", 2),
        ],
        expected_entries: vec![
            (Bytes::from("key"), 3, ValueDeletable::Value(Bytes::from("value3")))
        ],
    })]
    #[case::flush_highest_seqs_multiple_key(FlushImmTableTestCase {
        min_active_seq: 6,
        row_entries: vec![
            RowEntry::new_value(&Bytes::from("key1"), b"value1", 1),
            RowEntry::new_value(&Bytes::from("key1"), b"value2", 2),
            RowEntry::new_value(&Bytes::from("key2"), b"value3", 3),
            RowEntry::new_value(&Bytes::from("key3"), b"value4", 4),
            RowEntry::new_value(&Bytes::from("key1"), b"value5", 5),
            RowEntry::new_value(&Bytes::from("key2"), b"value6", 6),
        ],
        expected_entries: vec![
            // This is the expected results, because for each key slate needs to
            // a value at or before the min_active_seq
            // (see retention_iterator for more details)
            (Bytes::from("key1"), 5, ValueDeletable::Value(Bytes::from("value5"))),
            (Bytes::from("key2"), 6, ValueDeletable::Value(Bytes::from("value6"))),
            (Bytes::from("key3"), 4, ValueDeletable::Value(Bytes::from("value4"))),
        ],
    })]
    #[case::flush_tombstones(FlushImmTableTestCase {
        min_active_seq: 5,
        row_entries: vec![
            RowEntry::new_value(&Bytes::from("key1"), b"value1", 1),
            RowEntry::new_tombstone(&Bytes::from("key1"), 2),
            RowEntry::new_tombstone(&Bytes::from("key2"), 3),
            RowEntry::new_tombstone(&Bytes::from("key3"), 4),
            RowEntry::new_value(&Bytes::from("key3"), b"value3", 5),
            RowEntry::new_tombstone(&Bytes::from("key2"), 6),
        ],
        expected_entries: vec![
            (Bytes::from("key1"), 2, ValueDeletable::Tombstone),
            (Bytes::from("key2"), 6, ValueDeletable::Tombstone),
            (Bytes::from("key2"), 3, ValueDeletable::Tombstone),
            (Bytes::from("key3"), 5, ValueDeletable::Value(Bytes::from("value3"))),
        ],
    })]
    #[case::flush_merges_with_earlier_active_seqs(FlushImmTableTestCase {
        min_active_seq: 0,
        row_entries: vec![
            RowEntry::new_merge(&Bytes::from("key1"), b"value1", 1),
            RowEntry::new_value(&Bytes::from("key2"), b"value2", 2),
            RowEntry::new_merge(&Bytes::from("key1"), b"value3", 3),
            RowEntry::new_merge(&Bytes::from("key3"), b"value4", 4),
            RowEntry::new_merge(&Bytes::from("key2"), b"value5", 5),
            RowEntry::new_value(&Bytes::from("key3"), b"value6", 6),
        ],
        expected_entries: vec![
            (Bytes::from("key1"), 3, ValueDeletable::Merge(Bytes::from("value3"))),
            (Bytes::from("key1"), 1, ValueDeletable::Merge(Bytes::from("value1"))),
            (Bytes::from("key2"), 5, ValueDeletable::Merge(Bytes::from("value5"))),
            (Bytes::from("key2"), 2, ValueDeletable::Value(Bytes::from("value2"))),
            (Bytes::from("key3"), 6, ValueDeletable::Value(Bytes::from("value6"))),
            (Bytes::from("key3"), 4, ValueDeletable::Merge(Bytes::from("value4"))),
        ],
    })]
    #[case::flush_merges_and_tombstones(FlushImmTableTestCase {
        min_active_seq: 0,
        row_entries: vec![
            RowEntry::new_merge(&Bytes::from("key1"), b"value1", 1),
            RowEntry::new_value(&Bytes::from("key2"), b"value2", 2),
            RowEntry::new_merge(&Bytes::from("key1"), b"value3", 3),
            RowEntry::new_tombstone(&Bytes::from("key1"), 4),
            RowEntry::new_merge(&Bytes::from("key3"), b"value4", 5),
            RowEntry::new_merge(&Bytes::from("key2"), b"value5", 6),
            RowEntry::new_value(&Bytes::from("key3"), b"value6", 7),
            RowEntry::new_tombstone(&Bytes::from("key3"), 8),
        ],
        expected_entries: vec![
            (Bytes::from("key1"), 4, ValueDeletable::Tombstone),
            (Bytes::from("key1"), 3, ValueDeletable::Merge(Bytes::from("value3"))),
            (Bytes::from("key1"), 1, ValueDeletable::Merge(Bytes::from("value1"))),
            (Bytes::from("key2"), 6, ValueDeletable::Merge(Bytes::from("value5"))),
            (Bytes::from("key2"), 2, ValueDeletable::Value(Bytes::from("value2"))),
            (Bytes::from("key3"), 8, ValueDeletable::Tombstone),
            (Bytes::from("key3"), 7, ValueDeletable::Value(Bytes::from("value6"))),
            (Bytes::from("key3"), 5, ValueDeletable::Merge(Bytes::from("value4"))),
        ],
    })]
    #[case::flush_merges_with_recent_active_seqs(FlushImmTableTestCase {
        min_active_seq: 6,
        row_entries: vec![
            RowEntry::new_merge(&Bytes::from("key1"), b"value1", 1),
            RowEntry::new_value(&Bytes::from("key2"), b"value2", 2),
            RowEntry::new_merge(&Bytes::from("key1"), b"value3", 3),
            RowEntry::new_merge(&Bytes::from("key3"), b"value4", 4),
            RowEntry::new_merge(&Bytes::from("key2"), b"value5", 5),
            RowEntry::new_value(&Bytes::from("key3"), b"value6", 6),
        ],
        expected_entries: vec![
            (Bytes::from("key1"), 3, ValueDeletable::Merge(Bytes::from("value1value3"))),
            (Bytes::from("key2"), 5, ValueDeletable::Value(Bytes::from("value2value5"))),
            (Bytes::from("key3"), 6, ValueDeletable::Value(Bytes::from("value6"))),
        ],
    })]
    #[tokio::test]
    async fn test_flush(#[case] test_case: FlushImmTableTestCase) {
        // Given
        let db = setup_test_db_with_merge_operator().await;
        db.inner
            .snapshot_manager
            .new_snapshot(Some(test_case.min_active_seq));
        // Set durable watermark high so it doesn't interfere with transaction-based retention tests
        db.inner.oracle.advance_durable_seq(u64::MAX);
        let table = WritableKVTable::new();
        let row_entries_length = test_case.row_entries.len();
        for row_entry in test_case.row_entries {
            table.put(row_entry);
        }
        assert_eq!(table.table().metadata().entry_num, row_entries_length);

        // When
        let handles = db
            .inner
            .flush_l0_for_test(table.table().clone())
            .await
            .unwrap();

        // Then
        if test_case.expected_entries.is_empty() {
            assert!(
                handles.is_empty(),
                "expected no SSTs for empty post-retention memtable"
            );
        } else {
            let sst_handle = handles.into_iter().next().expect("expected single SST");
            verify_sst(&db, &sst_handle, &test_case.expected_entries).await;
        }

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn should_record_merge_operator_operands_on_flush_path() {
        let (metrics_recorder, _) = test_recorder_helper();
        let object_store: Arc<dyn object_store::ObjectStore> = Arc::new(InMemory::new());
        let db = Db::builder("/tmp/test_merge_operands_flush", object_store)
            .with_metrics_recorder(metrics_recorder.clone())
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        db.inner.oracle.advance_durable_seq(u64::MAX);

        let table = WritableKVTable::new();
        table.put(RowEntry::new_merge(&Bytes::from("key1"), b"a", 1));
        table.put(RowEntry::new_merge(&Bytes::from("key1"), b"b", 2));

        assert_eq!(
            lookup_merge_operator_operands(metrics_recorder.as_ref(), MERGE_OPERATOR_READ_PATH),
            Some(0)
        );
        assert_eq!(
            lookup_merge_operator_operands(metrics_recorder.as_ref(), MERGE_OPERATOR_FLUSH_PATH,),
            Some(0)
        );

        db.inner
            .flush_l0_for_test(table.table().clone())
            .await
            .unwrap();

        assert_eq!(
            lookup_merge_operator_operands(metrics_recorder.as_ref(), MERGE_OPERATOR_READ_PATH),
            Some(0)
        );
        assert_eq!(
            lookup_merge_operator_operands(metrics_recorder.as_ref(), MERGE_OPERATOR_FLUSH_PATH,),
            // Two raw merge rows produce one intermediate batch result and one
            // final merge_batch call over that result.
            Some(3)
        );

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_err_when_merge_operator_not_set_and_merges_exist() {
        // Given
        let db = setup_test_db_without_merge_operator().await;
        db.inner.snapshot_manager.new_snapshot(Some(0));
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(&Bytes::from("key"), b"value1", 1));
        table.put(RowEntry::new_merge(&Bytes::from("key"), b"value2", 2));

        // When
        db.inner
            .flush_l0_for_test(table.table().clone())
            .await
            .map_or_else(
                |err| match err {
                    MergeOperatorMissing => Ok::<(), SlateDBError>(()),
                    _ => panic!("Should return MergeOperatorMissing error"),
                },
                |_| panic!("Should return MergeOperatorMissing error"),
            )
            .unwrap();
    }

    #[tokio::test]
    async fn test_no_err_merge_operator_not_set_and_no_merges() {
        // Given
        let db = setup_test_db_without_merge_operator().await;
        db.inner.snapshot_manager.new_snapshot(Some(0));
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(&Bytes::from("key1"), b"value1", 1));
        table.put(RowEntry::new_tombstone(&Bytes::from("key2"), 2));

        // When
        db.inner
            .flush_l0_for_test(table.table().clone())
            .await
            .unwrap();
    }

    struct RetentionBoundaryTestCase {
        durable_seq: u64,
        snapshot_seq: Option<u64>,
        txn_seq: Option<u64>,
        expected_entries: Vec<(Bytes, u64, ValueDeletable)>,
    }

    #[rstest]
    #[case::durable_is_min(RetentionBoundaryTestCase {
        durable_seq: 1,
        snapshot_seq: Some(3),
        txn_seq: Some(2),
        expected_entries: vec![
            (Bytes::from("key"), 4, ValueDeletable::Value(Bytes::from("value4"))),
            (Bytes::from("key"), 3, ValueDeletable::Value(Bytes::from("value3"))),
            (Bytes::from("key"), 2, ValueDeletable::Value(Bytes::from("value2"))),
            (Bytes::from("key"), 1, ValueDeletable::Value(Bytes::from("value1"))),
        ],
    })]
    #[case::snapshot_is_min(RetentionBoundaryTestCase {
        durable_seq: 4,
        snapshot_seq: Some(2),
        txn_seq: Some(3),
        expected_entries: vec![
            (Bytes::from("key"), 4, ValueDeletable::Value(Bytes::from("value4"))),
            (Bytes::from("key"), 3, ValueDeletable::Value(Bytes::from("value3"))),
            (Bytes::from("key"), 2, ValueDeletable::Value(Bytes::from("value2"))),
        ],
    })]
    #[case::txn_is_min(RetentionBoundaryTestCase {
        durable_seq: 4,
        snapshot_seq: Some(3),
        txn_seq: Some(2),
        expected_entries: vec![
            (Bytes::from("key"), 4, ValueDeletable::Value(Bytes::from("value4"))),
            (Bytes::from("key"), 3, ValueDeletable::Value(Bytes::from("value3"))),
            (Bytes::from("key"), 2, ValueDeletable::Value(Bytes::from("value2"))),
        ],
    })]
    #[case::snapshot_is_none(RetentionBoundaryTestCase {
        durable_seq: 4,
        snapshot_seq: None,
        txn_seq: Some(2),
        expected_entries: vec![
            (Bytes::from("key"), 4, ValueDeletable::Value(Bytes::from("value4"))),
            (Bytes::from("key"), 3, ValueDeletable::Value(Bytes::from("value3"))),
            (Bytes::from("key"), 2, ValueDeletable::Value(Bytes::from("value2"))),
        ],
    })]
    #[case::txn_is_none(RetentionBoundaryTestCase {
        durable_seq: 4,
        snapshot_seq: Some(3),
        txn_seq: None,
        expected_entries: vec![
            (Bytes::from("key"), 4, ValueDeletable::Value(Bytes::from("value4"))),
            (Bytes::from("key"), 3, ValueDeletable::Value(Bytes::from("value3"))),
        ],
    })]
    #[case::snapshot_and_txn_are_none(RetentionBoundaryTestCase {
        durable_seq: 4,
        snapshot_seq: None,
        txn_seq: None,
        expected_entries: vec![
            (Bytes::from("key"), 4, ValueDeletable::Value(Bytes::from("value4"))),
        ],
    })]
    #[tokio::test]
    async fn should_use_min_of_retention_sources(#[case] test_case: RetentionBoundaryTestCase) {
        let db = setup_test_db_with_merge_operator().await;
        db.inner.oracle.advance_durable_seq(test_case.durable_seq);

        if let Some(snapshot_seq) = test_case.snapshot_seq {
            let (_, started_seq) = db.inner.snapshot_manager.new_snapshot(Some(snapshot_seq));
            assert_eq!(started_seq, snapshot_seq)
        }

        if let Some(txn_seq) = test_case.txn_seq {
            db.inner.oracle.advance_committed_seq(txn_seq);
            let (_, started_seq) = db.inner.txn_manager.new_transaction();
            assert_eq!(started_seq, txn_seq);
        }

        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(&Bytes::from("key"), b"value1", 1));
        table.put(RowEntry::new_value(&Bytes::from("key"), b"value2", 2));
        table.put(RowEntry::new_value(&Bytes::from("key"), b"value3", 3));
        table.put(RowEntry::new_value(&Bytes::from("key"), b"value4", 4));

        let handles = db
            .inner
            .flush_l0_for_test(table.table().clone())
            .await
            .unwrap();
        let sst_handle = handles.into_iter().next().expect("expected single SST");

        verify_sst(&db, &sst_handle, &test_case.expected_entries).await;
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn stream_imm_ssts_reuses_a_pinned_retention_boundary() {
        let db = setup_test_db_with_merge_operator().await;
        db.inner.oracle.advance_durable_seq(4);

        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(&Bytes::from("key"), b"value1", 1));
        table.put(RowEntry::new_value(&Bytes::from("key"), b"value2", 2));
        table.put(RowEntry::new_value(&Bytes::from("key"), b"value3", 3));
        table.put(RowEntry::new_value(&Bytes::from("key"), b"value4", 4));

        let pinned = db.inner.compute_min_retention_seq();
        assert_eq!(pinned, Some(4));

        // Opening a snapshot lowers the live boundary. A flush that reuses
        // its pinned boundary must ignore this change.
        let (_, snapshot_seq) = db.inner.snapshot_manager.new_snapshot(Some(1));
        assert_eq!(snapshot_seq, 1);
        assert_eq!(db.inner.compute_min_retention_seq(), Some(1));

        for _ in 0..2 {
            let ids = preallocate_ids([Bytes::new()]);
            let segments = db
                .inner
                .stream_imm_ssts(table.table().clone(), pinned, &ids)
                .await
                .unwrap();
            let handle = segments.into_iter().next().unwrap().sst_handle;
            verify_sst(
                &db,
                &handle,
                &[(
                    Bytes::from("key"),
                    4,
                    ValueDeletable::Value(Bytes::from("value4")),
                )],
            )
            .await;
        }

        // Streaming with the live boundary (1) keeps every version.
        let live = db.inner.compute_min_retention_seq();
        let ids = preallocate_ids([Bytes::new()]);
        let segments = db
            .inner
            .stream_imm_ssts(table.table().clone(), live, &ids)
            .await
            .unwrap();
        let handle = segments.into_iter().next().unwrap().sst_handle;
        verify_sst(
            &db,
            &handle,
            &[
                (
                    Bytes::from("key"),
                    4,
                    ValueDeletable::Value(Bytes::from("value4")),
                ),
                (
                    Bytes::from("key"),
                    3,
                    ValueDeletable::Value(Bytes::from("value3")),
                ),
                (
                    Bytes::from("key"),
                    2,
                    ValueDeletable::Value(Bytes::from("value2")),
                ),
                (
                    Bytes::from("key"),
                    1,
                    ValueDeletable::Value(Bytes::from("value1")),
                ),
            ],
        )
        .await;

        db.close().await.unwrap();
    }

    async fn setup_test_db_with_extractor(
        path: &str,
        extractor: Arc<dyn crate::prefix_extractor::PrefixExtractor>,
    ) -> Db {
        let object_store: Arc<dyn object_store::ObjectStore> = Arc::new(InMemory::new());
        Db::builder(path, object_store)
            .with_segment_extractor(extractor)
            .build()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn stream_imm_ssts_without_extractor_emits_single_empty_prefix() {
        let db = setup_test_db_without_merge_operator().await;
        db.inner.oracle.advance_durable_seq(u64::MAX);
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(b"k1", b"v1", 1));
        table.put(RowEntry::new_value(b"k2", b"v2", 2));
        let segment_sst_ids = preallocate_ids([Bytes::new()]);

        let ssts = db
            .inner
            .stream_imm_ssts(table.table().clone(), None, &segment_sst_ids)
            .await
            .unwrap();

        assert_eq!(ssts.len(), 1);
        assert!(ssts[0].prefix.is_empty());
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn stream_imm_ssts_with_extractor_yields_empty_vec_when_no_entries() {
        // With an extractor configured, an empty memtable produces no
        // entries and therefore opens no writers — the result is an
        // empty Vec.
        let db = setup_test_db_with_extractor(
            "/tmp/test_stream_imm_ssts_empty",
            Arc::new(FixedThreeBytePrefixExtractor),
        )
        .await;
        db.inner.oracle.advance_durable_seq(u64::MAX);
        let table = WritableKVTable::new();
        let segment_sst_ids = preallocate_ids([]);

        let ssts = db
            .inner
            .stream_imm_ssts(table.table().clone(), None, &segment_sst_ids)
            .await
            .unwrap();

        assert!(ssts.is_empty());
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn stream_imm_ssts_without_extractor_yields_empty_vec_when_no_entries() {
        // Without an extractor configured, an empty memtable also yields
        // an empty Vec — symmetric with the extractor case. Manifest
        // progress (last_l0_seq, replay frontier) advances independently.
        let db = setup_test_db_without_merge_operator().await;
        db.inner.oracle.advance_durable_seq(u64::MAX);
        let table = WritableKVTable::new();
        let segment_sst_ids = preallocate_ids([Bytes::new()]);

        let ssts = db
            .inner
            .stream_imm_ssts(table.table().clone(), None, &segment_sst_ids)
            .await
            .unwrap();

        assert!(ssts.is_empty());
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn stream_imm_ssts_with_extractor_groups_by_prefix() {
        let db = setup_test_db_with_extractor(
            "/tmp/test_stream_imm_ssts_groups",
            Arc::new(FixedThreeBytePrefixExtractor),
        )
        .await;
        db.inner.oracle.advance_durable_seq(u64::MAX);
        let table = WritableKVTable::new();
        // Sorted within and across prefixes.
        table.put(RowEntry::new_value(b"aaa-1", b"v1", 1));
        table.put(RowEntry::new_value(b"aaa-2", b"v2", 2));
        table.put(RowEntry::new_value(b"bbb-1", b"v3", 3));
        table.put(RowEntry::new_value(b"ccc-1", b"v4", 4));
        table.put(RowEntry::new_value(b"ccc-2", b"v5", 5));
        // Production paths (writer / replay) populate the touched
        // set inline; this test bypasses those, so we record the
        // expected prefixes explicitly.
        table.record_touched_segments(std::collections::BTreeSet::from([
            Bytes::from_static(b"aaa"),
            Bytes::from_static(b"bbb"),
            Bytes::from_static(b"ccc"),
        ]));
        let segment_sst_ids = preallocate_ids([
            Bytes::from_static(b"aaa"),
            Bytes::from_static(b"bbb"),
            Bytes::from_static(b"ccc"),
        ]);

        let ssts = db
            .inner
            .stream_imm_ssts(table.table().clone(), None, &segment_sst_ids)
            .await
            .unwrap();

        let prefixes: Vec<&[u8]> = ssts.iter().map(|s| s.prefix.as_ref()).collect();
        assert_eq!(prefixes, vec![&b"aaa"[..], &b"bbb"[..], &b"ccc"[..]]);

        // Each SST is already uploaded; verify it carries exactly its prefix's entries.
        let expected: Vec<Vec<(Bytes, u64, ValueDeletable)>> = vec![
            vec![
                (
                    Bytes::from("aaa-1"),
                    1,
                    ValueDeletable::Value(Bytes::from("v1")),
                ),
                (
                    Bytes::from("aaa-2"),
                    2,
                    ValueDeletable::Value(Bytes::from("v2")),
                ),
            ],
            vec![(
                Bytes::from("bbb-1"),
                3,
                ValueDeletable::Value(Bytes::from("v3")),
            )],
            vec![
                (
                    Bytes::from("ccc-1"),
                    4,
                    ValueDeletable::Value(Bytes::from("v4")),
                ),
                (
                    Bytes::from("ccc-2"),
                    5,
                    ValueDeletable::Value(Bytes::from("v5")),
                ),
            ],
        ];
        for (sst, entries) in ssts.into_iter().zip(expected.into_iter()) {
            verify_sst(&db, &sst.sst_handle, &entries).await;
        }
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn stream_imm_ssts_with_extractor_single_segment_yields_one() {
        let db = setup_test_db_with_extractor(
            "/tmp/test_stream_imm_ssts_single",
            Arc::new(FixedThreeBytePrefixExtractor),
        )
        .await;
        db.inner.oracle.advance_durable_seq(u64::MAX);
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(b"aaa-1", b"v1", 1));
        table.put(RowEntry::new_value(b"aaa-2", b"v2", 2));
        table.record_touched_segments(std::collections::BTreeSet::from([Bytes::from_static(
            b"aaa",
        )]));
        let segment_sst_ids = preallocate_ids([Bytes::from_static(b"aaa")]);

        let ssts = db
            .inner
            .stream_imm_ssts(table.table().clone(), None, &segment_sst_ids)
            .await
            .unwrap();

        assert_eq!(ssts.len(), 1);
        assert_eq!(ssts[0].prefix.as_ref(), b"aaa");
        db.close().await.unwrap();
    }

    /// `stream_imm_ssts` rejects the invariant violation where an
    /// extractor is configured but the memtable has entries with no
    /// recorded prefix set — surfacing what would otherwise be a
    /// silent inconsistency.
    #[tokio::test]
    async fn stream_imm_ssts_rejects_missing_touched_segments() {
        let db = setup_test_db_with_extractor(
            "/tmp/test_stream_imm_ssts_invariant",
            Arc::new(FixedThreeBytePrefixExtractor),
        )
        .await;
        db.inner.oracle.advance_durable_seq(u64::MAX);
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(b"aaa-1", b"v1", 1));
        // Deliberately do NOT call record_touched_segments.
        let segment_sst_ids = preallocate_ids([]);

        let err = match db
            .inner
            .stream_imm_ssts(table.table().clone(), None, &segment_sst_ids)
            .await
        {
            Ok(_) => panic!("expected InvalidDBState for missing touched_segments"),
            Err(e) => e,
        };
        assert!(matches!(err, SlateDBError::InvalidDBState));
        db.close().await.unwrap();
    }
}
