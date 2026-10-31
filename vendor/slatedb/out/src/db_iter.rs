use crate::batch::WriteBatchIterator;
use crate::bytes_range::BytesRange;
use crate::error::SlateDBError;
use crate::filter_iterator::FilterIterator;
use crate::iter::{EmptyIterator, IterationOrder, RowEntryIterator};
use crate::manifest::LsmTreeState;
use crate::merge_iterator::MergeIterator;
use crate::merge_operator::{
    MergeOperatorIterator, MergeOperatorRequiredIterator, MergeOperatorType,
};
use crate::reader::ReadTrace;
use crate::segment_iterator::{build_l0_point_iters, build_sr_point_iters, SegmentScanContext};
use crate::types::{KeyValue, RowEntry, ValueDeletable};

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{self, BoxStream, StreamExt};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::ops::RangeBounds;
use tracing::Instrument;

/// [`DbIteratorRangeTracker`] records the *requested* scan range of a
/// [`DbIterator`] so that the transaction manager can detect read-write
/// conflicts under Serializable Snapshot Isolation.
///
/// The tracker holds the range the caller asked to scan — not the bounding
/// box of keys the iterator happened to yield. Tracking the requested range
/// is required for correctness: an empty scan still establishes a read
/// dependency on every key in that range (and a concurrent insert anywhere
/// inside it is a phantom), and a partial scan or a scan whose tombstones
/// were skipped likewise depends on the whole requested range.
///
/// Early termination via [`DbIterator::seek`] does not narrow the registered
/// range: the transaction has already taken a logical read dependency on the
/// whole requested range when the scan was issued.
#[derive(Debug)]
pub(crate) struct DbIteratorRangeTracker {
    requested_range: BytesRange,
}

impl DbIteratorRangeTracker {
    pub(crate) fn new(requested_range: BytesRange) -> Self {
        Self { requested_range }
    }

    pub(crate) fn get_range(&self) -> BytesRange {
        self.requested_range.clone()
    }
}

/// Sources for a point lookup, in newest-first order.
/// The stream initializes each source before it yields the source.
///
/// `Mutex` makes this stream `Sync`, as [`RowEntryIterator`] requires.
/// The code accesses `Mutex` only through `get_mut`, so it never locks.
type InitializedSources =
    Mutex<BoxStream<'static, Result<Box<dyn RowEntryIterator + 'static>, SlateDBError>>>;

pub(crate) struct GetIterator {
    key: Bytes,
    sources: InitializedSources,
    current: Option<Box<dyn RowEntryIterator + 'static>>,
}

impl GetIterator {
    /// Builds an ordered stream of initialized sources.
    ///
    /// The stream initializes the first source alone. It initializes at most
    /// `lookahead` remaining sources at the same time.
    ///
    /// `init` calls can finish out of order. The stream yields sources in
    /// their original order.
    fn with_lookahead(
        key: Bytes,
        iters: Vec<Box<dyn RowEntryIterator + 'static>>,
        lookahead: usize,
    ) -> Self {
        async fn init_source(
            mut iter: Box<dyn RowEntryIterator + 'static>,
        ) -> Result<Box<dyn RowEntryIterator + 'static>, SlateDBError> {
            iter.init().await.map(|()| iter)
        }

        let mut iters = iters.into_iter();
        let newest = iters.next();
        let sources = stream::iter(newest)
            .map(init_source)
            .buffered(1)
            .chain(
                stream::iter(iters)
                    .map(init_source)
                    .buffered(lookahead.max(1)),
            )
            .boxed();

        Self {
            key,
            sources: Mutex::new(sources),
            current: None,
        }
    }

    pub(crate) fn new(
        key: Bytes,
        write_batch_iter: Box<dyn RowEntryIterator + 'static>,
        mem_iters: impl IntoIterator<Item = Box<dyn RowEntryIterator + 'static>>,
        segment_iter: Box<dyn RowEntryIterator + 'static>,
    ) -> Self {
        let iters = vec![write_batch_iter]
            .into_iter()
            .chain(mem_iters)
            .chain(std::iter::once(segment_iter))
            .collect();

        // Each source is in memory or is the segment chain.
        // No metadata fetches between these sources can overlap.
        Self::with_lookahead(key, iters, 1)
    }

    /// Build a per-tree `GetIterator` over the L0 + sorted-run iterators
    /// produced by `tree`. The chain stays flat (no `MergeIterator`) so a
    /// bloom-positive hit short-circuits the walk rather than opening every
    /// older SST.
    ///
    /// After the newest source, the iterator initializes up to
    /// `ctx.max_parallel` sources at the same time. Their metadata fetches
    /// overlap. An absent key requires
    /// `1 + ceil((sources - 1) / max_parallel)` round trips, instead of one
    /// round trip for each source.
    ///
    /// If a hit occurs after the newest source, the iterator can initialize up
    /// to `max_parallel - 1` extra sources. Each extra source reads a filter.
    /// If the filter is positive, the source also reads an index and one data
    /// block. The iterator drops pending `init` calls when the walk stops. It
    /// cannot stop the block fetches that those calls started, so the fetches
    /// continue to completion.
    ///
    /// `max_seq` is applied per leaf via `FilterIterator` so above-bound
    /// tombstones are dropped before the outer flat-walk encounters them
    /// and stops the search.
    pub(crate) fn from_lsm_tree(
        key: Bytes,
        tree: &LsmTreeState,
        ctx: &SegmentScanContext,
        max_seq: Option<u64>,
    ) -> Result<Self, SlateDBError> {
        let l0 = build_l0_point_iters(&tree.l0, ctx)?;
        let sr = build_sr_point_iters(&key, &tree.compacted, ctx)?;
        let iters = apply_filters(l0.into_iter().chain(sr), max_seq);
        Ok(Self::with_lookahead(key, iters, ctx.max_parallel))
    }
}

#[async_trait]
impl RowEntryIterator for GetIterator {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        // GetIterator departs from the normal convention for RowEntryIterator
        // in that it lazily initializes the sources only when necessary - this
        // is because it is used in a way that will early exit before all
        // sources are used.
        Ok(())
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        loop {
            if self.current.is_none() {
                match self.sources.get_mut().next().await {
                    Some(source) => self.current = Some(source?),
                    None => return Ok(None),
                }
            }
            let iter = self.current.as_mut().expect("source set above");

            if let Some(entry) = iter.next().await? {
                match &entry.value {
                    ValueDeletable::Value(_) | ValueDeletable::Tombstone => {
                        // Merge operands need their base, but no older entries.
                        // Drop the remaining sources and any pending initialization.
                        self.current = None;
                        *self.sources.get_mut() = stream::empty().boxed();
                        if entry.value.is_tombstone() {
                            return Ok(None);
                        }
                        return Ok(Some(entry));
                    }
                    ValueDeletable::Merge(_) => {
                        return Ok(Some(entry));
                    }
                }
            }
            self.current = None;
        }
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        // we expect the GetIterator to only cover a single key, so if we seek
        // to something other than that key we should just return an error
        if next_key != self.key {
            return Err(SlateDBError::SeekKeyOutOfRange {
                key: next_key.to_vec(),
                range: BytesRange::from(self.key.clone()..=self.key.clone()),
            });
        }

        Ok(())
    }
}

struct ScanIterator {
    delegate: Box<dyn RowEntryIterator + 'static>,
}

impl ScanIterator {
    pub(crate) fn new(
        write_batch_iter: Box<dyn RowEntryIterator + 'static>,
        mem_iters: impl IntoIterator<Item = Box<dyn RowEntryIterator + 'static>>,
        segment_iter: Box<dyn RowEntryIterator + 'static>,
        order: IterationOrder,
    ) -> Result<Self, SlateDBError> {
        // Three-arm top-level merge: write batch (transaction-only),
        // memtables, and the on-disk LSM chain. The chain itself
        // already merges each segment's L0 + sorted runs internally.
        let iters = vec![
            write_batch_iter,
            Box::new(MergeIterator::new_with_order(mem_iters, order)?),
            segment_iter,
        ];

        Ok(Self {
            delegate: Box::new(MergeIterator::new_with_order(iters, order)?),
        })
    }
}

#[async_trait]
impl RowEntryIterator for ScanIterator {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        self.delegate.init().await
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        self.delegate.next().await
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        self.delegate.seek(next_key).await
    }
}

pub struct DbIterator {
    range: BytesRange,
    iter: Box<dyn RowEntryIterator + 'static>,
    invalidated_error: Option<SlateDBError>,
    last_key: Option<Bytes>,
    order: IterationOrder,
    read_span: tracing::Span,
}

impl DbIterator {
    pub(crate) async fn new(
        range: BytesRange,
        write_batch_iter: Option<WriteBatchIterator>,
        mem_iters: impl IntoIterator<Item = Box<dyn RowEntryIterator + 'static>>,
        segment_iter: Box<dyn RowEntryIterator + 'static>,
        max_seq: Option<u64>,
        merge_operator: Option<MergeOperatorType>,
        order: IterationOrder,
        read_trace: ReadTrace,
    ) -> Result<Self, SlateDBError> {
        let read_span = read_trace.read_span();

        // The write_batch iterator is provided only when operating within a Transaction. It represents the uncommitted
        // writes made during the transaction. We do not need to apply the max_seq filter to them, because they do
        // not have an real committed sequence number yet.
        let write_batch_iter = write_batch_iter
            .map(|iter| Box::new(iter) as Box<dyn RowEntryIterator + 'static>)
            .unwrap_or_else(|| Box::new(EmptyIterator::new()));

        // Apply the max_seq filter before any deduping merge. The
        // per-segment merge inside `segment_iter` runs with dedup disabled,
        // so wrapping the chain as a whole is equivalent to wrapping
        // each leaf — versions > max_seq are dropped before the
        // top-level merge dedups.
        //
        // Example: a leaf carries [(key1, seq=96), (key1, seq=110)]
        // and another carries [(key1, seq=95)]. With max_seq=100,
        // filtering after a deduping merge would let seq=110 win and
        // then drop it, losing the correct seq=96 answer. Filter first.
        let mem_iters = apply_filters(mem_iters, max_seq);
        let segment_iter = Box::new(FilterIterator::new_with_max_seq(segment_iter, max_seq))
            as Box<dyn RowEntryIterator + 'static>;

        let mut iter = match range.as_point() {
            Some(key) => Box::new(GetIterator::new(
                key.clone(),
                write_batch_iter,
                mem_iters,
                segment_iter,
            )) as Box<dyn RowEntryIterator + 'static>,
            None => Box::new(ScanIterator::new(
                write_batch_iter,
                mem_iters,
                segment_iter,
                order,
            )?) as Box<dyn RowEntryIterator + 'static>,
        };

        if let Some(merge_operator) = merge_operator {
            iter = Box::new(MergeOperatorIterator::new(
                merge_operator,
                iter,
                true,
                // Its important not to set a snapshot seq num barrier for this merge iterator
                // The entries in the write batch iterator have seq num u64::MAX and any merges
                // there need to be merged with the entries from the other iterators.
                None,
            ));
        } else {
            // When no merge operator is configured, wrap with iterator that errors on merge operands
            iter = Box::new(MergeOperatorRequiredIterator::new(iter));
        }

        iter.init().instrument(read_span.clone()).await?;

        Ok(DbIterator {
            range,
            iter,
            invalidated_error: None,
            last_key: None,
            order,
            read_span,
        })
    }

    /// Get the next key-value pair.
    ///
    /// This method filters out tombstones and returns the user-facing [`KeyValue`] struct,
    /// which contains only the key and value.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the iterator has been invalidated due to an underlying error.
    pub async fn next(&mut self) -> Result<Option<KeyValue>, crate::Error> {
        let entry_opt = self.next_entry().await?;
        match entry_opt {
            Some(entry) => {
                if entry.value.is_tombstone() {
                    return Err(crate::Error::from(SlateDBError::UnexpectedTombstone));
                }
                Ok(Some(KeyValue::from(entry)))
            }
            None => Ok(None),
        }
    }

    pub(crate) async fn next_entry(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        let read_span = self.read_span.clone();
        self.next_entry_inner().instrument(read_span).await
    }

    async fn next_entry_inner(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        if let Some(error) = self.invalidated_error.clone() {
            Err(error)
        } else {
            let result = loop {
                let next = self.iter.next().await;
                // Keep cached iteration cooperative.
                tokio::task::coop::consume_budget().await;
                match next {
                    Ok(Some(entry)) => match entry.value {
                        ValueDeletable::Tombstone => continue,
                        _ => break Ok(Some(entry)),
                    },
                    Ok(None) => break Ok(None),
                    Err(e) => break Err(e),
                }
            };
            let result = self.maybe_invalidate(result);
            if let Ok(Some(ref entry)) = result {
                self.last_key = Some(entry.key.clone());
            }
            result
        }
    }

    fn maybe_invalidate<T: Clone>(
        &mut self,
        result: Result<T, SlateDBError>,
    ) -> Result<T, SlateDBError> {
        if let Err(error) = &result {
            self.invalidated_error = Some(error.clone());
        }
        result
    }

    /// Seek ahead to the next key. The next key must be larger than the
    /// last key returned by the iterator and less than the end bound specified
    /// in the `scan` arguments.
    ///
    /// After a successful seek, the iterator will return the next record
    /// with a key greater than or equal to `next_key`.
    ///
    /// Only supported for ascending scans. Descending scans return an error
    /// because the merge and sorted-run iterators only seek in ascending order.
    ///
    /// # Errors
    ///
    /// Returns an invalid argument error in the following cases:
    ///
    /// - if the scan was opened with [`IterationOrder::Descending`]
    /// - if `next_key` comes before the current iterator position
    /// - if `next_key` is beyond the upper bound specified in the original
    ///   [`crate::db::Db::scan`] parameters
    ///
    /// Returns [`Error`] if the iterator has been invalidated in order to reclaim resources.
    pub async fn seek<K: AsRef<[u8]>>(&mut self, next_key: K) -> Result<(), crate::Error> {
        let next_key = next_key.as_ref();
        if let Some(error) = self.invalidated_error.clone() {
            Err(error.into())
        } else if matches!(self.order, IterationOrder::Descending) {
            Err(SlateDBError::SeekNotSupportedForDescendingScan.into())
        } else if !self.range.contains(&next_key) {
            Err(SlateDBError::SeekKeyOutOfRange {
                key: next_key.to_vec(),
                range: self.range.clone(),
            }
            .into())
        } else if self
            .last_key
            .clone()
            .is_some_and(|last_key| next_key <= last_key)
        {
            Err(SlateDBError::SeekKeyLessThanLastReturnedKey.into())
        } else {
            let result = self.iter.seek(next_key).await;
            self.maybe_invalidate(result).map_err(Into::into)
        }
    }
}

pub(crate) fn apply_filters<T>(
    iters: impl IntoIterator<Item = T>,
    max_seq: Option<u64>,
) -> Vec<Box<dyn RowEntryIterator>>
where
    T: RowEntryIterator + 'static,
{
    match max_seq {
        Some(max_seq) => iters
            .into_iter()
            .map(|iter| {
                Box::new(FilterIterator::new_with_max_seq(iter, Some(max_seq)))
                    as Box<dyn RowEntryIterator + 'static>
            })
            .collect(),
        None => iters
            .into_iter()
            .map(|iter| Box::new(iter) as Box<dyn RowEntryIterator + 'static>)
            .collect(),
    }
}

/// See [`crate::Db::scan_prefix_by_recency`] for the full contract.
pub struct DbRecencyIterator {
    /// Source iterators ordered from most recent to least recent. The
    /// front of the deque is the source currently being drained;
    /// exhausted sources are popped off as the walk proceeds. Each
    /// source has already been wrapped with the sequence-number filter
    /// at construction time.
    iters: VecDeque<Box<dyn RowEntryIterator + 'static>>,
    /// Whether the current front-of-deque iterator has had `init`
    /// called. Reset to false whenever the front is popped, so the
    /// next source's `init` is invoked lazily on the next pull.
    current_initialized: bool,
    /// Sticky error. Once any underlying `init` or `next` call fails,
    /// the error is stashed here and every subsequent `next_entry`
    /// returns it instead of advancing, since after a failure the
    /// iterator's underlying state is unsafe to keep using.
    invalidated_error: Option<SlateDBError>,
    read_span: tracing::Span,
}

impl DbRecencyIterator {
    pub(crate) fn new(
        iters: VecDeque<Box<dyn RowEntryIterator + 'static>>,
        read_span: tracing::Span,
    ) -> Self {
        Self {
            iters,
            current_initialized: false,
            invalidated_error: None,
            read_span,
        }
    }

    /// Returns the next raw [`RowEntry`] in recency order, advancing to the
    /// next source when the current one is exhausted. Tombstones and merge
    /// operands are not filtered. See [`crate::Db::scan_prefix_by_recency`]
    /// for the full contract.
    pub async fn next_entry(&mut self) -> Result<Option<RowEntry>, crate::Error> {
        let read_span = self.read_span.clone();
        self.next_entry_inner().instrument(read_span).await
    }

    async fn next_entry_inner(&mut self) -> Result<Option<RowEntry>, crate::Error> {
        if let Some(error) = &self.invalidated_error {
            return Err(error.clone().into());
        }

        loop {
            let Some(iter) = self.iters.front_mut() else {
                return Ok(None);
            };

            if !self.current_initialized {
                if let Err(e) = iter.init().await {
                    self.invalidated_error = Some(e.clone());
                    return Err(e.into());
                }
                self.current_initialized = true;
            }

            let next = iter.next().await;
            // Keep cached iteration cooperative.
            tokio::task::coop::consume_budget().await;
            match next {
                Ok(Some(entry)) => return Ok(Some(entry)),
                Ok(None) => {
                    self.iters.pop_front();
                    self.current_initialized = false;
                }
                Err(e) => {
                    self.invalidated_error = Some(e.clone());
                    return Err(e.into());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::batch::{WriteBatch, WriteBatchIterator};
    use crate::bytes_range::BytesRange;
    use crate::db_iter::{DbIterator, GetIterator};
    use crate::error::SlateDBError;
    use crate::iter::{EmptyIterator, IterationOrder, RowEntryIterator};
    use crate::merge_operator::MergeOperatorType;
    use crate::reader::ReadTrace;
    use crate::test_utils::{StringConcatMergeOperator, TestIterator};
    use crate::types::RowEntry;
    use async_trait::async_trait;
    use bytes::Bytes;
    use parking_lot::Mutex;
    use rstest::rstest;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// `ProbeRecord` records how the point lookup probes its sources.
    #[derive(Default)]
    struct ProbeRecord {
        in_flight: AtomicUsize,
        peak: AtomicUsize,
        initialized: Mutex<Vec<usize>>,
    }

    /// This source records `init` calls that overlap. The record shows when
    /// probes run at the same time, without object storage.
    struct ProbeIterator {
        id: usize,
        entry: Option<RowEntry>,
        record: Arc<ProbeRecord>,
    }

    #[async_trait]
    impl RowEntryIterator for ProbeIterator {
        async fn init(&mut self) -> Result<(), SlateDBError> {
            self.record.initialized.lock().push(self.id);
            let in_flight = self.record.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.record.peak.fetch_max(in_flight, Ordering::SeqCst);
            // Stand in for the filter fetch: give siblings started in the same
            // window a chance to enter `init` before this one completes.
            tokio::task::yield_now().await;
            self.record.in_flight.fetch_sub(1, Ordering::SeqCst);
            Ok(())
        }

        async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
            Ok(self.entry.take())
        }

        async fn seek(&mut self, _next_key: &[u8]) -> Result<(), SlateDBError> {
            Ok(())
        }
    }

    /// This function creates `sources` probes. The probes in `hits` hold the key.
    /// Each matching probe stores its ID as the value. This value identifies the
    /// selected source.
    fn probe_sources(
        sources: usize,
        hits: &[usize],
    ) -> (Arc<ProbeRecord>, Vec<Box<dyn RowEntryIterator + 'static>>) {
        let record = Arc::new(ProbeRecord::default());
        let iters = (0..sources)
            .map(|id| {
                Box::new(ProbeIterator {
                    id,
                    entry: hits
                        .contains(&id)
                        .then(|| RowEntry::new_value(b"key", id.to_string().as_bytes(), 1)),
                    record: record.clone(),
                }) as Box<dyn RowEntryIterator + 'static>
            })
            .collect();
        (record, iters)
    }

    #[tokio::test]
    async fn test_get_probes_absent_sources_concurrently() {
        let (record, iters) = probe_sources(8, &[]);
        let mut iter = GetIterator::with_lookahead(Bytes::from_static(b"key"), iters, 4);

        assert_eq!(iter.next().await.unwrap(), None);

        // An absent key requires a probe of every source.
        // The probes run in groups of `lookahead` instead of one at a time.
        assert_eq!(record.initialized.lock().len(), 8);
        assert_eq!(record.peak.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn test_get_returns_newest_matching_source() {
        let (_, iters) = probe_sources(8, &[2, 3, 5]);
        let mut iter = GetIterator::with_lookahead(Bytes::from_static(b"key"), iters, 4);

        // The first window initializes sources 2, 3, and 5.
        // The iterator must use source order, not `init` completion order.
        let entry = iter
            .next()
            .await
            .unwrap()
            .expect("sources 2, 3, 5 hold key");
        assert_eq!(entry.value.as_bytes(), Some(Bytes::from_static(b"2")));
    }

    #[tokio::test]
    async fn test_get_hit_in_newest_source_probes_nothing_else() {
        let (record, iters) = probe_sources(8, &[0]);
        let mut iter = GetIterator::with_lookahead(Bytes::from_static(b"key"), iters, 4);

        let entry = iter.next().await.unwrap().expect("newest source holds key");
        assert_eq!(entry.value.as_bytes(), Some(Bytes::from_static(b"0")));
        assert_eq!(iter.next().await.unwrap(), None);
        assert_eq!(iter.next().await.unwrap(), None);

        // The iterator probes the newest source alone.
        // A hit in this source does not cause speculative reads.
        assert_eq!(*record.initialized.lock(), vec![0]);
    }

    #[tokio::test]
    async fn test_get_hit_past_the_newest_source_bounds_overshoot() {
        let (record, iters) = probe_sources(8, &[1]);
        let mut iter = GetIterator::with_lookahead(Bytes::from_static(b"key"), iters, 4);

        iter.next().await.unwrap().expect("source 1 holds key");
        assert_eq!(iter.next().await.unwrap(), None);
        assert_eq!(iter.next().await.unwrap(), None);

        // The iterator probes sources after the newest source in windows.
        // A hit in the first window prevents probes in the remaining chain.
        let initialized = record.initialized.lock();
        assert!(
            initialized.iter().all(|id| *id < 5),
            "probed past the lookahead window: {initialized:?}"
        );
    }

    #[tokio::test]
    async fn test_get_with_unit_lookahead_probes_one_source_at_a_time() {
        let (record, iters) = probe_sources(4, &[]);
        let mut iter = GetIterator::with_lookahead(Bytes::from_static(b"key"), iters, 1);

        assert_eq!(iter.next().await.unwrap(), None);

        assert_eq!(record.peak.load(Ordering::SeqCst), 1);
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_stops_at_base(
        #[values(false, true)] tombstone: bool,
        #[values(false, true)] merge: bool,
    ) {
        let base = if tombstone {
            RowEntry::new_tombstone(b"key", 3)
        } else {
            RowEntry::new_value(b"key", b"base", 3)
        };
        let newer_operand = RowEntry::new_merge(b"key", b"newer", 5);
        let older_operand = RowEntry::new_merge(b"key", b"older", 4);
        let mut base_source = TestIterator::new();
        let mut sources: Vec<Box<dyn RowEntryIterator>> = Vec::new();
        if merge {
            sources.push(Box::new(
                TestIterator::new().with_row_entry(newer_operand.clone()),
            ));
            base_source = base_source.with_row_entry(older_operand.clone());
        }
        sources.push(Box::new(
            base_source
                .with_row_entry(base.clone())
                .with_entry(b"key", b"obsolete", 2),
        ));
        let (record, older_sources) = probe_sources(3, &[0, 1, 2]);
        sources.extend(older_sources);
        let mut iter = GetIterator::with_lookahead(Bytes::from_static(b"key"), sources, 1);

        if merge {
            assert_eq!(iter.next().await.unwrap(), Some(newer_operand));
            assert_eq!(iter.next().await.unwrap(), Some(older_operand));
        }
        assert_eq!(iter.next().await.unwrap(), (!tombstone).then_some(base));
        for _ in 0..3 {
            assert_eq!(iter.next().await.unwrap(), None);
        }
        assert!(record.initialized.lock().is_empty());
    }

    #[rstest]
    #[tokio::test]
    async fn test_point_scan_stops_at_base(
        #[values(false, true)] tombstone: bool,
        #[values(false, true)] merge: bool,
        #[values(IterationOrder::Ascending, IterationOrder::Descending)] order: IterationOrder,
    ) {
        let base = if tombstone {
            RowEntry::new_tombstone(b"key", 3)
        } else {
            RowEntry::new_value(b"key", b"base", 3)
        };
        let mut newest = TestIterator::new().with_row_entry(RowEntry::new_tombstone(b"key", 6));
        if merge {
            newest = newest.with_row_entry(RowEntry::new_merge(b"key", b"newer", 5));
        }
        let mut older = TestIterator::new();
        if merge {
            older = older.with_row_entry(RowEntry::new_merge(b"key", b"older", 4));
        }
        older = older
            .with_row_entry(base)
            .with_entry(b"key", b"obsolete", 2);
        let mut iter = DbIterator::new(
            BytesRange::from(Bytes::from_static(b"key")..=Bytes::from_static(b"key")),
            None,
            vec![
                Box::new(newest) as Box<dyn RowEntryIterator>,
                Box::new(older),
            ],
            Box::new(TestIterator::new().with_entry(b"key", b"oldest", 1)),
            Some(5),
            merge.then(|| Arc::new(StringConcatMergeOperator) as MergeOperatorType),
            order,
            ReadTrace::new(None),
        )
        .await
        .unwrap();

        let expected = match (tombstone, merge) {
            (false, false) => Some(Bytes::from_static(b"base")),
            (true, false) => None,
            (false, true) => Some(Bytes::from_static(b"baseoldernewer")),
            (true, true) => Some(Bytes::from_static(b"oldernewer")),
        };
        assert_eq!(iter.next().await.unwrap().map(|kv| kv.value), expected);
        for _ in 0..3 {
            assert_eq!(iter.next().await.unwrap(), None);
        }
    }

    #[tokio::test]
    async fn test_invalidated_iterator() {
        let mem_iters: VecDeque<Box<dyn RowEntryIterator + 'static>> = VecDeque::new();
        let mut iter = DbIterator::new(
            BytesRange::from(..),
            None,
            mem_iters,
            Box::new(EmptyIterator::new()),
            None,
            None,
            IterationOrder::Ascending,
            ReadTrace::new(None),
        )
        .await
        .unwrap();

        iter.invalidated_error = Some(SlateDBError::ChecksumMismatch { path: None });

        let result = iter.next().await;
        let err = result.expect_err("Failed to return invalidated iterator");
        assert_invalidated_iterator_error(err);

        let result = iter.seek(Bytes::new()).await;
        let err = result.expect_err("Failed to return invalidated iterator");
        assert_invalidated_iterator_error(err);
    }

    fn assert_invalidated_iterator_error(err: crate::Error) {
        assert_eq!(err.to_string(), "Data error: checksum mismatch");
    }

    #[tokio::test]
    async fn test_sequence_number_filtering() {
        // Create two test iterators with overlapping keys but different sequence numbers
        let mem_iter1 = TestIterator::new()
            .with_entry(b"key1", b"value1", 96)
            .with_entry(b"key1", b"value2", 110);

        let mem_iter2 = TestIterator::new().with_entry(b"key1", b"value3", 95);

        // Create DbIterator with max_seq = 100
        let mut iter = DbIterator::new(
            BytesRange::from(..),
            None,
            vec![
                Box::new(mem_iter1) as Box<dyn RowEntryIterator + 'static>,
                Box::new(mem_iter2) as Box<dyn RowEntryIterator + 'static>,
            ],
            Box::new(EmptyIterator::new()),
            Some(100),
            None,
            IterationOrder::Ascending,
            ReadTrace::new(None),
        )
        .await
        .unwrap();

        // The iterator should return the entry with seq=96 from the first memtable
        // and not the one with seq=95 from the second memtable
        let result = iter.next().await.unwrap();
        assert!(result.is_some());
        let kv = result.unwrap();
        assert_eq!(kv.key, Bytes::from("key1"));
        assert_eq!(kv.value, Bytes::from("value1"));
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_seek_cannot_rewind() {
        // Build a simple test iterator with two keys
        let mem_iter = TestIterator::new()
            .with_entry(b"key1", b"value1", 1)
            .with_entry(b"key2", b"value2", 2);

        // Create a DbIterator over the whole range
        let mut iter = DbIterator::new(
            BytesRange::from(..),
            None,
            vec![Box::new(mem_iter) as Box<dyn RowEntryIterator + 'static>],
            Box::new(EmptyIterator::new()),
            None,
            None,
            IterationOrder::Ascending,
            ReadTrace::new(None),
        )
        .await
        .unwrap();

        // Consume the first record
        let first = iter.next().await.unwrap().unwrap();
        assert_eq!(first.key, Bytes::from_static(b"key1"));

        // Seeking to the current key or a prior key should fail
        let err = iter.seek(b"key1").await.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid error: cannot seek to a key less than the last returned key"
        );

        let err = iter.seek(b"key0").await.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Invalid error: cannot seek to a key less than the last returned key"
        );

        // Seeking forward succeeds and allows reading the next key
        iter.seek(b"key2").await.unwrap();
        let kv = iter.next().await.unwrap().unwrap();
        assert_eq!(kv.key, Bytes::from_static(b"key2"));
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_dbiterator_with_writebatch() {
        // Create a WriteBatch with some data
        let mut batch = WriteBatch::new();
        batch.put(b"key1", b"value1");
        batch.put(b"key3", b"value3");

        // Create WriteBatchIterator
        let wb_iter =
            WriteBatchIterator::new(&batch, .., IterationOrder::Ascending, u64::MAX, None, None);

        // Create DbIterator with WriteBatch
        let mem_iters: VecDeque<Box<dyn RowEntryIterator + 'static>> = VecDeque::new();
        let mut iter = DbIterator::new(
            BytesRange::from(..),
            Some(wb_iter),
            mem_iters,
            Box::new(EmptyIterator::new()),
            None,
            None,
            IterationOrder::Ascending,
            ReadTrace::new(None),
        )
        .await
        .unwrap();

        // Should get data from WriteBatch in sorted order
        let kv1 = iter.next().await.unwrap().unwrap();
        assert_eq!(kv1.key, Bytes::from_static(b"key1"));
        assert_eq!(kv1.value, Bytes::from_static(b"value1"));

        let kv2 = iter.next().await.unwrap().unwrap();
        assert_eq!(kv2.key, Bytes::from_static(b"key3"));
        assert_eq!(kv2.value, Bytes::from_static(b"value3"));

        // Should be done
        let kv3 = iter.next().await.unwrap();
        assert!(kv3.is_none());
    }
}
