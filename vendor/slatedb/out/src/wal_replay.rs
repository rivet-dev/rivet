use crate::error::SlateDBError;
use crate::manifest::ManifestCore;
use crate::mem_table::WritableKVTable;
use crate::tablestore::TableStore;
#[cfg(test)]
use crate::wal::slatedb::iterator::{
    SlateDbWalIterator, SlateDbWalIteratorOptions, WalIteratorEndBound,
};
#[cfg(test)]
use crate::wal::slatedb::store::WalTableStore;
use crate::wal::WalIterator as WalIteratorTrait;
#[cfg(test)]
use std::ops::Range;
use std::sync::Arc;

pub(crate) struct WalReplayOptions {
    /// The target maximum number of bytes in each returned table. WAL replay only
    /// splits between write batches (all rows of one commit stay in one table), so
    /// a returned table may exceed this if a single write batch is larger.
    pub(crate) max_memtable_bytes: usize,

    /// The minimum seq number to replay. If unset, will replay all
    /// entries after `last_l0_seq` in the manifest.
    pub(crate) min_seq: Option<u64>,
}

impl Default for WalReplayOptions {
    fn default() -> Self {
        Self {
            max_memtable_bytes: 64 * 1024 * 1024,
            min_seq: None,
        }
    }
}

pub(crate) struct ReplayedMemtable {
    pub(crate) table: WritableKVTable,
    pub(crate) last_tick: i64,
    pub(crate) last_seq: u64,
    pub(crate) last_wal_id: u64,
}

pub(crate) struct WalReplayIterator {
    options: WalReplayOptions,
    table_store: Arc<TableStore>,
    wal_iter: Box<dyn WalIteratorTrait>,
    terminal_result: Option<Result<(), SlateDBError>>,
    /// The greatest WAL ID such that it and every WAL file before it in the replay
    /// range are fully applied to returned tables. Tables are tagged with this
    /// conservative watermark so that a table ending mid-file never claims a WAL
    /// file it only partially contains.
    last_consumed_wal_file_id: u64,
    last_tick: i64,
    last_seq: u64,
    min_seq: u64,
}

impl WalReplayIterator {
    #[cfg(test)]
    pub(crate) fn range(
        wal_id_range: Range<u64>,
        db_state: &ManifestCore,
        iterator_options: SlateDbWalIteratorOptions,
        replay_options: WalReplayOptions,
        table_store: Arc<TableStore>,
        wal_store: Arc<WalTableStore>,
    ) -> Result<Self, SlateDBError> {
        let wal_iter = SlateDbWalIterator::range(
            wal_id_range.start,
            WalIteratorEndBound::Exclusive(wal_id_range.end),
            iterator_options,
            wal_store,
        )?;
        Self::for_wal_iterator(Box::new(wal_iter), db_state, replay_options, table_store)
    }

    pub(crate) fn for_wal_iterator(
        wal_iter: Box<dyn WalIteratorTrait>,
        db_state: &ManifestCore,
        options: WalReplayOptions,
        table_store: Arc<TableStore>,
    ) -> Result<Self, SlateDBError> {
        // load the last seq number from manifest, and use it as the starting seq number to avoid
        // replaying the entries that are already in the L0 SST. while replaying the WALs, we'll
        // update the last seq number to the max seq number, and this final `last_seq` will be passed
        // to the db_state for the further writes.
        let min_seq = options.min_seq.unwrap_or(db_state.last_l0_seq);
        let last_seq = db_state.last_l0_seq;
        let last_tick = db_state.last_l0_clock_tick;

        Ok(WalReplayIterator {
            options,
            table_store,
            wal_iter,
            terminal_result: None,
            last_consumed_wal_file_id: db_state.replay_after_wal_id,
            last_tick,
            last_seq,
            min_seq,
        })
    }

    /// Get the next table replayed from the WAL. Replay accumulates write batches
    /// until the returned table reaches [`WalReplayOptions::max_memtable_bytes`].
    /// Tables are only split between write batches — all rows sharing a commit seq
    /// stay in one table, and batches are applied in ascending seq order — so a
    /// returned table may exceed the target when a single write batch is larger.
    ///
    /// The returned table's `last_wal_id` is a conservative watermark: the greatest
    /// WAL ID such that it and every WAL file before it are fully contained in the
    /// tables returned so far. A table that ends mid-file is tagged with the last
    /// fully replayed WAL ID, so replaying from `last_wal_id + 1` and dropping rows
    /// with seq <= the table's `last_seq` never misses or duplicates a commit.
    pub(crate) async fn next(&mut self) -> Result<Option<ReplayedMemtable>, SlateDBError> {
        if let Some(terminal_result) = self.terminal_result.clone() {
            return terminal_result.map(|_v| None);
        }

        let table = WritableKVTable::new();
        let mut applied_any = false;

        loop {
            let writes = match self.wal_iter.next().await {
                Ok(Some(writes)) => writes,
                Ok(None) => {
                    // we've reached the end of iteration, mark the iterator as done
                    self.terminal_result = Some(Ok(()));
                    break;
                }
                // Hold the error back so the write batches already applied to this
                // table are returned first. `DbReader` treats a missing WAL file as
                // the end of the WAL, so rows replayed before the error must not be
                // dropped with it.
                Err(err) => {
                    self.terminal_result = Some(Err(err.into()));
                    break;
                }
            };

            applied_any = true;
            assert!(
                writes.last_consumed_wal_file_id >= self.last_consumed_wal_file_id,
                "WAL iterator moved its consumed file watermark backwards"
            );
            self.last_consumed_wal_file_id = writes.last_consumed_wal_file_id;

            for row_entry in writes.rows {
                // skip the entries that are already in the L0 SST.
                if row_entry.seq <= self.min_seq {
                    continue;
                }

                if let Some(ts) = row_entry.create_ts {
                    self.last_tick = self.last_tick.max(ts);
                }
                self.last_seq = self.last_seq.max(row_entry.seq);
                table.put(row_entry);
            }

            if !table.is_empty() {
                let meta = table.metadata();
                let estimated_bytes = self
                    .table_store
                    .estimate_encoded_size_compacted(meta.entry_num, meta.entries_size_in_bytes);
                if estimated_bytes >= self.options.max_memtable_bytes {
                    break;
                }
            }
        }

        if applied_any {
            // we use the applied_any check here rather than checking non-empty table size to
            // ensure that if we replayed values with a lower seq num we still carry the
            // wal id in an empty replayed memtable
            Ok(Some(ReplayedMemtable {
                table,
                last_tick: self.last_tick,
                last_seq: self.last_seq,
                last_wal_id: self.last_consumed_wal_file_id,
            }))
        } else {
            self.terminal_result
                .clone()
                .expect("applied_any false but no terminal result")
                .map(|_v| None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SlateDbWalIteratorOptions, WalReplayIterator, WalReplayOptions};
    use crate::block_cache_policy::BlockCachePolicy;
    use crate::bytes_range::BytesRange;
    use crate::format::sst::SsTableFormat;
    use crate::iter::{IterationOrder, RowEntryIterator};
    use crate::manifest::ManifestCore;
    use crate::mem_table::WritableKVTable;
    use crate::proptest_util::{rng, sample};
    use crate::tablestore::{TableStore, TableStoreKind};
    use crate::types::RowEntry;
    use crate::wal::slatedb::store::WalTableStore;
    use crate::wal::{WalError, WalIterator as WalIteratorTrait, WalRows};
    use crate::{error::SlateDBError, test_utils};
    use async_trait::async_trait;
    use bytes::Bytes;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use proptest::test_runner::TestRng;
    use rand::Rng;
    use std::cmp::min;
    use std::collections::btree_map::Iter;
    use std::collections::{BTreeMap, VecDeque};
    use std::sync::Arc;

    struct ScriptedWalIterator {
        results: VecDeque<Result<Option<WalRows>, WalError>>,
    }

    #[async_trait]
    impl WalIteratorTrait for ScriptedWalIterator {
        async fn next(&mut self) -> Result<Option<WalRows>, WalError> {
            self.results.pop_front().unwrap_or(Ok(None))
        }
    }

    impl WalReplayIterator {
        async fn all_wal_ids(
            db_state: &ManifestCore,
            options: WalReplayOptions,
            table_store: Arc<TableStore>,
            wal_store: Arc<WalTableStore>,
        ) -> Result<Self, SlateDBError> {
            let wal_id_start = db_state.replay_after_wal_id + 1;
            let wal_id_end = wal_store
                .last_seen_wal_id(db_state.replay_after_wal_id.into())
                .await?
                .value();
            let wal_id_range = wal_id_start..(wal_id_end + 1);
            Self::range(
                wal_id_range,
                db_state,
                SlateDbWalIteratorOptions::default(),
                options,
                table_store,
                wal_store,
            )
        }
    }

    #[tokio::test]
    async fn should_return_replayed_rows_before_repeating_terminal_error() {
        let table_store = test_table_store();
        let first_row = RowEntry::new_value(b"key_001", b"value_001", 1);
        let later_row = RowEntry::new_value(b"key_002", b"value_002", 2);
        let wal_iter = ScriptedWalIterator {
            results: VecDeque::from([
                Ok(Some(WalRows {
                    rows: vec![first_row],
                    last_consumed_wal_file_id: 1,
                })),
                Err(WalError::WalTruncated(2)),
                // A terminal error must prevent the replay iterator from resuming
                // the underlying iterator on later calls.
                Ok(Some(WalRows {
                    rows: vec![later_row],
                    last_consumed_wal_file_id: 3,
                })),
            ]),
        };
        let mut replay_iter = WalReplayIterator::for_wal_iterator(
            Box::new(wal_iter),
            &ManifestCore::new(),
            WalReplayOptions {
                max_memtable_bytes: usize::MAX,
                ..WalReplayOptions::default()
            },
            Arc::clone(&table_store),
        )
        .unwrap();

        let replayed = replay_iter.next().await.unwrap().unwrap();
        assert_eq!(replayed.last_wal_id, 1);
        assert_eq!(replayed.last_seq, 1);
        assert_eq!(replayed.table.metadata().entry_num, 1);

        assert!(matches!(
            replay_iter.next().await,
            Err(SlateDBError::WalTruncated(2))
        ));
        assert!(matches!(
            replay_iter.next().await,
            Err(SlateDBError::WalTruncated(2))
        ));
    }

    #[tokio::test]
    async fn should_repeat_terminal_none_for_wal_replay_iterator() {
        let table_store = test_table_store();
        let wal_iter = ScriptedWalIterator {
            results: VecDeque::from([
                Ok(None),
                // Normal termination must prevent the replay iterator from
                // resuming the underlying iterator on later calls.
                Ok(Some(WalRows {
                    rows: vec![RowEntry::new_value(b"key", b"value", 1)],
                    last_consumed_wal_file_id: 1,
                })),
            ]),
        };
        let mut replay_iter = WalReplayIterator::for_wal_iterator(
            Box::new(wal_iter),
            &ManifestCore::new(),
            WalReplayOptions::default(),
            Arc::clone(&table_store),
        )
        .unwrap();

        assert!(replay_iter.next().await.unwrap().is_none());
        assert!(replay_iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_use_last_consumed_wal_file_id_as_replay_watermark() {
        let table_store = test_table_store();
        let first_row = RowEntry::new_value(b"key_001", &[b'x'; 128], 1);
        let second_row = RowEntry::new_value(b"key_002", &[b'x'; 128], 2);
        let max_memtable_bytes =
            table_store.estimate_encoded_size_compacted(1, first_row.estimated_size());
        let wal_iter = ScriptedWalIterator {
            results: VecDeque::from([
                Ok(Some(WalRows {
                    rows: vec![first_row],
                    // The first batch ends partway through WAL file 1.
                    last_consumed_wal_file_id: 0,
                })),
                Ok(Some(WalRows {
                    rows: vec![second_row],
                    // The second batch consumes the rest of WAL file 1.
                    last_consumed_wal_file_id: 1,
                })),
            ]),
        };
        let mut replay_iter = WalReplayIterator::for_wal_iterator(
            Box::new(wal_iter),
            &ManifestCore::new(),
            WalReplayOptions {
                max_memtable_bytes,
                ..WalReplayOptions::default()
            },
            Arc::clone(&table_store),
        )
        .unwrap();

        let first = replay_iter.next().await.unwrap().unwrap();
        assert_eq!(first.last_wal_id, 0);
        assert_eq!(first.last_seq, 1);

        let second = replay_iter.next().await.unwrap().unwrap();
        assert_eq!(second.last_wal_id, 1);
        assert_eq!(second.last_seq, 2);
        assert!(replay_iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_replay_empty_wal() {
        let (table_store, wal_store) = test_stores();
        write_empty_wal(1, Arc::clone(&wal_store)).await.unwrap();
        let mut replay_iter = WalReplayIterator::all_wal_ids(
            &ManifestCore::new(),
            WalReplayOptions::default(),
            Arc::clone(&table_store),
            Arc::clone(&wal_store),
        )
        .await
        .unwrap();

        let Some(table) = replay_iter.next().await.unwrap() else {
            panic!("Expected empty table to be returned from iterator")
        };

        assert_eq!(table.last_wal_id, 1);
        assert_eq!(table.last_seq, 0);
        assert!(table.table.is_empty());
        assert_eq!(table.last_tick, i64::MIN);
        assert!(replay_iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_replay_zero_byte_wal_fence() {
        let (table_store, wal_store) = test_stores();
        wal_store.write_wal_fence(1.into()).await.unwrap();
        let mut replay_iter = WalReplayIterator::all_wal_ids(
            &ManifestCore::new(),
            WalReplayOptions::default(),
            Arc::clone(&table_store),
            Arc::clone(&wal_store),
        )
        .await
        .unwrap();

        let Some(table) = replay_iter.next().await.unwrap() else {
            panic!("Expected empty table to be returned from iterator")
        };

        assert_eq!(table.last_wal_id, 1);
        assert_eq!(table.last_seq, 0);
        assert!(table.table.is_empty());
        assert_eq!(table.last_tick, i64::MIN);
        assert!(replay_iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_replay_zero_byte_wal_fence_before_real_wal() {
        let (table_store, wal_store) = test_stores();
        wal_store.write_wal_fence(1.into()).await.unwrap();

        let row = RowEntry::new_value(b"key", b"value", 1);
        let mut builder = wal_store.table_builder();
        builder.add(row.clone()).await.unwrap();
        let encoded_sst = builder.build().await.unwrap();
        wal_store.write_sst(2.into(), &encoded_sst).await.unwrap();

        let mut replay_iter = WalReplayIterator::all_wal_ids(
            &ManifestCore::new(),
            WalReplayOptions::default(),
            Arc::clone(&table_store),
            Arc::clone(&wal_store),
        )
        .await
        .unwrap();

        let Some(replayed_table) = replay_iter.next().await.unwrap() else {
            panic!("Expected table to be returned from iterator")
        };
        assert_eq!(replayed_table.last_wal_id, 2);
        assert_eq!(replayed_table.last_seq, 1);

        let mut iter = replayed_table.table.table().iter();
        test_utils::assert_iterator(&mut iter, vec![row]).await;
        assert!(replay_iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_replay_all_entries() {
        let (table_store, wal_store) = test_stores();
        let mut rng = rng::new_test_rng(None);
        let entries = sample::table(&mut rng, 1000, 10);
        let next_wal_id = write_wals(&entries, 1, &mut rng, 200, Arc::clone(&wal_store))
            .await
            .unwrap();

        let mut replay_iter = WalReplayIterator::all_wal_ids(
            &ManifestCore::new(),
            WalReplayOptions::default(),
            Arc::clone(&table_store),
            Arc::clone(&wal_store),
        )
        .await
        .unwrap();

        let Some(replayed_table) = replay_iter.next().await.unwrap() else {
            panic!("Expected table to be returned from iterator")
        };
        assert_eq!(replayed_table.last_wal_id + 1, next_wal_id);

        let mut imm_table_iter = replayed_table.table.table().iter();
        test_utils::assert_ranged_kv_scan(
            &entries,
            &BytesRange::from(..),
            IterationOrder::Ascending,
            &mut imm_table_iter,
        )
        .await;
        assert!(replay_iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_enforce_max_memtable_bytes() {
        let (table_store, wal_store) = test_stores();
        let mut rng = rng::new_test_rng(None);
        let num_entries = 5000;
        let entries = sample::table(&mut rng, num_entries, 10);
        let next_wal_id = write_wals(&entries, 1, &mut rng, 200, Arc::clone(&wal_store))
            .await
            .unwrap();

        let max_memtable_bytes = 1024;
        let mut replay_iter = WalReplayIterator::all_wal_ids(
            &ManifestCore::new(),
            WalReplayOptions {
                max_memtable_bytes,
                ..WalReplayOptions::default()
            },
            Arc::clone(&table_store),
            Arc::clone(&wal_store),
        )
        .await
        .unwrap();

        let full_replayed_table = WritableKVTable::new();
        let mut last_wal_id = 0;
        let mut replayed_entry_count = 0;

        while let Some(replayed_table) = replay_iter.next().await.unwrap() {
            last_wal_id = replayed_table.last_wal_id;
            let metadata = replayed_table.table.metadata();
            replayed_entry_count += metadata.entry_num;

            // The last table may be less than `max_memtable_bytes`.
            if replayed_entry_count < num_entries {
                let estimated_bytes = table_store.estimate_encoded_size_compacted(
                    metadata.entry_num,
                    metadata.entries_size_in_bytes,
                );
                assert!(estimated_bytes >= max_memtable_bytes);
            }

            let mut iter = replayed_table.table.table().iter();
            while let Some(next) = iter.next().await.unwrap() {
                full_replayed_table.put(next);
            }
        }
        assert_eq!(last_wal_id + 1, next_wal_id);

        let mut full_replayed_iter = full_replayed_table.table().iter();
        test_utils::assert_ranged_kv_scan(
            &entries,
            &BytesRange::from(..),
            IterationOrder::Ascending,
            &mut full_replayed_iter,
        )
        .await;
    }

    #[tokio::test]
    async fn should_apply_max_memtable_bytes_at_wal_boundaries() {
        let (table_store, wal_store) = test_stores();
        let wal_entries = [
            vec![RowEntry::new_value(b"key_001", &[b'x'; 128], 1)],
            vec![RowEntry::new_value(b"key_002", &[b'x'; 128], 2)],
            vec![RowEntry::new_value(b"key_003", &[b'x'; 128], 3)],
        ];
        let single_row_size = wal_entries[0][0].estimated_size();
        let max_memtable_bytes =
            table_store.estimate_encoded_size_compacted(1, single_row_size) + 1;

        for (wal_id, entries) in wal_entries.into_iter().enumerate() {
            let mut builder = wal_store.table_builder();
            for entry in entries {
                builder.add(entry).await.unwrap();
            }
            let encoded_sst = builder.build().await.unwrap();
            wal_store
                .write_sst((wal_id as u64 + 1).into(), &encoded_sst)
                .await
                .unwrap();
        }

        let mut replay_iter = WalReplayIterator::all_wal_ids(
            &ManifestCore::new(),
            WalReplayOptions {
                max_memtable_bytes,
                ..WalReplayOptions::default()
            },
            Arc::clone(&table_store),
            Arc::clone(&wal_store),
        )
        .await
        .unwrap();

        let mut replayed_last_wal_ids = Vec::new();
        let mut replayed_table_sizes = Vec::new();
        let mut replayed_seqs = Vec::new();

        while let Some(replayed_table) = replay_iter.next().await.unwrap() {
            replayed_last_wal_ids.push(replayed_table.last_wal_id);
            let metadata = replayed_table.table.metadata();
            replayed_table_sizes.push(table_store.estimate_encoded_size_compacted(
                metadata.entry_num,
                metadata.entries_size_in_bytes,
            ));
            let mut iter = replayed_table.table.table().iter();
            while let Some(next) = iter.next().await.unwrap() {
                replayed_seqs.push(next.seq);
            }
        }

        assert_eq!(replayed_last_wal_ids, vec![2, 3]);
        assert!(
            replayed_table_sizes[0] > max_memtable_bytes,
            "first replayed table should exceed the target rather than split a WAL SST"
        );
        assert_eq!(replayed_seqs, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn should_not_split_one_commit_seq_across_replayed_memtables() {
        let (table_store, wal_store) = test_stores();
        let commit_seq = 42;

        // Simulate one committed write batch. Every row gets the same commit
        // sequence, which means replay must not split these rows into separate
        // memtable layers.
        let entries = (0..8)
            .map(|i| {
                RowEntry::new_value(format!("key_{i:03}").as_bytes(), &[b'x'; 128], commit_seq)
            })
            .collect::<Vec<_>>();

        // Size replayed memtables so one real row fits, but the second row
        // overflows into the next replayed memtable.
        let max_memtable_bytes =
            table_store.estimate_encoded_size_compacted(1, entries[0].estimated_size());

        // Use the real WAL SST builder so the fixture matches WAL flushes.
        let mut builder = wal_store.table_builder();
        for entry in entries {
            builder.add(entry).await.unwrap();
        }
        let encoded_sst = builder.build().await.unwrap();
        wal_store.write_sst(1.into(), &encoded_sst).await.unwrap();

        // Replay the single WAL SST into in-memory tables. If the replay code
        // can split a single commit sequence, it will do so here.
        let mut replay_iter = WalReplayIterator::all_wal_ids(
            &ManifestCore::new(),
            WalReplayOptions {
                max_memtable_bytes,
                ..WalReplayOptions::default()
            },
            Arc::clone(&table_store),
            Arc::clone(&wal_store),
        )
        .await
        .unwrap();

        let mut replayed_seq_ranges = Vec::new();
        while let Some(replayed_table) = replay_iter.next().await.unwrap() {
            let metadata = replayed_table.table.metadata();
            replayed_seq_ranges.push((metadata.first_seq, metadata.last_seq));
        }

        // This guards against producing multiple replayed memtables with the same
        // sequence range, which can make later replay logic treat part of the write
        // batch as already committed.
        assert_eq!(
            replayed_seq_ranges,
            vec![(commit_seq, commit_seq)],
            "WAL replay split one commit seq across replayed memtables: {replayed_seq_ranges:?}"
        );
    }

    #[tokio::test]
    async fn should_replay_memtables_in_sequence_order() {
        let (table_store, wal_store) = test_stores();

        // Write one WAL with entries whose sequence numbers do not match key
        // order. Replay must not expose a later memtable whose sequence range
        // starts before the previous memtable's sequence range ends.
        let entries = vec![
            RowEntry::new_value(b"key_000", &[b'x'; 128], 100),
            RowEntry::new_value(b"key_001", &[b'x'; 128], 10),
            RowEntry::new_value(b"key_002", &[b'x'; 128], 110),
        ];

        // Size replayed memtables so one real row fits, but the second row
        // overflows into the next replayed memtable.
        let max_memtable_bytes =
            table_store.estimate_encoded_size_compacted(1, entries[0].estimated_size());

        // Use the real WAL SST builder so replay sees the same entry order as a
        // flushed WAL.
        let mut builder = wal_store.table_builder();
        for entry in entries {
            builder.add(entry).await.unwrap();
        }
        let encoded_sst = builder.build().await.unwrap();
        wal_store.write_sst(1.into(), &encoded_sst).await.unwrap();

        // Replay the single WAL SST into in-memory tables.
        let mut replay_iter = WalReplayIterator::all_wal_ids(
            &ManifestCore::new(),
            WalReplayOptions {
                max_memtable_bytes,
                ..WalReplayOptions::default()
            },
            Arc::clone(&table_store),
            Arc::clone(&wal_store),
        )
        .await
        .unwrap();

        let mut replayed_seq_ranges = Vec::new();
        while let Some(replayed_table) = replay_iter.next().await.unwrap() {
            let metadata = replayed_table.table.metadata();
            replayed_seq_ranges.push((metadata.first_seq, metadata.last_seq));
        }

        // This guards against returning the seq=10 row in a later replayed
        // memtable after already returning seq=100.
        for adjacent in replayed_seq_ranges.windows(2) {
            let previous_last_seq = adjacent[0].1;
            let later_first_seq = adjacent[1].0;
            assert!(
                later_first_seq >= previous_last_seq,
                "WAL replay returned out-of-order memtable sequence ranges: {replayed_seq_ranges:?}"
            );
        }
    }

    #[tokio::test]
    async fn should_only_replay_wals_after_last_l0_flushed_wal_id() {
        let (table_store, wal_store) = test_stores();
        let mut rng = rng::new_test_rng(None);
        let compacted_entries = sample::table(&mut rng, 1000, 10);
        let mut next_wal_id = 1;

        next_wal_id = write_wals(
            &compacted_entries,
            next_wal_id,
            &mut rng,
            200,
            Arc::clone(&wal_store),
        )
        .await
        .unwrap();

        let replay_after_wal_id = next_wal_id - 1;
        let non_compacted_entries = sample::table(&mut rng, 1000, 10);
        next_wal_id = write_wals(
            &non_compacted_entries,
            next_wal_id,
            &mut rng,
            200,
            Arc::clone(&wal_store),
        )
        .await
        .unwrap();

        let mut db_state = ManifestCore::new();
        db_state.replay_after_wal_id = replay_after_wal_id;
        db_state.next_wal_sst_id = replay_after_wal_id + 1;

        let mut replay_iter = WalReplayIterator::all_wal_ids(
            &db_state,
            WalReplayOptions::default(),
            Arc::clone(&table_store),
            Arc::clone(&wal_store),
        )
        .await
        .unwrap();

        let Some(replayed_table) = replay_iter.next().await.unwrap() else {
            panic!("Expected table to be returned from iterator")
        };
        assert_eq!(replayed_table.last_wal_id + 1, next_wal_id);

        let mut imm_table_iter = replayed_table.table.table().iter();
        test_utils::assert_ranged_kv_scan(
            &non_compacted_entries,
            &BytesRange::from(..),
            IterationOrder::Ascending,
            &mut imm_table_iter,
        )
        .await;
        assert!(replay_iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_replay_wals_after_min_seq() {
        let (table_store, wal_store) = test_stores();
        let mut rng = rng::new_test_rng(None);
        let entries = sample::table(&mut rng, 1000, 10);
        let next_wal_id = write_wals(&entries, 1, &mut rng, 200, Arc::clone(&wal_store))
            .await
            .unwrap();

        // Set min_seq to skip the first half of entries
        let min_seq = 500;
        let mut db_state = ManifestCore::new();
        db_state.last_l0_seq = min_seq;
        db_state.last_l0_clock_tick = 0;

        let mut replay_iter = WalReplayIterator::all_wal_ids(
            &db_state,
            WalReplayOptions::default(),
            Arc::clone(&table_store),
            Arc::clone(&wal_store),
        )
        .await
        .unwrap();

        let Some(replayed_table) = replay_iter.next().await.unwrap() else {
            panic!("Expected table to be returned from iterator")
        };
        assert_eq!(replayed_table.last_wal_id + 1, next_wal_id);

        // Verify that only entries with seq > min_seq are replayed
        let mut imm_table_iter = replayed_table.table.table().iter();
        let mut replayed_entries = BTreeMap::new();
        let mut total = 0;
        while let Some(entry) = imm_table_iter.next().await.unwrap() {
            assert!(entry.seq > min_seq);
            replayed_entries.insert(entry.key.clone(), entry.value);
            total += 1;
        }
        assert_eq!(total, 500);
    }

    fn test_table_store() -> Arc<TableStore> {
        test_stores().0
    }

    fn test_stores() -> (Arc<TableStore>, Arc<WalTableStore>) {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/tmp/test_kv_store");
        let table_store = Arc::new(TableStore::new(
            Arc::clone(&object_store),
            SsTableFormat::default(),
            path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let wal_store = Arc::new(WalTableStore::new(
            object_store,
            SsTableFormat::default(),
            path,
            TableStoreKind::Main,
        ));
        (table_store, wal_store)
    }

    /// Write a sequence of WALs with a random (bounded) number of entries.
    /// Return the ID of the next WAL.
    async fn write_wals(
        entries: &BTreeMap<Bytes, Bytes>,
        next_wal_id: u64,
        rng: &mut TestRng,
        max_wal_entries: usize,
        wal_store: Arc<WalTableStore>,
    ) -> Result<u64, SlateDBError> {
        let mut iter = entries.iter();
        let mut next_seq = 1;
        let mut total_wal_entries = 0;
        let mut next_wal_id = next_wal_id;

        while total_wal_entries < entries.len() {
            let wal_entries = min(
                entries.len() - total_wal_entries,
                rng.random_range(0..max_wal_entries),
            );
            next_seq = write_wal(
                next_wal_id,
                next_seq,
                &mut iter,
                wal_entries,
                Arc::clone(&wal_store),
            )
            .await?;
            next_wal_id += 1;
            total_wal_entries += wal_entries;
        }
        Ok(next_wal_id)
    }

    async fn write_empty_wal(
        wal_id: u64,
        wal_store: Arc<WalTableStore>,
    ) -> Result<(), SlateDBError> {
        let empty_entries = BTreeMap::new();
        let mut empty_iter = empty_entries.iter();
        let _ = write_wal(wal_id, 0, &mut empty_iter, 0, wal_store).await?;
        Ok(())
    }

    async fn write_wal(
        wal_id: u64,
        next_seq: u64,
        entries: &mut Iter<'_, Bytes, Bytes>,
        max_entries: usize,
        wal_store: Arc<WalTableStore>,
    ) -> Result<u64, SlateDBError> {
        let mut builder = wal_store.table_builder();
        let mut next_seq = next_seq;
        let end_seq = next_seq + (max_entries as u64);
        while next_seq < end_seq {
            let Some((key, value)) = entries.next() else {
                break;
            };
            builder
                .add(RowEntry::new_value(key, value, next_seq))
                .await?;
            next_seq += 1;
        }
        let encoded_sst = builder.build().await?;
        wal_store.write_sst(wal_id.into(), &encoded_sst).await?;
        Ok(next_seq)
    }
}
