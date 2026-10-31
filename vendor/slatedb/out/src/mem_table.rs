use std::cell::Cell;
use std::ops::{Bound, RangeBounds};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use crossbeam_skiplist::map::Range;
use crossbeam_skiplist::SkipMap;
use ouroboros::self_referencing;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering::SeqCst;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;

use crate::db_common::extract_segment_prefix;
use crate::error::SlateDBError;
use crate::iter::{IterationOrder, RowEntryIterator};
use crate::prefix_extractor::PrefixExtractor;
use crate::reader::ReadTrace;
use crate::seq_tracker::{SequenceTracker, TrackedSeq};
use crate::types::RowEntry;
use crate::utils::{WatchableOnceCell, WatchableOnceCellReader};

/// Memtable may contains multiple versions of a single user key, with a monotonically increasing sequence number.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct SequencedKey {
    pub(crate) user_key: Bytes,
    pub(crate) seq: u64,
}

impl SequencedKey {
    pub(crate) fn new(user_key: Bytes, seq: u64) -> Self {
        Self { user_key, seq }
    }
}

impl Ord for SequencedKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.user_key
            .cmp(&other.user_key)
            .then(self.seq.cmp(&other.seq).reverse())
    }
}

impl PartialOrd for SequencedKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KVTableInternalKeyRange {
    start_bound: Bound<SequencedKey>,
    end_bound: Bound<SequencedKey>,
}

impl RangeBounds<SequencedKey> for KVTableInternalKeyRange {
    fn start_bound(&self) -> Bound<&SequencedKey> {
        self.start_bound.as_ref()
    }

    fn end_bound(&self) -> Bound<&SequencedKey> {
        self.end_bound.as_ref()
    }
}

/// Convert a user key range to a memtable internal key range. The internal key range should contain all the sequence
/// numbers for the given user key in the range. This is used for iterating over the memtable in [`KVTable::range`].
///
/// Please note that the sequence number is ordered in reverse, given a user key range (`key001`..=`key001`), the first
/// sequence number in this range is u64::MAX, and the last sequence number is 0. The output range should be
/// `(key001, u64::MAX) ..= (key001, 0)`.
impl<T: RangeBounds<Bytes>> From<T> for KVTableInternalKeyRange {
    fn from(range: T) -> Self {
        let start_bound = match range.start_bound() {
            Bound::Included(key) => Bound::Included(SequencedKey::new(key.clone(), u64::MAX)),
            Bound::Excluded(key) => Bound::Excluded(SequencedKey::new(key.clone(), 0)),
            Bound::Unbounded => Bound::Unbounded,
        };
        let end_bound = match range.end_bound() {
            Bound::Included(key) => Bound::Included(SequencedKey::new(key.clone(), 0)),
            Bound::Excluded(key) => Bound::Excluded(SequencedKey::new(key.clone(), u64::MAX)),
            Bound::Unbounded => Bound::Unbounded,
        };
        Self {
            start_bound,
            end_bound,
        }
    }
}

pub(crate) struct KVTable {
    map: Arc<SkipMap<SequencedKey, RowEntry>>,
    durable: WatchableOnceCell<Result<(), SlateDBError>>,
    entries_size_in_bytes: AtomicUsize,
    /// this corresponds to the timestamp of the most recent
    /// modifying operation on this KVTable (insertion or deletion)
    last_tick: AtomicI64,
    /// the sequence number of the most recent operation on this KVTable
    last_seq: AtomicU64,
    /// the sequence number of the oldest entry in this KVTable
    first_seq: AtomicU64,
    /// A sequence tracker that correlates sequence numbers with system clock ticks.
    /// The tracker is limited to 8192 entries and downsamples data when it gets full.
    sequence_tracker: Mutex<SequenceTracker>,
    /// RFC-0024: distinct segment prefixes touched by writes that have
    /// landed in this table. Populated by the write and WAL-replay
    /// paths after the antichain check. Empty when no extractor is
    /// configured.
    touched_segments: Mutex<std::collections::BTreeSet<Bytes>>,
}

pub(crate) struct KVTableMetadata {
    pub(crate) entry_num: usize,
    pub(crate) entries_size_in_bytes: usize,
    /// this corresponds to the timestamp of the most recent
    /// modifying operation on this KVTable (insertion or deletion)
    #[allow(dead_code)]
    pub(crate) last_tick: i64,
    /// the sequence number of the most recent operation on this KVTable
    pub(crate) last_seq: u64,
    /// the sequence number of the oldest entry in this KVTable
    pub(crate) first_seq: u64,
}

pub(crate) struct WritableKVTable {
    table: Arc<KVTable>,
}

impl WritableKVTable {
    pub(crate) fn new() -> Self {
        Self {
            table: Arc::new(KVTable::new()),
        }
    }

    pub(crate) fn table(&self) -> &Arc<KVTable> {
        &self.table
    }

    pub(crate) fn put(&self, row: RowEntry) {
        self.table.put(row);
    }

    pub(crate) fn metadata(&self) -> KVTableMetadata {
        self.table.metadata()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    pub(crate) fn record_sequence(&self, seq: u64, ts: DateTime<Utc>) {
        self.table.record_sequence(seq, ts);
    }

    /// Delegate to [`KVTable::record_touched_segments`].
    pub(crate) fn record_touched_segments(&self, prefixes: std::collections::BTreeSet<Bytes>) {
        self.table.record_touched_segments(prefixes);
    }
}

pub(crate) struct ImmutableMemtable {
    /// A WAL ID for which it is guaranteed that all writes in the contained WAL object are present
    /// in this memtable. This is used to determine a safe replay point when this IMM is flushed to
    /// a new L0. On a restart, only WALs after this id may contain data that is not present in the
    /// tree.
    ///
    /// Please note that this WAL ID might not exactly match the last WAL ID that produced this
    /// IMM, so we still need to take the last l0's `last_seq` to filter out the entries that are
    /// already contained in the last L0 SST.
    replay_after_wal_id: u64,
    table: Arc<KVTable>,
    /// Notified when the memtable's SST has been uploaded to object storage.
    /// Used to release backpressure on writers when unflushed bytes are too high.
    uploaded: WatchableOnceCell<Result<(), SlateDBError>>,
    /// A snapshot of the sequence tracker taken when this immutable memtable was created.
    /// This avoids needing to access the sequence tracker through a mutex on the underlying table.
    sequence_tracker: SequenceTracker,
}

#[self_referencing]
pub(crate) struct MemTableIteratorInner<T: RangeBounds<SequencedKey>> {
    map: Arc<SkipMap<SequencedKey, RowEntry>>,
    /// `inner` is the Iterator impl of SkipMap, which is the underlying data structure of MemTable.
    #[borrows(map)]
    #[not_covariant]
    inner: Range<'this, SequencedKey, T, SequencedKey, RowEntry>,
    ordering: IterationOrder,
    item: Option<RowEntry>,
    /// Stack used in descending mode to re-order entries within a key group.
    /// When iterating descending, `next_back()` yields entries for the same key
    /// in seq-ascending order. Pushing them onto this stack and popping gives
    /// seq-descending order, which is what the merge iterator needs for dedup.
    descending_stack: Vec<RowEntry>,
    read_trace: ReadTrace,
}
pub(crate) type MemTableIterator = MemTableIteratorInner<KVTableInternalKeyRange>;

#[async_trait]
impl RowEntryIterator for MemTableIterator {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        Ok(())
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        Ok(self.next_sync())
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        loop {
            let front = self.borrow_item().clone();
            if front.is_some_and(|record| record.key < next_key) {
                self.next_sync();
                // Keep in-memory seeking cooperative.
                tokio::task::coop::consume_budget().await;
            } else {
                return Ok(());
            }
        }
    }
}

impl MemTableIterator {
    pub(crate) fn next_sync(&mut self) -> Option<RowEntry> {
        let span = self.borrow_read_trace().new_memtable_span();
        let _guard = span.enter();
        match self.borrow_ordering() {
            IterationOrder::Ascending => self.next_ascending(),
            IterationOrder::Descending => self.next_descending(),
        }
    }

    fn next_ascending(&mut self) -> Option<RowEntry> {
        let ans = self.borrow_item().clone();
        let next_entry = self.with_inner_mut(|inner| inner.next());
        let cloned_next = next_entry.map(|entry| entry.value().clone());
        self.with_item_mut(|item| *item = cloned_next);
        ans
    }

    fn next_descending(&mut self) -> Option<RowEntry> {
        // If the stack has entries, pop from it (LIFO gives seq-descending).
        let top = self.with_descending_stack_mut(|stack| stack.pop());
        if top.is_some() {
            return top;
        }

        // Stack is empty. The current `item` (if any) is the start of a new
        // key group. Collect all entries for this key from the SkipMap.
        let first = self.borrow_item().clone();
        match first {
            Some(first) => {
                let current_key = first.key.clone();

                // Collect remaining entries with the same key. next_back()
                // yields them in seq-ascending order. Pushing onto the stack
                // and popping gives seq-descending order.
                self.with_descending_stack_mut(|stack| stack.push(first));
                self.fill_descending_stack(&current_key);

                self.with_descending_stack_mut(|stack| stack.pop())
            }
            None => {
                // item is None — either iterator is exhausted, or this is the
                // priming call. Advance inner to populate item for the next call.
                let next = self
                    .with_inner_mut(|inner| inner.next_back().map(|entry| entry.value().clone()));
                self.with_item_mut(|item| *item = next);
                None
            }
        }
    }

    fn fill_descending_stack(&mut self, current_key: &Bytes) {
        loop {
            let next = self.with_inner_mut(|inner| {
                inner
                    .next_back()
                    .map(|entry| (entry.key().user_key.clone(), entry.value().clone()))
            });
            match next {
                Some((key, row)) if key == *current_key => {
                    self.with_descending_stack_mut(|stack| stack.push(row));
                }
                other => {
                    self.with_item_mut(|item| *item = other.map(|(_, row)| row));
                    break;
                }
            }
        }
    }
}

impl ImmutableMemtable {
    pub(crate) fn new(table: WritableKVTable, replay_after_wal_id: u64) -> Self {
        let sequence_tracker = table.table.sequence_tracker_snapshot();
        Self {
            table: table.table,
            replay_after_wal_id,
            uploaded: WatchableOnceCell::new(),
            sequence_tracker,
        }
    }

    /// Segment prefixes touched by writes in this imm.
    /// Delegate to [`KVTable::touched_segments`].
    pub(crate) fn touched_segments(&self) -> std::collections::BTreeSet<Bytes> {
        self.table.touched_segments()
    }

    pub(crate) fn table(&self) -> Arc<KVTable> {
        self.table.clone()
    }

    pub(crate) fn recent_flushed_wal_id(&self) -> u64 {
        self.replay_after_wal_id
    }

    pub(crate) async fn await_uploaded(&self) -> Result<(), SlateDBError> {
        self.uploaded.reader().await_value().await
    }

    pub(crate) fn notify_uploaded(&self, result: Result<(), SlateDBError>) {
        self.uploaded.write(result);
    }

    pub(crate) fn sequence_tracker(&self) -> &SequenceTracker {
        &self.sequence_tracker
    }

    /// Returns a new [`ImmutableMemtable`] that only contains entries with sequence
    /// number greater than the given `seq`. [`ImmutableMemtable::recent_flushed_wal_id`]
    /// remains the same.
    ///
    /// When `segment_extractor` is `Some`, the surviving entries are run
    /// through it to rebuild the touched-segment set (RFC-0024) from
    /// scratch — the same derivation the writer applies in
    /// [`WriteBatch::extract_entries`], via the shared
    /// [`extract_segment_prefix`]. Prefixes whose every row was filtered
    /// out are therefore dropped, and no row's prefix can survive without
    /// its row. An empty/absent prefix is an `EmptySegmentPrefix` error,
    /// matching the write and replay paths. When `segment_extractor` is
    /// `None`, the new table records no segments.
    pub(crate) fn filter_after_seq(
        &self,
        seq: u64,
        segment_extractor: Option<&dyn PrefixExtractor>,
    ) -> Result<Self, SlateDBError> {
        let new_table = WritableKVTable::new();
        let mut surviving_segments = std::collections::BTreeSet::new();
        let mut table_iter = self.table.iter();
        while let Some(entry) = table_iter.next_sync() {
            if entry.seq > seq {
                if let Some(extractor) = segment_extractor {
                    surviving_segments.insert(extract_segment_prefix(extractor, &entry.key)?);
                }
                new_table.put(entry);
            }
        }
        new_table.record_touched_segments(surviving_segments);
        Ok(Self::new(new_table, self.replay_after_wal_id))
    }
}

impl KVTable {
    pub(crate) fn new() -> Self {
        Self {
            map: Arc::new(SkipMap::new()),
            entries_size_in_bytes: AtomicUsize::new(0),
            durable: WatchableOnceCell::new(),
            last_tick: AtomicI64::new(i64::MIN),
            last_seq: AtomicU64::new(0),
            first_seq: AtomicU64::new(u64::MAX),
            sequence_tracker: Mutex::new(SequenceTracker::new()),
            touched_segments: Mutex::new(std::collections::BTreeSet::new()),
        }
    }

    /// Merge a batch's touched-segment prefixes into this table's
    /// running set. The caller's set is moved in, so the first batch
    /// against an empty table is a direct adoption rather than an
    /// element-wise extend.
    pub(crate) fn record_touched_segments(&self, prefixes: std::collections::BTreeSet<Bytes>) {
        if prefixes.is_empty() {
            return;
        }
        let mut guard = self.touched_segments.lock();
        if guard.is_empty() {
            *guard = prefixes;
        } else {
            guard.extend(prefixes);
        }
    }

    /// Snapshot of the segment prefixes touched by writes in this
    /// table. Cheap clone (cardinality is bounded by the number of
    /// distinct active segments — typically one).
    pub(crate) fn touched_segments(&self) -> std::collections::BTreeSet<Bytes> {
        self.touched_segments.lock().clone()
    }

    /// True iff this table's touched-segment set is empty *or* contains
    /// `prefix`. Used by per-prefix reservation accounting on the
    /// dispatch tracker, where an empty set is treated conservatively
    /// as "may target any prefix" (because we can't rule it out).
    /// Avoids cloning the full set when a single membership check
    /// suffices.
    pub(crate) fn touched_segments_empty_or_contains(&self, prefix: &Bytes) -> bool {
        let touched = self.touched_segments.lock();
        touched.is_empty() || touched.contains(prefix)
    }

    /// Validate `prefix` against this table's touched-segment set
    /// (RFC-0024 antichain). Returns `Ok(true)` if `prefix` is already
    /// an exact member — it was validated when first inserted, so the
    /// caller can skip checking it against older sources. Returns
    /// `Ok(false)` if `prefix` is disjoint from every recorded
    /// prefix; the caller must continue validating against
    /// downstream sources.
    ///
    /// Only the immediate sort-order neighbors in `touched_segments`
    /// can nest with `prefix`: the predecessor (largest `<= prefix`)
    /// is the only candidate ancestor, and the successor (smallest
    /// `> prefix`) is the only candidate descendant. Every other
    /// existing prefix would itself nest with one of those neighbors,
    /// which is impossible because `touched_segments` is already an
    /// antichain by induction on prior writes.
    pub(crate) fn ensure_valid_segment(&self, prefix: &Bytes) -> Result<bool, SlateDBError> {
        let touched = self.touched_segments.lock();
        if touched.is_empty() {
            return Ok(false);
        }
        if let Some(pred) = touched.range::<Bytes, _>(..=prefix).next_back() {
            if pred == prefix {
                return Ok(true);
            }
            if prefix.starts_with(pred.as_ref()) {
                return Err(SlateDBError::InvalidSegmentPrefix {
                    prefix: prefix.clone(),
                    conflict: pred.clone(),
                });
            }
        }
        if let Some(succ) = touched
            .range::<Bytes, _>((Bound::Excluded(prefix), Bound::Unbounded))
            .next()
        {
            if succ.starts_with(prefix.as_ref()) {
                return Err(SlateDBError::InvalidSegmentPrefix {
                    prefix: prefix.clone(),
                    conflict: succ.clone(),
                });
            }
        }
        Ok(false)
    }

    pub(crate) fn metadata(&self) -> KVTableMetadata {
        let entry_num = self.map.len();
        let entries_size_in_bytes = self.entries_size_in_bytes.load(Ordering::Relaxed);
        let last_tick = self.last_tick.load(SeqCst);
        let last_seq = self.last_seq().unwrap_or(0);
        let first_seq = self.first_seq().unwrap_or(0);
        KVTableMetadata {
            entry_num,
            entries_size_in_bytes,
            last_tick,
            last_seq,
            first_seq,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub(crate) fn last_tick(&self) -> i64 {
        self.last_tick.load(SeqCst)
    }

    pub(crate) fn last_seq(&self) -> Option<u64> {
        if self.is_empty() {
            None
        } else {
            let last_seq = self.last_seq.load(SeqCst);
            Some(last_seq)
        }
    }

    pub(crate) fn first_seq(&self) -> Option<u64> {
        if self.is_empty() {
            None
        } else {
            let first_seq = self.first_seq.load(SeqCst);
            Some(first_seq)
        }
    }

    pub(crate) fn iter(&self) -> MemTableIterator {
        self.range_ascending(..)
    }

    pub(crate) fn range_ascending<T: RangeBounds<Bytes>>(&self, range: T) -> MemTableIterator {
        self.range(range, IterationOrder::Ascending, ReadTrace::new(None))
    }

    pub(crate) fn range<T: RangeBounds<Bytes>>(
        &self,
        range: T,
        ordering: IterationOrder,
        read_trace: ReadTrace,
    ) -> MemTableIterator {
        let internal_range = KVTableInternalKeyRange::from(range);
        let mut iterator = MemTableIteratorInnerBuilder {
            map: self.map.clone(),
            inner_builder: |map| map.range(internal_range),
            ordering,
            item: None,
            descending_stack: Vec::new(),
            read_trace,
        }
        .build();
        iterator.next_sync();
        iterator
    }

    pub(crate) fn put(&self, row: RowEntry) {
        let internal_key = SequencedKey::new(row.key.clone(), row.seq);
        let previous_size = Cell::new(None);

        // it is safe to use fetch_max here to update the last tick
        // because the monotonicity is enforced when generating the clock tick
        // (see [crate::utils::MonotonicClock::now])
        if let Some(create_ts) = row.create_ts {
            self.last_tick.fetch_max(create_ts, SeqCst);
        }
        // update the last seq number if it is greater than the current last seq
        self.last_seq.fetch_max(row.seq, SeqCst);
        // update the first seq number if it is smaller than the current first seq
        self.first_seq.fetch_min(row.seq, SeqCst);

        let row_size = row.estimated_size();
        self.map.compare_insert(internal_key, row, |previous_row| {
            // Optimistically calculate the size of the previous value.
            // `compare_fn` might be called multiple times in case of concurrent
            // writes to the same key, so we use `Cell` to avoid subtracting
            // the size multiple times. The last call will set the correct size.
            previous_size.set(Some(previous_row.estimated_size()));
            true
        });
        if let Some(size) = previous_size.take() {
            self.entries_size_in_bytes
                .fetch_sub(size, Ordering::Relaxed);
            self.entries_size_in_bytes
                .fetch_add(row_size, Ordering::Relaxed);
        } else {
            self.entries_size_in_bytes
                .fetch_add(row_size, Ordering::Relaxed);
        }
    }

    pub(crate) fn durable_watcher(&self) -> WatchableOnceCellReader<Result<(), SlateDBError>> {
        self.durable.reader()
    }

    pub(crate) fn notify_durable(&self, result: Result<(), SlateDBError>) {
        self.durable.write(result);
    }

    pub(crate) fn record_sequence(&self, seq: u64, ts: DateTime<Utc>) {
        let mut tracker = self.sequence_tracker.lock();
        tracker.insert(TrackedSeq { seq, ts });
    }

    pub(crate) fn sequence_tracker_snapshot(&self) -> SequenceTracker {
        self.sequence_tracker.lock().clone()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::bytes_range::BytesRange;
    use crate::merge_iterator::MergeIterator;
    use crate::prefix_extractor::PrefixTarget;
    use crate::proptest_util::{arbitrary, sample};
    use crate::test_utils::assert_iterator;
    use crate::{proptest_util, test_utils};
    use rstest::rstest;
    use tokio::runtime::Runtime;

    #[tokio::test]
    async fn test_memtable_iter() {
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(b"abc333", b"value3", 1));
        table.put(RowEntry::new_value(b"abc111", b"value1", 2));
        table.put(RowEntry::new_value(b"abc555", b"value5", 3));
        table.put(RowEntry::new_value(b"abc444", b"value4", 4));
        table.put(RowEntry::new_value(b"abc222", b"value2", 5));
        assert_eq!(table.table().last_seq(), Some(5));
        assert_eq!(table.table().first_seq(), Some(1));

        let mut iter = table.table().iter();
        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_value(b"abc111", b"value1", 2),
                RowEntry::new_value(b"abc222", b"value2", 5),
                RowEntry::new_value(b"abc333", b"value3", 1),
                RowEntry::new_value(b"abc444", b"value4", 4),
                RowEntry::new_value(b"abc555", b"value5", 3),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_memtable_iter_entry_attrs() {
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(b"abc333", b"value3", 1));
        table.put(RowEntry::new_value(b"abc111", b"value1", 2));

        let mut iter = table.table().iter();
        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_value(b"abc111", b"value1", 2),
                RowEntry::new_value(b"abc333", b"value3", 1),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_memtable_range_from_existing_key() {
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(b"abc333", b"value3", 1));
        table.put(RowEntry::new_value(b"abc111", b"value1", 2));
        table.put(RowEntry::new_value(b"abc555", b"value5", 3));
        table.put(RowEntry::new_value(b"abc444", b"value4", 4));
        table.put(RowEntry::new_value(b"abc222", b"value2", 5));

        let mut iter = table
            .table()
            .range_ascending(BytesRange::from(Bytes::from_static(b"abc333")..));
        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_value(b"abc333", b"value3", 1),
                RowEntry::new_value(b"abc444", b"value4", 4),
                RowEntry::new_value(b"abc555", b"value5", 3),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_memtable_range_from_nonexisting_key() {
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(b"abc333", b"value3", 1));
        table.put(RowEntry::new_value(b"abc111", b"value1", 2));
        table.put(RowEntry::new_value(b"abc555", b"value5", 3));
        table.put(RowEntry::new_value(b"abc444", b"value4", 4));
        table.put(RowEntry::new_value(b"abc222", b"value2", 5));

        let mut iter = table
            .table()
            .range_ascending(BytesRange::from(Bytes::from_static(b"abc334")..));
        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_value(b"abc444", b"value4", 4),
                RowEntry::new_value(b"abc555", b"value5", 3),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_memtable_iter_delete() {
        let table = WritableKVTable::new();
        table.put(RowEntry::new_tombstone(b"abc333", 2));
        table.put(RowEntry::new_value(b"abc333", b"value3", 1));
        table.put(RowEntry::new_value(b"abc444", b"value4", 4));

        // in merge iterator, it should only return one entry
        let iter = table.table().iter();
        let mut merge_iter = MergeIterator::new(VecDeque::from(vec![iter])).unwrap();
        assert_iterator(
            &mut merge_iter,
            vec![
                RowEntry::new_tombstone(b"abc333", 2),
                RowEntry::new_value(b"abc444", b"value4", 4),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_memtable_track_sz_and_num() {
        let table = WritableKVTable::new();
        let mut metadata = table.table().metadata();

        assert_eq!(metadata.entry_num, 0);
        assert_eq!(metadata.entries_size_in_bytes, 0);
        table.put(RowEntry::new_value(b"first", b"foo", 1));
        metadata = table.table().metadata();
        assert_eq!(metadata.entry_num, 1);
        assert_eq!(metadata.entries_size_in_bytes, 16);

        table.put(RowEntry::new_tombstone(b"first", 2));
        metadata = table.table().metadata();
        assert_eq!(metadata.entry_num, 2);
        assert_eq!(metadata.entries_size_in_bytes, 29);

        table.put(RowEntry::new_value(b"abc333", b"val1", 1));
        metadata = table.table().metadata();
        assert_eq!(metadata.entry_num, 3);
        assert_eq!(metadata.entries_size_in_bytes, 47);

        table.put(RowEntry::new_value(b"def456", b"blablabla", 2));
        metadata = table.table().metadata();
        assert_eq!(metadata.entry_num, 4);
        assert_eq!(metadata.entries_size_in_bytes, 70);

        table.put(RowEntry::new_value(b"def456", b"blabla", 3));
        metadata = table.table().metadata();
        assert_eq!(metadata.entry_num, 5);
        assert_eq!(metadata.entries_size_in_bytes, 90);

        table.put(RowEntry::new_tombstone(b"abc333", 4));
        metadata = table.table().metadata();
        assert_eq!(metadata.entry_num, 6);
        assert_eq!(metadata.entries_size_in_bytes, 104);
    }

    #[tokio::test]
    async fn test_memtable_track_seqs() {
        let table = WritableKVTable::new();
        let mut metadata = table.table().metadata();

        assert_eq!(metadata.last_seq, 0);
        assert_eq!(metadata.first_seq, 0);
        table.put(RowEntry::new_value(b"first", b"foo", 1));
        metadata = table.table().metadata();
        assert_eq!(metadata.last_seq, 1);
        assert_eq!(metadata.first_seq, 1);

        table.put(RowEntry::new_tombstone(b"first", 2));
        metadata = table.table().metadata();
        assert_eq!(metadata.last_seq, 2);
        assert_eq!(metadata.first_seq, 1);

        table.put(RowEntry::new_value(b"abc333", b"val1", 1));
        metadata = table.table().metadata();
        assert_eq!(metadata.last_seq, 2);
        assert_eq!(metadata.first_seq, 1);

        table.put(RowEntry::new_value(b"def456", b"blablabla", 2));
        metadata = table.table().metadata();
        assert_eq!(metadata.last_seq, 2);
        assert_eq!(metadata.first_seq, 1);

        table.put(RowEntry::new_value(b"def456", b"blabla", 3));
        metadata = table.table().metadata();
        assert_eq!(metadata.last_seq, 3);
        assert_eq!(metadata.first_seq, 1);

        table.put(RowEntry::new_tombstone(b"abc333", 4));
        metadata = table.table().metadata();
        assert_eq!(metadata.last_seq, 4);
        assert_eq!(metadata.first_seq, 1);
    }

    #[rstest]
    #[case(
        BytesRange::from(..),
        KVTableInternalKeyRange {
            start_bound: Bound::Unbounded,
            end_bound: Bound::Unbounded,
        },
        vec![SequencedKey::new(Bytes::from_static(b"abc111"), 1)],
        vec![]
    )]
    #[case(
        BytesRange::from(Bytes::from_static(b"abc111")..=Bytes::from_static(b"abc333")),
        KVTableInternalKeyRange {
            start_bound: Bound::Included(SequencedKey::new(Bytes::from_static(b"abc111"), u64::MAX)),
            end_bound: Bound::Included(SequencedKey::new(Bytes::from_static(b"abc333"), 0)),
        },
        vec![
            SequencedKey::new(Bytes::from_static(b"abc111"), 1),
            SequencedKey::new(Bytes::from_static(b"abc222"), 2),
            SequencedKey::new(Bytes::from_static(b"abc333"), 3),
            SequencedKey::new(Bytes::from_static(b"abc333"), 0),
            SequencedKey::new(Bytes::from_static(b"abc333"), u64::MAX),
        ],
        vec![SequencedKey::new(Bytes::from_static(b"abc444"), 4)]
    )]
    #[case(
        BytesRange::from(Bytes::from_static(b"abc222")..Bytes::from_static(b"abc444")),
        KVTableInternalKeyRange {
            start_bound: Bound::Included(SequencedKey::new(Bytes::from_static(b"abc222"), u64::MAX)),
            end_bound: Bound::Excluded(SequencedKey::new(Bytes::from_static(b"abc444"), u64::MAX)),
        },
        vec![
            SequencedKey::new(Bytes::from_static(b"abc222"), 1),
            SequencedKey::new(Bytes::from_static(b"abc333"), 2),
        ],
        vec![
            SequencedKey::new(Bytes::from_static(b"abc444"), 0),
            SequencedKey::new(Bytes::from_static(b"abc444"), u64::MAX),
            SequencedKey::new(Bytes::from_static(b"abc555"), u64::MAX),
        ]
    )]
    #[case(
        BytesRange::from(..=Bytes::from_static(b"abc333")),
        KVTableInternalKeyRange {
            start_bound: Bound::Unbounded,
            end_bound: Bound::Included(SequencedKey::new(Bytes::from_static(b"abc333"), 0)),
        },
        vec![
            SequencedKey::new(Bytes::from_static(b"abc111"), 1),
            SequencedKey::new(Bytes::from_static(b"abc222"), 2),
            SequencedKey::new(Bytes::from_static(b"abc333"), 3),
            SequencedKey::new(Bytes::from_static(b"abc333"), u64::MAX),
        ],
        vec![SequencedKey::new(Bytes::from_static(b"abc444"), 4)]
    )]
    fn test_from_internal_key_range(
        #[case] range: BytesRange,
        #[case] expected: KVTableInternalKeyRange,
        #[case] should_contains: Vec<SequencedKey>,
        #[case] should_not_contains: Vec<SequencedKey>,
    ) {
        let range = KVTableInternalKeyRange::from(range);
        assert_eq!(range, expected);
        for key in should_contains {
            assert!(range.contains(&key));
        }
        for key in should_not_contains {
            assert!(!range.contains(&key));
        }
    }

    #[test]
    fn should_iterate_arbitrary_range() {
        let mut runner = proptest_util::runner::new(file!(), None);
        let runtime = Runtime::new().unwrap();
        let sample_table = sample::table(runner.rng(), 500, 10);

        let kv_table = WritableKVTable::new();
        let mut seq = 1;
        for (key, value) in &sample_table {
            let row_entry = RowEntry::new_value(key, value, seq);
            kv_table.put(row_entry);
            seq += 1;
        }

        runner
            .run(
                &(arbitrary::nonempty_range(10), arbitrary::iteration_order()),
                |(range, ordering)| {
                    let mut kv_iter =
                        kv_table
                            .table
                            .range(range.clone(), ordering, ReadTrace::new(None));

                    runtime.block_on(test_utils::assert_ranged_kv_scan(
                        &sample_table,
                        &range,
                        ordering,
                        &mut kv_iter,
                    ));
                    Ok(())
                },
            )
            .unwrap();
    }

    fn touched(prefixes: &[&[u8]]) -> std::collections::BTreeSet<Bytes> {
        prefixes.iter().map(|p| Bytes::copy_from_slice(p)).collect()
    }

    /// Extracts a single-byte segment prefix, matching the 1-byte
    /// prefixes used by the `filter_after_seq` tests.
    #[derive(Debug)]
    struct SingleByteExtractor;

    impl PrefixExtractor for SingleByteExtractor {
        fn name(&self) -> &str {
            "single-byte-test-only"
        }
        fn prefix_len(&self, target: &PrefixTarget) -> Option<usize> {
            let len = match target {
                PrefixTarget::Point(b) | PrefixTarget::Prefix(b) => b.len(),
            };
            (len >= 1).then_some(1)
        }
    }

    fn assert_invalid_segment_prefix(err: SlateDBError, prefix: &[u8], conflict: &[u8]) {
        match err {
            SlateDBError::InvalidSegmentPrefix {
                prefix: p,
                conflict: c,
            } => {
                assert_eq!(p.as_ref(), prefix, "unexpected prefix in error");
                assert_eq!(c.as_ref(), conflict, "unexpected conflict in error");
            }
            other => panic!("expected InvalidSegmentPrefix, got {other:?}"),
        }
    }

    #[test]
    fn ensure_valid_segment_returns_false_when_table_has_no_touched_segments() {
        let table = KVTable::new();
        assert!(!table
            .ensure_valid_segment(&Bytes::from_static(b"abc"))
            .unwrap());
    }

    #[test]
    fn ensure_valid_segment_returns_true_on_exact_match() {
        let table = KVTable::new();
        table.record_touched_segments(touched(&[b"abc"]));
        assert!(table
            .ensure_valid_segment(&Bytes::from_static(b"abc"))
            .unwrap());
    }

    #[test]
    fn ensure_valid_segment_returns_false_when_disjoint() {
        let table = KVTable::new();
        table.record_touched_segments(touched(&[b"abc", b"xyz"]));
        // "def" sorts between but does not nest with either neighbor.
        assert!(!table
            .ensure_valid_segment(&Bytes::from_static(b"def"))
            .unwrap());
    }

    #[test]
    fn ensure_valid_segment_rejects_when_predecessor_is_proper_prefix() {
        let table = KVTable::new();
        table.record_touched_segments(touched(&[b"abc"]));
        let err = table
            .ensure_valid_segment(&Bytes::from_static(b"abcd"))
            .unwrap_err();
        assert_invalid_segment_prefix(err, b"abcd", b"abc");
    }

    #[test]
    fn ensure_valid_segment_rejects_when_candidate_is_proper_prefix_of_successor() {
        let table = KVTable::new();
        table.record_touched_segments(touched(&[b"abc"]));
        let err = table
            .ensure_valid_segment(&Bytes::from_static(b"ab"))
            .unwrap_err();
        assert_invalid_segment_prefix(err, b"ab", b"abc");
    }

    #[test]
    fn ensure_valid_segment_only_inspects_immediate_neighbors() {
        // Predecessor "aa" doesn't nest with "ac"; successor "az" doesn't
        // either. The far-away "x" must not be consulted.
        let table = KVTable::new();
        table.record_touched_segments(touched(&[b"aa", b"az", b"x"]));
        assert!(!table
            .ensure_valid_segment(&Bytes::from_static(b"ac"))
            .unwrap());
    }

    #[test]
    fn ensure_valid_segment_handles_candidate_smaller_than_all() {
        let table = KVTable::new();
        table.record_touched_segments(touched(&[b"mmm", b"ppp"]));
        // "aaa" has no predecessor; "mmm" doesn't start with "aaa".
        assert!(!table
            .ensure_valid_segment(&Bytes::from_static(b"aaa"))
            .unwrap());
    }

    #[test]
    fn ensure_valid_segment_handles_candidate_larger_than_all() {
        let table = KVTable::new();
        table.record_touched_segments(touched(&[b"aaa", b"bbb"]));
        // "zzz" has no successor; predecessor "bbb" is not a prefix of "zzz".
        assert!(!table
            .ensure_valid_segment(&Bytes::from_static(b"zzz"))
            .unwrap());
    }

    fn surviving_keys(imm: &ImmutableMemtable) -> Vec<Bytes> {
        let mut iter = imm.table().iter();
        let mut keys = Vec::new();
        while let Some(entry) = iter.next_sync() {
            keys.push(entry.key);
        }
        keys
    }

    #[test]
    fn should_drop_prefix_with_no_surviving_rows() {
        // given
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(b"a1", b"v", 1));
        table.put(RowEntry::new_value(b"a2", b"v", 2));
        table.put(RowEntry::new_value(b"b1", b"v", 3));
        table.put(RowEntry::new_value(b"b2", b"v", 4));
        table.record_touched_segments(touched(&[b"a", b"b"]));
        let imm = ImmutableMemtable::new(table, 0);

        // when
        let filtered = imm.filter_after_seq(2, Some(&SingleByteExtractor)).unwrap();

        // then
        assert_eq!(filtered.table().touched_segments(), touched(&[b"b"]));
        assert_eq!(
            surviving_keys(&filtered),
            vec![Bytes::from_static(b"b1"), Bytes::from_static(b"b2")]
        );
    }

    #[test]
    fn should_keep_prefixes_with_surviving_rows() {
        // given
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(b"a1", b"v", 1));
        table.put(RowEntry::new_value(b"b1", b"v", 2));
        table.record_touched_segments(touched(&[b"a", b"b"]));
        let imm = ImmutableMemtable::new(table, 0);

        // when
        let filtered = imm.filter_after_seq(0, Some(&SingleByteExtractor)).unwrap();

        // then
        assert_eq!(filtered.table().touched_segments(), touched(&[b"a", b"b"]));
    }

    #[test]
    fn should_empty_touched_segments_when_no_rows_survive() {
        // given
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(b"a1", b"v", 1));
        table.put(RowEntry::new_value(b"b1", b"v", 2));
        table.record_touched_segments(touched(&[b"a", b"b"]));
        let imm = ImmutableMemtable::new(table, 0);

        // when
        let filtered = imm
            .filter_after_seq(10, Some(&SingleByteExtractor))
            .unwrap();

        // then
        assert!(filtered.table().touched_segments().is_empty());
        assert!(surviving_keys(&filtered).is_empty());
    }

    #[test]
    fn should_leave_touched_segments_empty_for_non_segmented() {
        // given
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(b"a1", b"v", 1));
        table.put(RowEntry::new_value(b"b1", b"v", 2));
        let imm = ImmutableMemtable::new(table, 0);

        // when
        let filtered = imm.filter_after_seq(1, None).unwrap();

        // then
        assert!(filtered.table().touched_segments().is_empty());
        assert_eq!(surviving_keys(&filtered), vec![Bytes::from_static(b"b1")]);
    }

    #[tokio::test]
    async fn test_memtable_descending_returns_highest_seq_first_for_same_key() {
        let table = WritableKVTable::new();
        table.put(RowEntry::new_value(b"aaaa", b"v1", 0));
        table.put(RowEntry::new_value(b"bbbb", b"old", 1));
        table.put(RowEntry::new_value(b"bbbb", b"new", 2));
        table.put(RowEntry::new_value(b"cccc", b"v3", 3));

        let mut iter = table
            .table()
            .range(.., IterationOrder::Descending, ReadTrace::new(None));

        // In descending order, for key "bbbb" the newest version (seq 2) must
        // come before the older version (seq 1) so that dedup works correctly.
        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_value(b"cccc", b"v3", 3),
                RowEntry::new_value(b"bbbb", b"new", 2),
                RowEntry::new_value(b"bbbb", b"old", 1),
                RowEntry::new_value(b"aaaa", b"v1", 0),
            ],
        )
        .await;
    }
}
