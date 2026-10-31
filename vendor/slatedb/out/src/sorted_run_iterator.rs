use crate::bytes_range::BytesRange;
use crate::db_state::{SortedRun, SsTableView};
use crate::db_stats::DbStats;
use crate::error::SlateDBError;
use crate::iter::{IterationOrder, RowEntryIterator};
use crate::sst_iter::{SstIterator, SstIteratorOptions, SstTracingContext, SstView};
use crate::tablestore::TableStore;
use crate::types::RowEntry;
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::VecDeque;
use std::ops::{Bound, RangeBounds};
use std::sync::Arc;

#[derive(Debug)]
enum SortedRunView<'a> {
    Owned(VecDeque<SsTableView>, BytesRange),
    Borrowed(
        VecDeque<&'a SsTableView>,
        (Bound<&'a [u8]>, Bound<&'a [u8]>),
    ),
}

impl<'a> SortedRunView<'a> {
    /// Pops the next table in iteration order, restricting the iteration range
    /// to the table's view range. Tables are stored in ascending key order, so
    /// a descending scan pops from the back. Projected tables (e.g. in cloned
    /// manifests) may have a `visible_range` narrower than the requested range;
    /// tables whose view range does not intersect the requested range are
    /// skipped entirely.
    fn pop_sst(&mut self, order: IterationOrder) -> Option<SstView<'a>> {
        match self {
            SortedRunView::Owned(tables, r) => loop {
                let table = match order {
                    IterationOrder::Ascending => tables.pop_front()?,
                    IterationOrder::Descending => tables.pop_back()?,
                };
                if let Some(view_range) = table.calculate_view_range(r.clone()) {
                    return Some(SstView::Owned(Box::new(table), view_range));
                }
            },
            SortedRunView::Borrowed(tables, r) => loop {
                let table = match order {
                    IterationOrder::Ascending => tables.pop_front()?,
                    IterationOrder::Descending => tables.pop_back()?,
                };
                if let Some(view_range) = table.calculate_view_range(BytesRange::from_slice(*r)) {
                    return Some(SstView::Borrowed(table, view_range));
                }
            },
        }
    }

    pub(crate) async fn build_next_iter(
        &mut self,
        table_store: Arc<TableStore>,
        sst_iterator_options: SstIteratorOptions,
        sst_tracing_context: Option<SstTracingContext>,
        db_stats: Option<DbStats>,
    ) -> Result<Option<SstIterator<'a>>, SlateDBError> {
        let order = sst_iterator_options.order;
        let next_iter = if let Some(view) = self.pop_sst(order) {
            Some(SstIterator::new_with_stats(
                view,
                table_store,
                sst_iterator_options,
                sst_tracing_context,
                db_stats,
            )?)
        } else {
            None
        };
        Ok(next_iter)
    }

    /// The table that [`Self::pop_sst`] will consider next, without popping it.
    fn peek_next_table(&self, order: IterationOrder) -> Option<&SsTableView> {
        match self {
            SortedRunView::Owned(tables, _) => match order {
                IterationOrder::Ascending => tables.front(),
                IterationOrder::Descending => tables.back(),
            },
            SortedRunView::Borrowed(tables, _) => match order {
                IterationOrder::Ascending => tables.front().copied(),
                IterationOrder::Descending => tables.back().copied(),
            },
        }
    }
}

/// The state a descending scan needs on top of the SST chain.
///
/// A descending scan visits the SSTs from last to first, which inverts the
/// sequence order of a key that spans an SST boundary: the earlier SST holds
/// that key's higher sequence numbers but is reached last. The scan
/// therefore gathers every entry of a key before emitting any of it. See
/// [`SortedRunIterator::buffer_next_descending_key`].
#[derive(Default)]
struct DescendingIteratorState {
    /// A complete key, in emission order, ready to be returned.
    current_key_entries: VecDeque<RowEntry>,
    /// The first entry of the next key, read while finding the end of the
    /// current key.
    next_key_first_entry: Option<RowEntry>,
}

impl DescendingIteratorState {
    /// Adds what one table contributed to the current key. An earlier table
    /// holds the higher sequence numbers but is read later, so each
    /// contribution goes in front of the ones already gathered.
    fn prepend_table_entries(&mut self, entries: Vec<RowEntry>) {
        for entry in entries.into_iter().rev() {
            self.current_key_entries.push_front(entry);
        }
    }
}

/// Iterates the SSTs of a sorted run in the order requested by
/// [`SstIteratorOptions::order`], chaining one SST iterator at a time.
///
/// Ascending iteration is a plain chain: the SSTs partition the key space in
/// ascending order, and a key that spans two SSTs is written with its higher
/// sequence numbers in the earlier SST, so concatenating the SST iterators
/// already yields keys ascending and, within a key, sequence numbers
/// descending. Descending iteration needs the extra bookkeeping described on
/// [`DescendingIteratorState`].
pub(crate) struct SortedRunIterator<'a> {
    table_store: Arc<TableStore>,
    sst_iter_options: SstIteratorOptions,
    sst_tracing_context: Option<SstTracingContext>,
    db_stats: Option<DbStats>,
    view: SortedRunView<'a>,
    current_iter: Option<SstIterator<'a>>,
    /// Present only for descending scans; ascending scans chain the SST
    /// iterators directly and carry no state of their own.
    descending_state: Option<DescendingIteratorState>,
    initialized: bool,
}

impl<'a> SortedRunIterator<'a> {
    async fn new(
        view: SortedRunView<'a>,
        table_store: Arc<TableStore>,
        sst_iter_options: SstIteratorOptions,
        sst_tracing_context: Option<SstTracingContext>,
        db_stats: Option<DbStats>,
    ) -> Result<Self, SlateDBError> {
        let descending_state = match sst_iter_options.order {
            IterationOrder::Ascending => None,
            IterationOrder::Descending => Some(DescendingIteratorState::default()),
        };
        let mut res = Self {
            table_store,
            sst_iter_options,
            sst_tracing_context,
            db_stats,
            view,
            current_iter: None,
            descending_state,
            initialized: false,
        };
        res.advance_table().await?;
        Ok(res)
    }

    pub(crate) async fn new_owned<T: RangeBounds<Bytes>>(
        range: T,
        sorted_run: SortedRun,
        table_store: Arc<TableStore>,
        sst_iter_options: SstIteratorOptions,
        sst_tracing_context: Option<SstTracingContext>,
        db_stats: Option<DbStats>,
    ) -> Result<Self, SlateDBError> {
        let range = BytesRange::from(range);
        let tables = sorted_run.into_tables_covering_range(&range);
        let view = SortedRunView::Owned(tables, range);
        SortedRunIterator::new(
            view,
            table_store,
            sst_iter_options,
            sst_tracing_context,
            db_stats,
        )
        .await
    }

    #[allow(dead_code)]
    pub(crate) async fn new_owned_initialized<T: RangeBounds<Bytes>>(
        range: T,
        sorted_run: SortedRun,
        table_store: Arc<TableStore>,
        sst_iter_options: SstIteratorOptions,
    ) -> Result<Self, SlateDBError> {
        SortedRunIterator::new_owned_initialized_with_stats(
            range,
            sorted_run,
            table_store,
            sst_iter_options,
            None,
            None,
        )
        .await
    }

    pub(crate) async fn new_owned_initialized_with_stats<T: RangeBounds<Bytes>>(
        range: T,
        sorted_run: SortedRun,
        table_store: Arc<TableStore>,
        sst_iter_options: SstIteratorOptions,
        sst_tracing_context: Option<SstTracingContext>,
        db_stats: Option<DbStats>,
    ) -> Result<Self, SlateDBError> {
        let mut iter = SortedRunIterator::new_owned(
            range,
            sorted_run,
            table_store,
            sst_iter_options,
            sst_tracing_context,
            db_stats,
        )
        .await?;
        iter.init().await?;
        Ok(iter)
    }

    pub(crate) async fn new_borrowed<T: RangeBounds<&'a [u8]>>(
        range: T,
        sorted_run: &'a SortedRun,
        table_store: Arc<TableStore>,
        sst_iter_options: SstIteratorOptions,
    ) -> Result<Self, SlateDBError> {
        Self::new_borrowed_with_stats(range, sorted_run, table_store, sst_iter_options, None).await
    }

    pub(crate) async fn new_borrowed_with_stats<T: RangeBounds<&'a [u8]>>(
        range: T,
        sorted_run: &'a SortedRun,
        table_store: Arc<TableStore>,
        sst_iter_options: SstIteratorOptions,
        db_stats: Option<DbStats>,
    ) -> Result<Self, SlateDBError> {
        let range = (range.start_bound().cloned(), range.end_bound().cloned());
        let tables = sorted_run.tables_covering_range(BytesRange::from_slice(range));
        let view = SortedRunView::Borrowed(tables, range);
        SortedRunIterator::new(view, table_store, sst_iter_options, None, db_stats).await
    }

    #[cfg(test)]
    pub(crate) async fn new_borrowed_initialized<T: RangeBounds<&'a [u8]>>(
        range: T,
        sorted_run: &'a SortedRun,
        table_store: Arc<TableStore>,
        sst_iter_options: SstIteratorOptions,
    ) -> Result<Self, SlateDBError> {
        let mut iter =
            SortedRunIterator::new_borrowed(range, sorted_run, table_store, sst_iter_options)
                .await?;
        iter.init().await?;
        Ok(iter)
    }

    async fn advance_table(&mut self) -> Result<(), SlateDBError> {
        self.current_iter = self
            .view
            .build_next_iter(
                self.table_store.clone(),
                self.sst_iter_options.clone(),
                self.sst_tracing_context.clone(),
                self.db_stats.clone(),
            )
            .await?;
        if self.initialized {
            if let Some(iter) = self.current_iter.as_mut() {
                iter.init().await?;
            }
        }
        Ok(())
    }

    fn descending_state(&mut self) -> &mut DescendingIteratorState {
        self.descending_state
            .as_mut()
            .expect("descending state is set for descending scans")
    }

    /// Buffers every entry of the next key in emission order.
    ///
    /// A single key may span consecutive SSTs of a sorted run, with the
    /// earlier SST holding the higher sequence numbers. A descending scan
    /// reaches the later SST first, so once the current table runs out on a key
    /// the scan crosses into the preceding table while that table's range still
    /// covers the key, and puts what it finds ahead of what it already has.
    async fn buffer_next_descending_key(&mut self) -> Result<(), SlateDBError> {
        debug_assert!(
            self.descending_state().current_key_entries.is_empty(),
            "buffer_next_descending_key should only be called when the current key is exhausted"
        );

        let mut key: Option<Bytes> = None;
        let mut next_entry = self.descending_state().next_key_first_entry.take();

        // The outer loop iterates over tables, the inner loop over entries of a table.
        loop {
            // Each SST already yields versions newest-first. Buffer its entries
            // so the SST can be prepended at the end in the right order.
            let mut table_entries = Vec::new();
            loop {
                let entry = match next_entry.take() {
                    Some(entry) => entry,
                    None => {
                        let Some(iter) = self.current_iter.as_mut() else {
                            break;
                        };
                        let Some(entry) = iter.next().await? else {
                            break;
                        };
                        entry
                    }
                };

                match &key {
                    None => {
                        key = Some(entry.key.clone());
                        table_entries.push(entry);
                    }
                    Some(current) if current == &entry.key => table_entries.push(entry),
                    Some(_) => {
                        // The table is exhausted on this key. Save the first
                        // entry of the next key for the next call, and prepend
                        // what this table contributed to the current key.
                        self.descending_state().next_key_first_entry = Some(entry);
                        self.descending_state().prepend_table_entries(table_entries);
                        return Ok(());
                    }
                }
            }

            self.descending_state().prepend_table_entries(table_entries);

            // The table is exhausted and contributed nothing, either
            // because it held no entries or because the range excluded all of
            // them. Move on to the next table, or stop once the run is done.
            let Some(key) = key.as_ref() else {
                self.advance_table().await?;
                if self.current_iter.is_none() {
                    return Ok(());
                }
                continue;
            };

            // The table is exhausted on this key. Only the immediately
            // preceding table can continue it.
            let spans_tables = self
                .view
                .peek_next_table(IterationOrder::Descending)
                .is_some_and(|table| table.compacted_effective_range().contains(key));
            if !spans_tables {
                return Ok(());
            }

            // The key continues into the preceding table, which holds its
            // higher sequence numbers. Gather those too, in front of what this
            // table gave.
            self.advance_table().await?;
        }
    }

    async fn next_descending(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        if self.descending_state().current_key_entries.is_empty() {
            self.buffer_next_descending_key().await?;
        }
        Ok(self.descending_state().current_key_entries.pop_front())
    }

    async fn next_ascending(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        while let Some(iter) = &mut self.current_iter {
            if let Some(kv) = iter.next().await? {
                return Ok(Some(kv));
            } else {
                self.advance_table().await?;
            }
        }
        Ok(None)
    }
}

#[async_trait]
impl RowEntryIterator for SortedRunIterator<'_> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        if !self.initialized {
            if let Some(iter) = self.current_iter.as_mut() {
                iter.init().await?;
            }
            self.initialized = true;
        }
        Ok(())
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        if !self.initialized {
            return Err(SlateDBError::IteratorNotInitialized);
        }
        match self.sst_iter_options.order {
            IterationOrder::Ascending => self.next_ascending().await,
            IterationOrder::Descending => self.next_descending().await,
        }
    }

    /// Ascending only. Descending scans reject `seek` at the API boundary, in
    /// [`crate::db_iter::DbIterator::seek`], so this is never reached with a
    /// descending order.
    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        debug_assert!(
            matches!(self.sst_iter_options.order, IterationOrder::Ascending),
            "descending seek is rejected in DbIterator::seek and must not reach a sorted run"
        );
        if !self.initialized {
            return Err(SlateDBError::IteratorNotInitialized);
        }
        while let Some(next_table) = self.view.peek_next_table(IterationOrder::Ascending) {
            if next_table.compacted_effective_start_key() < next_key {
                self.advance_table().await?;
            } else {
                break;
            }
        }
        if let Some(iter) = &mut self.current_iter {
            iter.seek(next_key).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_cache_policy::BlockCachePolicy;
    use crate::bytes_generator::OrderedBytesGenerator;
    use crate::db_state::{SsTableHandle, SsTableId};
    use crate::format::sst::SsTableFormat;
    use crate::proptest_util;
    use crate::proptest_util::sample;
    use crate::tablestore::TableStoreKind;
    use crate::test_utils::{assert_iterator, assert_kv, build_sorted_runs, write_ssts};
    use crate::types::KeyValue;

    use bytes::{BufMut, BytesMut};
    use object_store::path::Path;
    use object_store::{memory::InMemory, ObjectStore};
    use proptest::test_runner::TestRng;
    use rand::distr::uniform::SampleRange;
    use rand::Rng;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_one_sst_sr_iter() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 3,
            ..SsTableFormat::default()
        };
        let table_store = Arc::new(TableStore::new(
            object_store,
            format,
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let mut builder = table_store.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", Some(2), None)
            .await
            .unwrap();
        builder
            .add_value(b"key3", b"value3", Some(3), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        let id = SsTableId::from(ulid::Ulid::new());
        let handle = table_store
            .write_sst(&id, &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let sr = SortedRun::new(0, [SsTableView::identity(handle)]);

        let mut iter = SortedRunIterator::new_owned_initialized(
            ..,
            sr,
            table_store,
            SstIteratorOptions::default(),
        )
        .await
        .unwrap();

        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        assert_eq!(kv.key, b"key1".as_slice());
        assert_eq!(kv.value, b"value1".as_slice());
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        assert_eq!(kv.key, b"key2".as_slice());
        assert_eq!(kv.value, b"value2".as_slice());
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        assert_eq!(kv.key, b"key3".as_slice());
        assert_eq!(kv.value, b"value3".as_slice());
        let kv = iter.next().await.unwrap().map(KeyValue::from);
        assert!(kv.is_none());
    }

    #[tokio::test]
    async fn test_many_sst_sr_iter() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 3,
            ..SsTableFormat::default()
        };
        let table_store = Arc::new(TableStore::new(
            object_store,
            format,
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let mut builder = table_store.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", Some(2), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        let id1 = SsTableId::from(ulid::Ulid::new());
        let handle1 = table_store
            .write_sst(&id1, &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let mut builder = table_store.table_builder();
        builder
            .add_value(b"key3", b"value3", Some(3), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        let id2 = SsTableId::from(ulid::Ulid::new());
        let handle2 = table_store
            .write_sst(&id2, &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let sr = SortedRun::new(
            0,
            [
                SsTableView::identity(handle1),
                SsTableView::identity(handle2),
            ],
        );

        let mut iter = SortedRunIterator::new_owned_initialized(
            ..,
            sr,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap();

        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        assert_eq!(kv.key, b"key1".as_slice());
        assert_eq!(kv.value, b"value1".as_slice());
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        assert_eq!(kv.key, b"key2".as_slice());
        assert_eq!(kv.value, b"value2".as_slice());
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        assert_eq!(kv.key, b"key3".as_slice());
        assert_eq!(kv.value, b"value3".as_slice());
        let kv = iter.next().await.unwrap().map(KeyValue::from);
        assert!(kv.is_none());
    }

    fn descending_options() -> SstIteratorOptions {
        SstIteratorOptions {
            order: IterationOrder::Descending,
            ..SstIteratorOptions::default()
        }
    }

    fn build_table_store() -> Arc<TableStore> {
        Arc::new(TableStore::new(
            Arc::new(InMemory::new()),
            SsTableFormat {
                min_filter_keys: 3,
                ..SsTableFormat::default()
            },
            Path::from(""),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ))
    }

    fn row(key: &[u8], seq: u64) -> RowEntry {
        let value = format!("{}@{seq}", String::from_utf8_lossy(key));
        RowEntry::new_value(key, value.as_bytes(), seq)
    }

    /// Writes each entry set as one SST and returns them as sorted run 0.
    async fn build_sr(table_store: &Arc<TableStore>, sst_sets: Vec<Vec<RowEntry>>) -> SortedRun {
        build_sorted_runs(table_store, &[sst_sets], usize::MAX)
            .await
            .remove(0)
    }

    #[tokio::test]
    async fn test_sr_iter_descending_visits_ssts_from_last_to_first() {
        let table_store = build_table_store();
        let sr = build_sr(
            &table_store,
            vec![
                vec![row(b"key1", 1), row(b"key2", 2)],
                vec![row(b"key3", 3), row(b"key4", 4)],
            ],
        )
        .await;

        let mut iter = SortedRunIterator::new_borrowed_initialized(
            ..,
            &sr,
            table_store.clone(),
            descending_options(),
        )
        .await
        .unwrap();

        assert_iterator(
            &mut iter,
            vec![
                row(b"key4", 4),
                row(b"key3", 3),
                row(b"key2", 2),
                row(b"key1", 1),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_sr_iter_descending_orders_key_spanning_ssts_by_seq() {
        // given: key3 spans both SSTs. Compaction writes keys ascending and
        // seqs descending, so the first SST holds the higher seqs.
        let table_store = build_table_store();
        let sr = build_sr(
            &table_store,
            vec![
                vec![row(b"key1", 1), row(b"key3", 30), row(b"key3", 20)],
                vec![row(b"key3", 10), row(b"key5", 5)],
            ],
        )
        .await;

        // when: scanning descending
        let mut iter = SortedRunIterator::new_borrowed_initialized(
            ..,
            &sr,
            table_store.clone(),
            descending_options(),
        )
        .await
        .unwrap();

        // then: key3's versions stay in descending seq order across the boundary
        assert_iterator(
            &mut iter,
            vec![
                row(b"key5", 5),
                row(b"key3", 30),
                row(b"key3", 20),
                row(b"key3", 10),
                row(b"key1", 1),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_sr_iter_descending_orders_key_spanning_three_ssts_by_seq() {
        let table_store = build_table_store();
        let sr = build_sr(
            &table_store,
            vec![
                vec![row(b"key1", 1), row(b"key3", 30)],
                vec![row(b"key3", 20)],
                vec![row(b"key3", 10), row(b"key5", 5)],
            ],
        )
        .await;

        let mut iter = SortedRunIterator::new_borrowed_initialized(
            ..,
            &sr,
            table_store.clone(),
            descending_options(),
        )
        .await
        .unwrap();

        assert_iterator(
            &mut iter,
            vec![
                row(b"key5", 5),
                row(b"key3", 30),
                row(b"key3", 20),
                row(b"key3", 10),
                row(b"key1", 1),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_sr_iter_descending_respects_range() {
        let table_store = build_table_store();
        let sr = build_sr(
            &table_store,
            vec![
                vec![row(b"key1", 1), row(b"key2", 2)],
                vec![row(b"key3", 3), row(b"key4", 4)],
                vec![row(b"key5", 5), row(b"key6", 6)],
            ],
        )
        .await;

        let mut iter = SortedRunIterator::new_owned_initialized(
            BytesRange::from_ref("key2".."key5"),
            sr,
            table_store.clone(),
            descending_options(),
        )
        .await
        .unwrap();

        assert_iterator(
            &mut iter,
            vec![row(b"key4", 4), row(b"key3", 3), row(b"key2", 2)],
        )
        .await;
    }

    #[tokio::test]
    async fn test_sr_iter_descending_skips_tables_disjoint_with_range() {
        // given: projections that leave the middle table with no visible keys
        // in the query range
        let table_store = build_table_store();
        let mut ssts = Vec::new();
        for entries in [
            vec![row(b"key1", 1), row(b"key2", 2)],
            vec![row(b"key3", 3), row(b"key4", 4)],
            vec![row(b"key5", 5), row(b"key6", 6)],
        ] {
            ssts.extend(write_ssts(&table_store, &entries, usize::MAX).await);
        }
        let [first, middle, last]: [SsTableHandle; 3] = ssts.try_into().unwrap();
        let sr = SortedRun::new(
            0,
            [
                SsTableView::identity(first),
                SsTableView::identity(middle)
                    .with_visible_range(BytesRange::from_ref("key3".."key4")),
                SsTableView::identity(last),
            ],
        );

        let mut iter = SortedRunIterator::new_owned_initialized(
            BytesRange::from_ref("key2".."key6"),
            sr,
            table_store.clone(),
            descending_options(),
        )
        .await
        .unwrap();

        assert_iterator(
            &mut iter,
            vec![row(b"key5", 5), row(b"key3", 3), row(b"key2", 2)],
        )
        .await;
    }

    #[tokio::test]
    async fn test_sr_iter_respects_visible_range() {
        // given: a sorted run whose views carry visible_range restrictions,
        // as produced by manifest projection (e.g. range-restricted clones)
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 3,
            ..SsTableFormat::default()
        };
        let table_store = Arc::new(TableStore::new(
            object_store,
            format,
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let mut builder = table_store.table_builder();
        for i in 1..=4 {
            let key = format!("key{i}");
            let value = format!("value{i}");
            builder
                .add_value(key.as_bytes(), value.as_bytes(), Some(i), None)
                .await
                .unwrap();
        }
        let encoded = builder.build().await.unwrap();
        let id1 = SsTableId::from(ulid::Ulid::new());
        let handle1 = table_store
            .write_sst(&id1, &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let mut builder = table_store.table_builder();
        for i in 5..=8 {
            let key = format!("key{i}");
            let value = format!("value{i}");
            builder
                .add_value(key.as_bytes(), value.as_bytes(), Some(i), None)
                .await
                .unwrap();
        }
        let encoded = builder.build().await.unwrap();
        let id2 = SsTableId::from(ulid::Ulid::new());
        let handle2 = table_store
            .write_sst(&id2, &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let sr = SortedRun::new(
            0,
            [
                SsTableView::new_projected(
                    ulid::Ulid::new(),
                    handle1,
                    Some(BytesRange::from_ref("key2".."key4")),
                ),
                SsTableView::new_projected(
                    ulid::Ulid::new(),
                    handle2,
                    Some(BytesRange::from_ref("key5".."key7")),
                ),
            ],
        );

        // when: iterating the full range, then: only visible keys appear
        let mut iter = SortedRunIterator::new_borrowed_initialized(
            ..,
            &sr,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap();
        for i in [2, 3, 5, 6] {
            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            assert_eq!(kv.key.as_ref(), format!("key{i}").as_bytes());
        }
        assert!(iter.next().await.unwrap().is_none());

        // when: iterating a sub-range, then: the narrower bound applies and
        // tables disjoint with the query range are skipped entirely
        let mut iter = SortedRunIterator::new_owned_initialized(
            BytesRange::from_ref("key6"..),
            sr,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap();
        let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
        assert_eq!(kv.key.as_ref(), b"key6");
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_sr_iter_from_key() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 3,
            ..SsTableFormat::default()
        };
        let table_store = Arc::new(TableStore::new(
            object_store,
            format,
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let key_gen = OrderedBytesGenerator::new_with_byte_range(&[b'a'; 16], b'a', b'z');
        let mut test_case_key_gen = key_gen.clone();
        let val_gen = OrderedBytesGenerator::new_with_byte_range(&[0u8; 16], 0u8, 26u8);
        let mut test_case_val_gen = val_gen.clone();
        let sr = build_sr_with_ssts(table_store.clone(), 3, 10, key_gen, val_gen).await;

        for i in 0..30 {
            let mut expected_key_gen = test_case_key_gen.clone();
            let mut expected_val_gen = test_case_val_gen.clone();
            let from_key = test_case_key_gen.next();
            _ = test_case_val_gen.next();
            let mut iter = SortedRunIterator::new_borrowed_initialized(
                from_key.as_ref()..,
                &sr,
                table_store.clone(),
                SstIteratorOptions::default(),
            )
            .await
            .unwrap();
            for _ in 0..30 - i {
                assert_kv(
                    &iter.next().await.unwrap().unwrap().into(),
                    expected_key_gen.next().as_ref(),
                    expected_val_gen.next().as_ref(),
                );
            }
            assert!(iter.next().await.unwrap().is_none());
        }
    }

    #[tokio::test]
    async fn test_sr_iter_from_key_lower_than_range() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 3,
            ..SsTableFormat::default()
        };
        let table_store = Arc::new(TableStore::new(
            object_store,
            format,
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let key_gen = OrderedBytesGenerator::new_with_byte_range(&[b'a'; 16], b'a', b'z');
        let mut expected_key_gen = key_gen.clone();
        let val_gen = OrderedBytesGenerator::new_with_byte_range(&[0u8; 16], 0u8, 26u8);
        let mut expected_val_gen = val_gen.clone();
        let sr = build_sr_with_ssts(table_store.clone(), 3, 10, key_gen, val_gen).await;
        let mut iter = SortedRunIterator::new_borrowed_initialized(
            [b'a', 10].as_ref()..,
            &sr,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap();

        for _ in 0..30 {
            assert_kv(
                &iter.next().await.unwrap().unwrap().into(),
                expected_key_gen.next().as_ref(),
                expected_val_gen.next().as_ref(),
            );
        }
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_sr_iter_from_key_higher_than_range() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 3,
            ..SsTableFormat::default()
        };
        let table_store = Arc::new(TableStore::new(
            object_store,
            format,
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let key_gen = OrderedBytesGenerator::new_with_byte_range(&[b'a'; 16], b'a', b'z');
        let val_gen = OrderedBytesGenerator::new_with_byte_range(&[0u8; 16], 0u8, 26u8);
        let sr = build_sr_with_ssts(table_store.clone(), 3, 10, key_gen, val_gen).await;

        let mut iter = SortedRunIterator::new_borrowed_initialized(
            [b'z', 30].as_ref()..,
            &sr,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap();

        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_seek_through_sorted_run() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let table_store = Arc::new(TableStore::new(
            object_store,
            SsTableFormat::default(),
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));

        let mut rng = proptest_util::rng::new_test_rng(None);
        let table = sample::table(&mut rng, 400, 10);
        let max_entries_per_sst = 20u64;
        let entries_per_sst = 1..max_entries_per_sst;
        let sr =
            build_sorted_run_from_table(&table, table_store.clone(), entries_per_sst, &mut rng)
                .await;
        let mut sr_iter = SortedRunIterator::new_owned_initialized(
            ..,
            sr,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap();
        let mut table_iter = table.iter();
        loop {
            let skip = rng.random::<u64>() % (max_entries_per_sst * 2);
            let run = rng.random::<u64>() % (max_entries_per_sst * 2);

            let Some((k, _)) = table_iter.nth(skip as usize) else {
                break;
            };
            let seek_key = increment_length(k);
            sr_iter.seek(&seek_key).await.unwrap();

            for (key, value) in table_iter.by_ref().take(run as usize) {
                let kv: KeyValue = sr_iter.next().await.unwrap().unwrap().into();
                assert_eq!(*key, kv.key);
                assert_eq!(*value, kv.value);
            }
        }
    }

    fn increment_length(b: &[u8]) -> Bytes {
        let mut buf = BytesMut::from(b);
        buf.put_u8(u8::MIN);
        buf.freeze()
    }

    async fn build_sorted_run_from_table<R: SampleRange<u64> + Clone>(
        table: &BTreeMap<Bytes, Bytes>,
        table_store: Arc<TableStore>,
        entries_per_sst: R,
        rng: &mut TestRng,
    ) -> SortedRun {
        let mut ssts = Vec::new();
        let mut entries = table.iter();
        loop {
            let sst_len = rng.random_range(entries_per_sst.clone());
            let mut builder = table_store.table_builder();

            let sst_kvs: Vec<(&Bytes, &Bytes)> = entries.by_ref().take(sst_len as usize).collect();
            if sst_kvs.is_empty() {
                break;
            }

            for (key, value) in sst_kvs {
                builder.add_value(key, value, Some(0), None).await.unwrap();
            }

            let encoded = builder.build().await.unwrap();
            let id = SsTableId::from(ulid::Ulid::new());
            let handle = table_store
                .write_sst(&id, &encoded, Some(Bytes::new()))
                .await
                .unwrap();
            ssts.push(SsTableView::identity(handle));
        }

        SortedRun::new(0, ssts)
    }

    async fn build_sr_with_ssts(
        table_store: Arc<TableStore>,
        n: usize,
        keys_per_sst: usize,
        mut key_gen: OrderedBytesGenerator,
        mut val_gen: OrderedBytesGenerator,
    ) -> SortedRun {
        let mut ssts = Vec::<SsTableView>::new();
        for _ in 0..n {
            let mut writer =
                table_store.table_writer(SsTableId::from(ulid::Ulid::new()), Some(Bytes::new()));
            for _ in 0..keys_per_sst {
                let entry =
                    RowEntry::new_value(key_gen.next().as_ref(), val_gen.next().as_ref(), 0);
                writer.add(entry).await.unwrap();
            }
            let (sst, _) = writer.close().await.unwrap();
            ssts.push(SsTableView::identity(sst));
        }
        SortedRun::new(0, ssts)
    }

    mod mixed_version_tests {
        use super::*;
        use crate::sst_builder::BlockFormat;

        async fn build_sst_v1(
            table_store: &Arc<TableStore>,
            keys_and_values: &[(&[u8], &[u8])],
        ) -> SsTableHandle {
            let mut builder = table_store
                .table_builder()
                .with_block_format(BlockFormat::V1);
            for (key, value) in keys_and_values {
                builder.add_value(key, value, Some(0), None).await.unwrap();
            }
            let encoded = builder.build().await.unwrap();
            let id = SsTableId::from(ulid::Ulid::new());
            table_store
                .write_sst(&id, &encoded, Some(Bytes::new()))
                .await
                .unwrap()
        }

        async fn build_sst_v2(
            table_store: &Arc<TableStore>,
            keys_and_values: &[(&[u8], &[u8])],
        ) -> SsTableHandle {
            // V2 is now the default, so no need to explicitly set block format
            let mut builder = table_store.table_builder();
            for (key, value) in keys_and_values {
                builder.add_value(key, value, Some(0), None).await.unwrap();
            }
            let encoded = builder.build().await.unwrap();
            let id = SsTableId::from(ulid::Ulid::new());
            table_store
                .write_sst(&id, &encoded, Some(Bytes::new()))
                .await
                .unwrap()
        }

        #[tokio::test]
        async fn should_iterate_sorted_run_with_mixed_v1_and_v2_ssts() {
            // given: a sorted run with alternating v1 and v2 SSTs
            let root_path = Path::from("");
            let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
            let format = SsTableFormat {
                min_filter_keys: 10,
                ..SsTableFormat::default()
            };
            let table_store = Arc::new(TableStore::new(
                object_store,
                format,
                root_path,
                None,
                TableStoreKind::Main,
                BlockCachePolicy::default(),
            ));

            // Build a sorted run with v1, v2, v1, v2 SSTs
            let sst1_v1 = build_sst_v1(
                &table_store,
                &[(b"key01", b"value01"), (b"key02", b"value02")],
            )
            .await;
            let sst2_v2 = build_sst_v2(
                &table_store,
                &[(b"key03", b"value03"), (b"key04", b"value04")],
            )
            .await;
            let sst3_v1 = build_sst_v1(
                &table_store,
                &[(b"key05", b"value05"), (b"key06", b"value06")],
            )
            .await;
            let sst4_v2 = build_sst_v2(
                &table_store,
                &[(b"key07", b"value07"), (b"key08", b"value08")],
            )
            .await;

            let sorted_run = SortedRun::new(
                0,
                [
                    SsTableView::identity(sst1_v1),
                    SsTableView::identity(sst2_v2),
                    SsTableView::identity(sst3_v1),
                    SsTableView::identity(sst4_v2),
                ],
            );

            // when: iterating over the sorted run
            let mut iter = SortedRunIterator::new_owned_initialized(
                ..,
                sorted_run,
                table_store.clone(),
                SstIteratorOptions::default(),
            )
            .await
            .unwrap();

            // then: all keys should be returned in order across both v1 and v2 SSTs
            for i in 1..=8 {
                let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
                let expected_key = format!("key{:02}", i);
                let expected_value = format!("value{:02}", i);
                assert_eq!(kv.key.as_ref(), expected_key.as_bytes());
                assert_eq!(kv.value.as_ref(), expected_value.as_bytes());
            }

            let kv = iter.next().await.unwrap().map(KeyValue::from);
            assert!(kv.is_none());
        }

        #[tokio::test]
        async fn should_seek_through_mixed_v1_and_v2_ssts() {
            // given: a sorted run with alternating v1 and v2 SSTs
            let root_path = Path::from("");
            let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
            let format = SsTableFormat {
                min_filter_keys: 10,
                ..SsTableFormat::default()
            };
            let table_store = Arc::new(TableStore::new(
                object_store,
                format,
                root_path,
                None,
                TableStoreKind::Main,
                BlockCachePolicy::default(),
            ));

            // Build a sorted run with v1, v2, v1, v2 SSTs
            let sst1_v1 = build_sst_v1(
                &table_store,
                &[(b"key01", b"value01"), (b"key02", b"value02")],
            )
            .await;
            let sst2_v2 = build_sst_v2(
                &table_store,
                &[(b"key03", b"value03"), (b"key04", b"value04")],
            )
            .await;
            let sst3_v1 = build_sst_v1(
                &table_store,
                &[(b"key05", b"value05"), (b"key06", b"value06")],
            )
            .await;
            let sst4_v2 = build_sst_v2(
                &table_store,
                &[(b"key07", b"value07"), (b"key08", b"value08")],
            )
            .await;

            let sorted_run = SortedRun::new(
                0,
                [
                    SsTableView::identity(sst1_v1),
                    SsTableView::identity(sst2_v2),
                    SsTableView::identity(sst3_v1),
                    SsTableView::identity(sst4_v2),
                ],
            );

            let mut iter = SortedRunIterator::new_owned_initialized(
                ..,
                sorted_run,
                table_store.clone(),
                SstIteratorOptions::default(),
            )
            .await
            .unwrap();

            // when: seeking to key05 (which is in a v1 SST after a v2 SST)
            iter.seek(b"key05").await.unwrap();

            // then: we should get key05 and subsequent keys
            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            assert_eq!(kv.key.as_ref(), b"key05");
            assert_eq!(kv.value.as_ref(), b"value05");

            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            assert_eq!(kv.key.as_ref(), b"key06");
            assert_eq!(kv.value.as_ref(), b"value06");

            // Seek again to a v2 SST
            iter.seek(b"key07").await.unwrap();

            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            assert_eq!(kv.key.as_ref(), b"key07");
            assert_eq!(kv.value.as_ref(), b"value07");
        }
    }
}
