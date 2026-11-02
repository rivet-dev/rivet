use async_trait::async_trait;
use bytes::Bytes;
use log::error;
use slatedb_common::metrics::CounterFn;
use std::collections::VecDeque;
use std::ops::Bound::{Excluded, Included, Unbounded};
use std::ops::{Bound, Range, RangeBounds};
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::block_iterator::DataBlockIterator;
use crate::bytes_range::BytesRange;
use crate::db_state::{SsTableId, SsTableView};
use crate::db_stats::DbStats;
use crate::error::SlateDBError;
use crate::filter_policy::{FilterContext, FilterQuery, FilterTarget, NamedFilter};
use crate::flatbuffer_types::SsTableIndexOwned;
use crate::format::block::Block;
use crate::reader::{ReadTrace, SstTraceLevel};
use crate::{
    iter::{IterationOrder, RowEntryIterator},
    partitioned_keyspace,
    tablestore::TableStore,
    types::RowEntry,
    utils::panic_string,
};

enum FetchTask {
    InFlight(JoinHandle<Result<VecDeque<Arc<Block>>, SlateDBError>>),
    Finished(VecDeque<Arc<Block>>),
}

#[derive(Clone, Debug)]
pub(crate) struct SstIteratorOptions {
    pub(crate) max_fetch_tasks: usize,
    pub(crate) target_bytes_to_fetch: usize,
    pub(crate) cache_blocks: bool,
    pub(crate) cache_metadata: bool,
    pub(crate) eager_spawn: bool,
    pub(crate) order: IterationOrder,
    pub(crate) prefix: Option<Bytes>,
    pub(crate) filter_context: Option<FilterContext>,
    pub(crate) segment: Option<Bytes>,
}

impl Default for SstIteratorOptions {
    fn default() -> Self {
        SstIteratorOptions {
            max_fetch_tasks: 1,
            target_bytes_to_fetch: 1,
            cache_blocks: true,
            cache_metadata: true,
            eager_spawn: false,
            order: IterationOrder::Ascending,
            prefix: None,
            filter_context: None,
            segment: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SstTracingContext {
    pub(crate) sst_level: SstTraceLevel,
    pub(crate) read_trace: ReadTrace,
}

impl SstTracingContext {
    pub(crate) fn new(sst_level: SstTraceLevel, read_trace: ReadTrace) -> Self {
        Self {
            sst_level,
            read_trace,
        }
    }
}

/// This enum encapsulates access to an SST and corresponding ownership requirements.
/// For example, [`SstView::Owned`] allows the table view to be owned, which is
/// needed for [`crate::db::Db::scan`] since it returns the iterator, while [`SstView::Borrowed`]
/// accommodates access by reference which is useful for [`crate::db::Db::get`].
pub(crate) enum SstView<'a> {
    Owned(Box<SsTableView>, BytesRange),
    Borrowed(&'a SsTableView, BytesRange),
}

impl SstView<'_> {
    fn start_key(&self) -> Bound<&[u8]> {
        match self {
            SstView::Owned(_, r) | SstView::Borrowed(_, r) => r.start_bound().map(|b| b.as_ref()),
        }
    }

    fn end_key(&self) -> Bound<&[u8]> {
        match self {
            SstView::Owned(_, r) | SstView::Borrowed(_, r) => r.end_bound().map(|b| b.as_ref()),
        }
    }

    /// The view's bounds.
    fn bounds(&self) -> (Bound<Bytes>, Bound<Bytes>) {
        match self {
            SstView::Owned(_, r) | SstView::Borrowed(_, r) => (
                r.comparable_start_bound().cloned().into(),
                r.comparable_end_bound().cloned().into(),
            ),
        }
    }

    fn point_key(&self) -> Option<&[u8]> {
        match (self.start_key(), self.end_key()) {
            (Included(start), Included(end)) if start == end => Some(start),
            _ => None,
        }
    }

    fn table_as_ref(&self) -> &SsTableView {
        match self {
            SstView::Owned(t, _) => t,
            SstView::Borrowed(t, _) => t,
        }
    }

    /// Check whether a key is contained within this view.
    fn contains(&self, key: &[u8]) -> bool {
        match self {
            SstView::Owned(_, r) => r.contains(key),
            SstView::Borrowed(_, r) => r.contains(key),
        }
    }

    /// Check whether a key exceeds the range of this view.
    fn key_exceeds(&self, key: &[u8]) -> bool {
        match self.end_key() {
            Included(end) => key > end,
            Excluded(end) => key >= end,
            Unbounded => false,
        }
    }

    /// Check whether a key is below the range of this view.
    fn key_precedes(&self, key: &[u8]) -> bool {
        match self.start_key() {
            Included(start) => key < start,
            Excluded(start) => key <= start,
            Unbounded => false,
        }
    }
}

struct IteratorState {
    initialized: bool,
    current_iter: Option<DataBlockIterator<Arc<Block>>>,
}

impl IteratorState {
    fn new() -> Self {
        Self {
            initialized: false,
            current_iter: None,
        }
    }

    fn is_finished(&self) -> bool {
        self.initialized && self.current_iter.is_none()
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn advance(&mut self, iterator: DataBlockIterator<Arc<Block>>) {
        self.initialized = true;
        self.current_iter = Some(iterator);
    }

    fn stop(&mut self) {
        self.initialized = true;
        self.current_iter = None;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum FilterState {
    NotChecked,
    NoFilter,
    Positive,
    Negative,
}

struct FilterEvaluator {
    query: FilterQuery,
    db_stats: Option<DbStats>,
    state: FilterState,
    found_key: bool,
    false_positive_recorded: bool,
}

impl FilterEvaluator {
    fn new_point(key: Bytes, context: Option<FilterContext>, db_stats: Option<DbStats>) -> Self {
        Self {
            query: FilterQuery::point(key).with_context(context),
            db_stats,
            state: FilterState::NotChecked,
            found_key: false,
            false_positive_recorded: false,
        }
    }

    fn new_prefix(
        prefix: Bytes,
        context: Option<FilterContext>,
        db_stats: Option<DbStats>,
    ) -> Self {
        Self {
            query: FilterQuery::prefix(prefix).with_context(context),
            db_stats,
            state: FilterState::NotChecked,
            found_key: false,
            false_positive_recorded: false,
        }
    }

    fn new_range(
        lower: Bound<Bytes>,
        upper: Bound<Bytes>,
        context: Option<FilterContext>,
        db_stats: Option<DbStats>,
    ) -> Self {
        Self {
            query: FilterQuery::range(lower, upper).with_context(context),
            db_stats,
            state: FilterState::NotChecked,
            found_key: false,
            false_positive_recorded: false,
        }
    }

    /// Evaluate the filters against the query using AND logic.
    ///
    /// All filters must agree the query might match for the read to proceed.
    /// If any filter says the query is absent, the SST is skipped.
    /// If no filters are provided, the state is set to `NoFilter`.
    fn evaluate(
        &mut self,
        filters: &[NamedFilter],
        sst_id: SsTableId,
        sst_level: Option<&SstTraceLevel>,
        read_trace: &ReadTrace,
    ) {
        if self.state != FilterState::NotChecked {
            return;
        }

        if filters.is_empty() {
            self.state = FilterState::NoFilter;
            return;
        }

        // AND logic: if any filter says the key is NOT present, filter it out.
        // All filters reaching here are decoded. TableStore::read_filters
        // resolves any raw cache entries before returning.
        let might_match = filters
            .iter()
            .all(|nf| self.filter_might_match(nf, sst_id, sst_level, read_trace));

        if might_match {
            if let Some(stats) = &self.db_stats {
                self.positives_counter(stats).increment(1);
            }
            self.state = FilterState::Positive;
        } else {
            if let Some(stats) = &self.db_stats {
                self.negatives_counter(stats).increment(1);
            }
            self.state = FilterState::Negative;
        }
    }

    fn filter_might_match(
        &self,
        filter: &NamedFilter,
        sst_id: SsTableId,
        sst_level: Option<&SstTraceLevel>,
        read_trace: &ReadTrace,
    ) -> bool {
        let span = read_trace.new_evaluate_filter_span(sst_id, sst_level, &filter.name);
        let _guard = span.enter();
        let result = filter.filter.might_match(&self.query);
        span.record("result", result);
        result
    }

    fn positives_counter<'a>(&self, stats: &'a DbStats) -> &'a Arc<dyn CounterFn> {
        match &self.query.target {
            FilterTarget::Point(_) => &stats.sst_filter_point_positives,
            FilterTarget::Prefix(_) => &stats.sst_filter_prefix_positives,
            FilterTarget::Range { .. } => &stats.sst_filter_range_positives,
        }
    }

    fn negatives_counter<'a>(&self, stats: &'a DbStats) -> &'a Arc<dyn CounterFn> {
        match &self.query.target {
            FilterTarget::Point(_) => &stats.sst_filter_point_negatives,
            FilterTarget::Prefix(_) => &stats.sst_filter_prefix_negatives,
            FilterTarget::Range { .. } => &stats.sst_filter_range_negatives,
        }
    }

    fn false_positives_counter<'a>(&self, stats: &'a DbStats) -> &'a Arc<dyn CounterFn> {
        match &self.query.target {
            FilterTarget::Point(_) => &stats.sst_filter_point_false_positives,
            FilterTarget::Prefix(_) => &stats.sst_filter_prefix_false_positives,
            FilterTarget::Range { .. } => &stats.sst_filter_range_false_positives,
        }
    }

    fn is_filtered_out(&self) -> bool {
        self.state == FilterState::Negative
    }

    fn notify_key_found(&mut self, key: &[u8]) {
        match &self.query.target {
            FilterTarget::Point(k) if key == k.as_ref() => self.found_key = true,
            FilterTarget::Prefix(p) if key.starts_with(p.as_ref()) => self.found_key = true,
            // Keys the iterator yields are already inside the range, so any
            // row confirms the match.
            FilterTarget::Range { .. } => self.found_key = true,
            _ => {}
        }
    }

    fn notify_finished_iteration(&mut self) {
        if self.state == FilterState::Positive && !self.found_key && !self.false_positive_recorded {
            if let Some(stats) = &self.db_stats {
                self.false_positives_counter(stats).increment(1);
            }
            self.false_positive_recorded = true;
        }
    }
}

pub(crate) struct InternalSstIterator<'a> {
    view: SstView<'a>,
    index: Option<Arc<SsTableIndexOwned>>,
    state: IteratorState,
    next_block_idx_to_fetch: usize,
    block_idx_range: Range<usize>,
    fetch_tasks: VecDeque<FetchTask>,
    table_store: Arc<TableStore>,
    options: SstIteratorOptions,
    tracing_context: Option<SstTracingContext>,
    /// Buffer for descending iteration to maintain correct sequence order within keys.
    descending_buffer: Option<VecDeque<RowEntry>>,
    /// Pending entry that was read ahead but belongs to the next key group.
    /// Only used in descending mode.
    pending_entry: Option<RowEntry>,
}

impl<'a> InternalSstIterator<'a> {
    fn new(
        view: SstView<'a>,
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
        tracing_context: Option<SstTracingContext>,
    ) -> Result<Self, SlateDBError> {
        assert!(options.max_fetch_tasks > 0);
        assert!(options.target_bytes_to_fetch > 0);

        let descending_buffer = match options.order {
            IterationOrder::Descending => Some(VecDeque::new()),
            IterationOrder::Ascending => None,
        };

        Ok(Self {
            view,
            index: None,
            state: IteratorState::new(),
            next_block_idx_to_fetch: 0,
            block_idx_range: 0..0,
            fetch_tasks: VecDeque::new(),
            table_store,
            options,
            tracing_context,
            descending_buffer,
            pending_entry: None,
        })
    }

    fn view(&self) -> &SstView<'a> {
        &self.view
    }

    fn table_store(&self) -> &Arc<TableStore> {
        &self.table_store
    }

    fn sst_level(&self) -> Option<&SstTraceLevel> {
        self.tracing_context
            .as_ref()
            .map(|context| &context.sst_level)
    }

    fn read_trace(&self) -> ReadTrace {
        self.tracing_context
            .as_ref()
            .map(|context| context.read_trace.clone())
            .unwrap_or_else(|| ReadTrace::new(None))
    }

    fn new_owned<T: RangeBounds<Bytes>>(
        range: T,
        table: SsTableView,
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
        tracing_context: Option<SstTracingContext>,
    ) -> Result<Option<Self>, SlateDBError> {
        let Some(view_range) = table.calculate_view_range(BytesRange::from(range)) else {
            return Ok(None);
        };
        let view = SstView::Owned(Box::new(table), view_range);
        Self::new(view, table_store, options, tracing_context).map(Some)
    }

    fn new_borrowed<T: RangeBounds<Bytes>>(
        range: T,
        table: &'a SsTableView,
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
    ) -> Result<Option<Self>, SlateDBError> {
        let Some(view_range) = table.calculate_view_range(BytesRange::from(range)) else {
            return Ok(None);
        };
        let view = SstView::Borrowed(table, view_range);
        Self::new(view, table_store, options, None).map(Some)
    }

    fn for_key(
        table: &'a SsTableView,
        key: &'a [u8],
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
    ) -> Result<Option<Self>, SlateDBError> {
        Self::new_borrowed(
            BytesRange::from_slice(key..=key),
            table,
            table_store,
            options,
        )
    }

    /// Spawns fetch tasks for blocks based on iteration order.
    ///
    /// For ascending order: Fetches blocks forward from `next_block_idx_to_fetch`, incrementing it
    /// as blocks are scheduled. Stops when reaching `block_idx_range.end`.
    ///
    /// For descending order: Fetches blocks backward from `next_block_idx_to_fetch - 1`,
    /// decrementing `next_block_idx_to_fetch` as blocks are scheduled. Stops when reaching
    /// `block_idx_range.start`.
    fn spawn_fetches(&mut self) {
        let Some(index) = self.index.as_ref() else {
            return;
        };

        match self.options.order {
            IterationOrder::Ascending => {
                // Fetch blocks forward: next_block_idx_to_fetch advances toward block_idx_range.end
                while self.fetch_tasks.len() < self.options.max_fetch_tasks
                    && self.block_idx_range.contains(&self.next_block_idx_to_fetch)
                {
                    let table = self.view.table_as_ref().sst.clone();
                    let mut blocks = self.table_store.block_range_for_target_bytes(
                        &table,
                        index,
                        self.next_block_idx_to_fetch,
                        self.options.target_bytes_to_fetch,
                        IterationOrder::Ascending,
                    );
                    blocks.end = blocks.end.min(self.block_idx_range.end);
                    let table_store = self.table_store.clone();
                    let index = index.clone();
                    let cache_blocks = self.options.cache_blocks;
                    let segment = self.options.segment.clone();
                    let blocks_end = blocks.end;
                    self.fetch_tasks
                        .push_back(FetchTask::InFlight(tokio::spawn(async move {
                            table_store
                                .read_blocks_using_index(
                                    &table,
                                    index,
                                    blocks,
                                    cache_blocks,
                                    segment,
                                )
                                .await
                        })));
                    self.next_block_idx_to_fetch = blocks_end;
                }
            }
            IterationOrder::Descending => {
                // Fetch blocks backward: next_block_idx_to_fetch retreats toward block_idx_range.start
                while self.fetch_tasks.len() < self.options.max_fetch_tasks
                    && self.next_block_idx_to_fetch > self.block_idx_range.start
                {
                    let table = self.view.table_as_ref().sst.clone();
                    let mut blocks = self.table_store.block_range_for_target_bytes(
                        &table,
                        index,
                        self.next_block_idx_to_fetch - 1,
                        self.options.target_bytes_to_fetch,
                        IterationOrder::Descending,
                    );
                    blocks.start = blocks.start.max(self.block_idx_range.start);
                    let table_store = self.table_store.clone();
                    let index = index.clone();
                    let cache_blocks = self.options.cache_blocks;
                    let segment = self.options.segment.clone();
                    let blocks_start = blocks.start;
                    self.fetch_tasks
                        .push_back(FetchTask::InFlight(tokio::spawn(async move {
                            table_store
                                .read_blocks_using_index(
                                    &table,
                                    index,
                                    blocks,
                                    cache_blocks,
                                    segment,
                                )
                                .await
                        })));
                    self.next_block_idx_to_fetch = blocks_start;
                }
            }
        }
    }

    async fn next_iter(
        &mut self,
        spawn_fetches: bool,
    ) -> Result<Option<DataBlockIterator<Arc<Block>>>, SlateDBError> {
        if self.index.is_none() {
            return Ok(None);
        }
        let sst_version = self.view.table_as_ref().sst.format_version;
        let sst_id = self.view.table_as_ref().sst.id;
        loop {
            if spawn_fetches {
                self.spawn_fetches();
            }
            if let Some(fetch_task) = self.fetch_tasks.front_mut() {
                match fetch_task {
                    FetchTask::InFlight(jh) => {
                        let blocks = jh
                            .await
                            .map_err(|join_err| block_fetch_join_error(join_err, sst_id))??;
                        *fetch_task = FetchTask::Finished(blocks);
                    }
                    FetchTask::Finished(blocks) => {
                        // For descending order, pop from back; for ascending, pop from front
                        let block = match self.options.order {
                            IterationOrder::Ascending => blocks.pop_front(),
                            IterationOrder::Descending => blocks.pop_back(),
                        };

                        if let Some(block) = block {
                            return Ok(Some(DataBlockIterator::new(
                                block,
                                sst_version,
                                self.options.order,
                            )?));
                        } else {
                            self.fetch_tasks.pop_front();
                        }
                    }
                }
            } else {
                assert!(self.fetch_tasks.is_empty());
                // With spawn_fetches=true, running out of tasks means we've
                // exhausted the entire range.
                // With spawn_fetches=false, it only means the prefetch buffer
                // is drained, but there may still be blocks in the range, and
                // the caller is responsible for scheduling more fetches if
                // needed.
                if spawn_fetches {
                    match self.options.order {
                        IterationOrder::Ascending => {
                            assert_eq!(self.next_block_idx_to_fetch, self.block_idx_range.end);
                        }
                        IterationOrder::Descending => {
                            assert_eq!(self.next_block_idx_to_fetch, self.block_idx_range.start);
                        }
                    }
                }
                return Ok(None);
            }
        }
    }

    async fn advance_block(&mut self) -> Result<(), SlateDBError> {
        self.ensure_metadata_loaded().await?;
        if !self.state.is_finished() {
            if let Some(mut iter) = self.next_iter(true).await? {
                // Only seek on the first block to position at the range boundary.
                // For subsequent blocks, iterate through the entire block in the specified order.
                if !self.state.is_initialized() {
                    match self.options.order {
                        IterationOrder::Ascending => match self.view.start_key() {
                            Included(start_key) | Excluded(start_key) => {
                                iter.seek(start_key).await?
                            }
                            Unbounded => (),
                        },
                        IterationOrder::Descending => match self.view.end_key() {
                            Included(end_key) | Excluded(end_key) => iter.seek(end_key).await?,
                            Unbounded => (),
                        },
                    }
                }
                self.state.advance(iter);
            } else {
                self.state.stop();
            }
        }
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(index) = self.index.as_ref() {
            // For ascending order, stopping means we've gone to the end
            // For descending order, stopping means we've gone to the beginning
            match self.options.order {
                IterationOrder::Ascending => {
                    let num_blocks = index.borrow().block_meta().len();
                    self.next_block_idx_to_fetch = num_blocks;
                }
                IterationOrder::Descending => {
                    self.next_block_idx_to_fetch = 0;
                }
            }
        }
        self.state.stop();
    }

    async fn ensure_metadata_loaded(&mut self) -> Result<(), SlateDBError> {
        if self.index.is_none() {
            let read_trace = self.read_trace();
            let index = self
                .table_store
                .read_index(
                    &self.view.table_as_ref().sst,
                    self.options.cache_metadata,
                    self.options.segment.clone(),
                    &read_trace,
                    self.sst_level(),
                )
                .await?;
            let block_idx_range = partitioned_keyspace::partitions_covering_range(
                &index.borrow(),
                self.view.start_key(),
                self.view.end_key(),
            );
            self.block_idx_range = block_idx_range.clone();
            // For descending order, start from the end and work backwards
            self.next_block_idx_to_fetch = match self.options.order {
                IterationOrder::Ascending => block_idx_range.start,
                IterationOrder::Descending => block_idx_range.end,
            };
            self.index = Some(index);
            if self.options.eager_spawn {
                self.spawn_fetches();
            }
        }
        Ok(())
    }

    async fn fill_descending_buffer(&mut self) -> Result<(), SlateDBError> {
        let mut temp_buffer = Vec::new();
        let mut target_key: Option<Bytes> = None;

        if let Some(pending) = self.pending_entry.take() {
            target_key = Some(pending.key.clone());
            temp_buffer.push(pending);
        }

        loop {
            let next = if let Some(iter) = self.state.current_iter.as_mut() {
                iter.next().await?
            } else {
                None
            };

            match next {
                Some(kv) => {
                    if !self.view.contains(&kv.key) {
                        if self.view.key_precedes(&kv.key) {
                            self.stop();
                            break;
                        }
                        continue;
                    }

                    if target_key.is_none() {
                        target_key = Some(kv.key.clone());
                        temp_buffer.push(kv);
                    } else if Some(&kv.key) == target_key.as_ref() {
                        temp_buffer.push(kv);
                    } else {
                        self.pending_entry = Some(kv);
                        break;
                    }
                }
                None => {
                    self.advance_block().await?;
                    if self.state.is_finished() {
                        break;
                    }
                }
            }
        }

        temp_buffer.reverse();
        self.descending_buffer
            .as_mut()
            .expect("descending_buffer must exist in descending mode")
            .extend(temp_buffer);

        Ok(())
    }
}

#[async_trait]
impl RowEntryIterator for InternalSstIterator<'_> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        if !self.state.is_initialized() {
            self.advance_block().await?;
        }
        Ok(())
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        if !self.state.is_initialized() {
            return Err(SlateDBError::IteratorNotInitialized);
        }

        match self.options.order {
            IterationOrder::Descending => {
                if let Some(buffer) = &mut self.descending_buffer {
                    if let Some(entry) = buffer.pop_front() {
                        return Ok(Some(entry));
                    }
                }

                self.fill_descending_buffer().await?;

                return Ok(self
                    .descending_buffer
                    .as_mut()
                    .expect("descending_buffer must exist in descending mode")
                    .pop_front());
            }
            IterationOrder::Ascending => {}
        }

        while !self.state.is_finished() {
            let next = if let Some(iter) = self.state.current_iter.as_mut() {
                iter.next().await?
            } else {
                None
            };

            match next {
                Some(kv) => {
                    if self.view.contains(&kv.key) {
                        return Ok(Some(kv));
                    } else if self.view.key_exceeds(&kv.key) {
                        self.stop();
                    }
                }
                None => self.advance_block().await?,
            }
        }
        Ok(None)
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        if !self.state.is_initialized() {
            return Err(SlateDBError::IteratorNotInitialized);
        }
        if !self.view.contains(next_key) {
            if self.view.key_exceeds(next_key) {
                match self.options.order {
                    IterationOrder::Ascending => {
                        // Seeking beyond the end of the view range in ascending order
                        // means there are no more results.
                        self.stop();
                        return Ok(());
                    }
                    IterationOrder::Descending => {
                        // Seeking beyond the end in descending order means "start
                        // from the last key and go backwards" — fall through to
                        // the normal seek logic.
                    }
                }
            } else {
                return Err(SlateDBError::SeekKeyOutOfKeyRange {
                    key: next_key.to_vec(),
                    start_key: self.view.start_key().map(|b| b.to_vec()),
                    end_key: self.view.end_key().map(|b| b.to_vec()),
                });
            }
        }
        if !self.state.is_finished() {
            if let Some(iter) = self.state.current_iter.as_mut() {
                iter.seek(next_key).await?;
                if !iter.is_empty() {
                    return Ok(());
                }
            }

            let index = self
                .index
                .as_ref()
                .expect("metadata must be initialized")
                .clone();

            // For descending order, find the last block with the key
            // For ascending order, find the first block with or after the key
            let block_idx = match self.options.order {
                IterationOrder::Ascending => {
                    partitioned_keyspace::first_partition_including_or_after_key(
                        &index.borrow(),
                        next_key,
                    )
                }
                IterationOrder::Descending => {
                    partitioned_keyspace::last_partition_including_key(&index.borrow(), next_key)
                        .unwrap_or(self.block_idx_range.start)
                }
            };

            // Check if block is in the already-fetched direction
            let already_fetched = match self.options.order {
                IterationOrder::Ascending => block_idx < self.next_block_idx_to_fetch,
                IterationOrder::Descending => block_idx >= self.next_block_idx_to_fetch,
            };

            if already_fetched {
                while let Some(mut block_iter) = self.next_iter(false).await? {
                    block_iter.seek(next_key).await?;
                    if !block_iter.is_empty() {
                        self.state.advance(block_iter);
                        return Ok(());
                    }
                }
            }

            self.fetch_tasks.clear();
            self.next_block_idx_to_fetch = match self.options.order {
                IterationOrder::Ascending => block_idx,
                IterationOrder::Descending => block_idx + 1,
            };

            if let Some(mut block_iter) = self.next_iter(true).await? {
                block_iter.seek(next_key).await?;
                self.state.advance(block_iter);
            } else {
                self.state.stop();
            }
        }
        Ok(())
    }
}

struct FilterIterator<'a> {
    inner: InternalSstIterator<'a>,
    filter: FilterEvaluator,
    initialized: bool,
}

impl<'a> FilterIterator<'a> {
    fn new(inner: InternalSstIterator<'a>, filter: FilterEvaluator) -> Self {
        Self {
            inner,
            filter,
            initialized: false,
        }
    }

    fn is_filtered_out(&self) -> bool {
        self.filter.is_filtered_out()
    }

    async fn read_filters(&self) -> Result<Arc<[NamedFilter]>, SlateDBError> {
        let read_trace = self.inner.read_trace();
        self.inner
            .table_store()
            .read_filters(
                &self.inner.view().table_as_ref().sst,
                self.inner.options.cache_metadata,
                self.inner.options.segment.clone(),
                &read_trace,
                self.inner.sst_level(),
            )
            .await
    }
}

#[async_trait]
impl RowEntryIterator for FilterIterator<'_> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        if !self.initialized {
            let filters = self.read_filters().await?;
            let sst_id = self.inner.view().table_as_ref().sst.id;
            let read_trace = self.inner.read_trace();
            self.filter
                .evaluate(&filters, sst_id, self.inner.sst_level(), &read_trace);

            if self.is_filtered_out() {
                return Ok(());
            }

            // make sure initializing the inner iterator only happens after
            // the filter is evaluated to avoid unnecessary work
            self.inner.init().await?;
            self.initialized = true;
        }

        Ok(())
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        if self.is_filtered_out() {
            self.filter.notify_finished_iteration();
            return Ok(None);
        }

        let next = self.inner.next().await?;
        if let Some(entry) = next.as_ref() {
            self.filter.notify_key_found(entry.key.as_ref());
        } else {
            self.filter.notify_finished_iteration();
        }

        Ok(next)
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        if self.is_filtered_out() {
            return Ok(());
        }

        self.inner.seek(next_key).await
    }
}

#[allow(clippy::large_enum_variant)]
enum SstIteratorDelegate<'a> {
    Direct(InternalSstIterator<'a>),
    Filter(FilterIterator<'a>),
}

pub(crate) struct SstIterator<'a> {
    delegate: SstIteratorDelegate<'a>,
}

impl<'a> SstIterator<'a> {
    fn from_internal(internal: InternalSstIterator<'a>, db_stats: Option<DbStats>) -> Self {
        let point_key = internal.view().point_key().map(Bytes::copy_from_slice);
        let prefix = internal.options.prefix.clone();
        let range = internal.view().bounds();
        let filter_context = internal.options.filter_context.clone();
        let filter_evaluator = match (point_key, prefix, range) {
            (Some(key), _, _) => Some(FilterEvaluator::new_point(key, filter_context, db_stats)),
            (None, Some(p), _) => Some(FilterEvaluator::new_prefix(p, filter_context, db_stats)),
            // A plain range scan reads this SST's filters only when there is
            // a registered policy that can answer a range.
            (None, None, (lower, upper))
                if internal
                    .table_store()
                    .any_filter_policy_supports_range_queries() =>
            {
                Some(FilterEvaluator::new_range(
                    lower,
                    upper,
                    filter_context,
                    db_stats,
                ))
            }
            (None, None, _) => None,
        };
        let delegate = match filter_evaluator {
            Some(fe) => SstIteratorDelegate::Filter(FilterIterator::new(internal, fe)),
            None => SstIteratorDelegate::Direct(internal),
        };
        Self { delegate }
    }

    #[allow(dead_code)]
    pub(crate) fn new(
        view: SstView<'a>,
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
    ) -> Result<Self, SlateDBError> {
        Self::new_with_stats(view, table_store, options, None, None)
    }

    pub(crate) fn new_with_stats(
        view: SstView<'a>,
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
        tracing_context: Option<SstTracingContext>,
        db_stats: Option<DbStats>,
    ) -> Result<Self, SlateDBError> {
        let internal = InternalSstIterator::new(view, table_store, options, tracing_context)?;
        Ok(Self::from_internal(internal, db_stats))
    }

    #[allow(dead_code)]
    pub(crate) fn new_owned_with_stats<T: RangeBounds<Bytes>>(
        range: T,
        table: SsTableView,
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
        tracing_context: Option<SstTracingContext>,
        db_stats: Option<DbStats>,
    ) -> Result<Option<Self>, SlateDBError> {
        let internal =
            InternalSstIterator::new_owned(range, table, table_store, options, tracing_context)?;
        Ok(internal.map(|iter| Self::from_internal(iter, db_stats.clone())))
    }

    #[allow(dead_code)]
    pub(crate) fn new_owned<T: RangeBounds<Bytes>>(
        range: T,
        table: SsTableView,
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
        tracing_context: Option<SstTracingContext>,
    ) -> Result<Option<Self>, SlateDBError> {
        Self::new_owned_with_stats(range, table, table_store, options, tracing_context, None)
    }

    #[cfg(test)]
    pub(crate) async fn new_owned_initialized<T: RangeBounds<Bytes>>(
        range: T,
        table: SsTableView,
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
    ) -> Result<Option<Self>, SlateDBError> {
        Self::new_owned_initialized_with_stats(range, table, table_store, options, None, None).await
    }

    pub(crate) async fn new_owned_initialized_with_stats<T: RangeBounds<Bytes>>(
        range: T,
        table: SsTableView,
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
        tracing_context: Option<SstTracingContext>,
        db_stats: Option<DbStats>,
    ) -> Result<Option<Self>, SlateDBError> {
        // Construct the inner iterator without initializing it. The filter
        // is evaluated first so that an SST whose filter rules out the query
        // never pays for an index or data block read.
        let internal =
            InternalSstIterator::new_owned(range, table, table_store, options, tracing_context)?;
        match internal {
            Some(inner) => {
                let mut iterator = Self::from_internal(inner, db_stats);
                match &mut iterator.delegate {
                    SstIteratorDelegate::Filter(filter_iter) => {
                        filter_iter.init().await?;
                        if filter_iter.is_filtered_out() {
                            return Ok(None);
                        }
                    }
                    SstIteratorDelegate::Direct(inner_iter) => {
                        inner_iter.init().await?;
                    }
                }
                Ok(Some(iterator))
            }
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    fn new_borrowed_with_stats<T: RangeBounds<Bytes>>(
        range: T,
        table: &'a SsTableView,
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
        db_stats: Option<DbStats>,
    ) -> Result<Option<Self>, SlateDBError> {
        let internal = InternalSstIterator::new_borrowed(range, table, table_store, options)?;
        Ok(internal.map(|iter| Self::from_internal(iter, db_stats.clone())))
    }

    #[allow(dead_code)]
    pub(crate) fn new_borrowed<T: RangeBounds<Bytes>>(
        range: T,
        table: &'a SsTableView,
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
    ) -> Result<Option<Self>, SlateDBError> {
        Self::new_borrowed_with_stats(range, table, table_store, options, None)
    }

    pub(crate) async fn new_borrowed_initialized<T: RangeBounds<Bytes>>(
        range: T,
        table: &'a SsTableView,
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
    ) -> Result<Option<Self>, SlateDBError> {
        let internal = InternalSstIterator::new_borrowed(range, table, table_store, options)?;
        match internal {
            Some(inner) => {
                let mut iterator = Self::from_internal(inner, None);
                match &mut iterator.delegate {
                    SstIteratorDelegate::Filter(filter_iter) => {
                        filter_iter.init().await?;
                        if filter_iter.is_filtered_out() {
                            return Ok(None);
                        }
                    }
                    SstIteratorDelegate::Direct(inner_iter) => {
                        inner_iter.init().await?;
                    }
                }
                Ok(Some(iterator))
            }
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn for_key_with_stats(
        table: &'a SsTableView,
        key: &'a [u8],
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
        db_stats: Option<DbStats>,
    ) -> Result<Option<Self>, SlateDBError> {
        let internal = InternalSstIterator::for_key(table, key, table_store, options)?;
        Ok(internal.map(|iter| Self::from_internal(iter, db_stats.clone())))
    }

    #[cfg(test)]
    pub(crate) async fn for_key_with_stats_initialized(
        table: &'a SsTableView,
        key: &'a [u8],
        table_store: Arc<TableStore>,
        options: SstIteratorOptions,
        db_stats: Option<DbStats>,
    ) -> Result<Option<Self>, SlateDBError> {
        let internal = InternalSstIterator::for_key(table, key, table_store, options)?;
        match internal {
            Some(inner) => {
                let mut iterator = Self::from_internal(inner, db_stats);
                match &mut iterator.delegate {
                    SstIteratorDelegate::Filter(filter_iter) => {
                        filter_iter.init().await?;
                        if filter_iter.is_filtered_out() {
                            return Ok(None);
                        }
                    }
                    SstIteratorDelegate::Direct(inner_iter) => {
                        inner_iter.init().await?;
                    }
                }
                Ok(Some(iterator))
            }
            None => Ok(None),
        }
    }
}

#[async_trait]
impl RowEntryIterator for SstIterator<'_> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        match &mut self.delegate {
            SstIteratorDelegate::Direct(inner) => inner.init().await,
            SstIteratorDelegate::Filter(inner) => inner.init().await,
        }
    }

    async fn next(&mut self) -> Result<Option<RowEntry>, SlateDBError> {
        match &mut self.delegate {
            SstIteratorDelegate::Direct(inner) => inner.next().await,
            SstIteratorDelegate::Filter(inner) => inner.next().await,
        }
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        match &mut self.delegate {
            SstIteratorDelegate::Direct(inner) => inner.seek(next_key).await,
            SstIteratorDelegate::Filter(inner) => inner.seek(next_key).await,
        }
    }
}

/// Converts a failed join on a block fetch task into an error.
///
/// A fetch task is cancelled when the runtime it was spawned on shuts down,
/// so the iterator reports the cancellation to its caller instead of panicking
/// the task that is awaiting the fetch.
fn block_fetch_join_error(join_err: tokio::task::JoinError, sst_id: SsTableId) -> SlateDBError {
    let task_name = format!("sst_block_fetch[{:?}]", sst_id);
    match join_err.try_into_panic() {
        Ok(panic_err) => {
            error!(
                "sst block fetch task panicked unexpectedly. [task_name={}, panic={}]",
                task_name,
                panic_string(&panic_err),
            );
            SlateDBError::BackgroundTaskPanic(task_name)
        }
        Err(_) => SlateDBError::BackgroundTaskCancelled(task_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_cache_policy::BlockCachePolicy;
    use crate::bytes_generator::OrderedBytesGenerator;
    use crate::config::TracingOptions;
    use crate::db_cache::test_utils::TestCache;
    use crate::db_cache::DbCache;
    use crate::db_cache::SplitCache;
    use crate::db_state::{SsTableId, SsTableView};
    use crate::db_stats::DbStats;
    use crate::filter_policy::{
        BloomFilterPolicy, Filter, FilterBuilder, FilterPolicy, FilterQuery,
    };
    use crate::format::sst::SsTableFormat;
    use crate::sst_builder::BlockFormat;
    use crate::tablestore::TableStoreKind;
    use crate::test_utils::{assert_kv, SpanRecorder};
    use crate::types::{KeyValue, ValueDeletable};
    use object_store::path::Path;
    use object_store::{memory::InMemory, ObjectStore};
    use rstest::rstest;
    use slatedb_common::metrics::{
        lookup_metric_with_labels, DefaultMetricsRecorder, MetricLevel, MetricsRecorderHelper,
    };
    use std::collections::HashMap;
    use std::sync::Arc;
    use tracing_subscriber::layer::SubscriberExt;
    use ulid::Ulid;

    fn test_sst_id(id: u64) -> SsTableId {
        SsTableId::from(Ulid::from_parts(id, 0))
    }

    #[tokio::test]
    async fn test_one_block_sst_iter() {
        test_one_block_sst_iter_with_order(IterationOrder::Ascending).await;
        test_one_block_sst_iter_with_order(IterationOrder::Descending).await;
    }

    async fn test_one_block_sst_iter_with_order(order: IterationOrder) {
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
        builder
            .add_value(b"key4", b"value4", Some(4), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let sst_handle = table_store
            .open_sst(&test_sst_id(0), Some(Bytes::new()))
            .await
            .unwrap();
        let index = table_store
            .read_index(
                &sst_handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert_eq!(index.borrow().block_meta().len(), 1);

        let sst_iter_options = SstIteratorOptions {
            cache_blocks: true,
            order,
            ..SstIteratorOptions::default()
        };
        let mut iter = SstIterator::new_owned_initialized(
            ..,
            SsTableView::identity(sst_handle),
            table_store.clone(),
            sst_iter_options,
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // Expected keys based on order
        let expected_keys = match order {
            IterationOrder::Ascending => vec![b"key1", b"key2", b"key3", b"key4"],
            IterationOrder::Descending => vec![b"key4", b"key3", b"key2", b"key1"],
        };
        let expected_values = match order {
            IterationOrder::Ascending => vec![b"value1", b"value2", b"value3", b"value4"],
            IterationOrder::Descending => vec![b"value4", b"value3", b"value2", b"value1"],
        };

        for (expected_key, expected_value) in expected_keys.iter().zip(expected_values.iter()) {
            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            assert_eq!(kv.key, expected_key.as_slice());
            assert_eq!(kv.value, expected_value.as_slice());
        }
        let kv = iter.next().await.unwrap().map(KeyValue::from);
        assert!(kv.is_none());
    }

    #[tokio::test]
    async fn should_record_bloom_filter_positive_for_single_key() {
        // given
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), MetricLevel::default());
        let db_stats = DbStats::new(&helper);
        let table_store = bloom_filter_enabled_table_store(10);
        let sst_handle = build_single_block_sst(&table_store, &[b"k1", b"k2"]).await;

        // when
        let mut iter = SstIterator::for_key_with_stats_initialized(
            &sst_handle,
            b"k1",
            table_store.clone(),
            SstIteratorOptions::default(),
            Some(db_stats.clone()),
        )
        .await
        .expect("iterator construction should succeed")
        .expect("expected iterator for present key");
        let entry = iter
            .next()
            .await
            .expect("iteration should succeed")
            .expect("expected entry for present key");

        // then
        assert_eq!(entry.key.as_ref(), b"k1");
        match entry.value {
            ValueDeletable::Value(value) => assert_eq!(value.as_ref(), b"v_k1"),
            other => panic!("expected value, found {other:?}"),
        }
        assert_eq!(
            lookup_metric_with_labels(
                &recorder,
                crate::db_stats::SST_FILTER_POSITIVE_COUNT,
                &[(
                    crate::db_stats::FILTER_KIND_LABEL,
                    crate::db_stats::FILTER_KIND_POINT,
                )],
            ),
            Some(1)
        );
        assert_eq!(
            lookup_metric_with_labels(
                &recorder,
                crate::db_stats::SST_FILTER_FALSE_POSITIVE_COUNT,
                &[(
                    crate::db_stats::FILTER_KIND_LABEL,
                    crate::db_stats::FILTER_KIND_POINT,
                )],
            ),
            Some(0)
        );
    }

    #[tokio::test]
    async fn should_record_bloom_filter_negative_for_missing_key() {
        // given
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), MetricLevel::default());
        let db_stats = DbStats::new(&helper);
        let table_store = bloom_filter_enabled_table_store(10);
        let sst_handle = build_single_block_sst(&table_store, &[b"k1", b"k3"]).await;

        // when
        let iter = SstIterator::for_key_with_stats_initialized(
            &sst_handle,
            b"k2",
            table_store,
            SstIteratorOptions::default(),
            Some(db_stats.clone()),
        )
        .await
        .expect("iterator construction should succeed");

        // then
        assert!(iter.is_none(), "negative bloom result should skip iterator");
        assert_eq!(
            lookup_metric_with_labels(
                &recorder,
                crate::db_stats::SST_FILTER_NEGATIVE_COUNT,
                &[(
                    crate::db_stats::FILTER_KIND_LABEL,
                    crate::db_stats::FILTER_KIND_POINT,
                )],
            ),
            Some(1)
        );
        assert_eq!(
            lookup_metric_with_labels(
                &recorder,
                crate::db_stats::SST_FILTER_FALSE_POSITIVE_COUNT,
                &[(
                    crate::db_stats::FILTER_KIND_LABEL,
                    crate::db_stats::FILTER_KIND_POINT,
                )],
            ),
            Some(0)
        );
    }

    #[tokio::test]
    async fn should_record_bloom_filter_false_positive_for_single_key() {
        // given
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), MetricLevel::default());
        let db_stats = DbStats::new(&helper);
        let table_store = bloom_filter_enabled_table_store(2);
        // these keys share the same bucket in the bloom filter (hard coded)
        // after testing with the SIP13 algorithm. The collision key must be
        // within the SST's key range [k1, k3] for range pruning.
        let existing_keys = [b"k1".as_slice(), b"k3".as_slice()];
        let sst_handle = build_single_block_sst(&table_store, &existing_keys).await;

        let read_trace = ReadTrace::new(None);
        let filters = table_store
            .read_filters(&sst_handle.sst, true, Some(Bytes::new()), &read_trace, None)
            .await
            .expect("filter read should succeed");
        assert!(!filters.is_empty(), "filter should exist");

        let collision_key = b"k12";
        assert!(
            filters[0]
                .filter
                .might_match(&FilterQuery::point(Bytes::from_static(collision_key))),
            "bloom filter should report collision for hard-coded key"
        );

        // when
        let mut iter = SstIterator::for_key_with_stats_initialized(
            &sst_handle,
            collision_key,
            table_store.clone(),
            SstIteratorOptions::default(),
            Some(db_stats.clone()),
        )
        .await
        .expect("iterator construction should succeed")
        .expect("filter positive should yield iterator");

        let entry = iter.next().await.expect("iteration should succeed");

        // then
        assert!(entry.is_none(), "false positive must return no entry");
        let point_labels = &[(
            crate::db_stats::FILTER_KIND_LABEL,
            crate::db_stats::FILTER_KIND_POINT,
        )];
        assert_eq!(
            lookup_metric_with_labels(
                &recorder,
                crate::db_stats::SST_FILTER_POSITIVE_COUNT,
                point_labels,
            ),
            Some(1)
        );
        assert_eq!(
            lookup_metric_with_labels(
                &recorder,
                crate::db_stats::SST_FILTER_FALSE_POSITIVE_COUNT,
                point_labels,
            ),
            Some(1)
        );
        assert_eq!(
            lookup_metric_with_labels(
                &recorder,
                crate::db_stats::SST_FILTER_NEGATIVE_COUNT,
                point_labels,
            ),
            Some(0)
        );
    }

    #[tokio::test]
    async fn test_bloom_filter_iterator_caches_filter_regardless_of_cache_blocks() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let writer = TableStore::new(
            object_store.clone(),
            format.clone(),
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = writer.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", Some(2), None)
            .await
            .unwrap();
        let sst = writer
            .write_sst(
                &SsTableId::from(Ulid::new()),
                &builder.build().await.unwrap(),
                Some(Bytes::new()),
            )
            .await
            .unwrap();
        let handle = SsTableView::identity(sst);

        let meta_cache = Arc::new(TestCache::new());
        let cache = Arc::new(
            SplitCache::new()
                .with_meta_cache(Some(meta_cache.clone()))
                .build(),
        );
        let reader = Arc::new(TableStore::new(
            object_store,
            format,
            root_path,
            Some(cache),
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));

        let filter_key = (handle.sst.id, handle.sst.info.filter_offset).into();

        for cache_blocks in [false, true] {
            meta_cache.remove(&filter_key).await;

            let options = SstIteratorOptions {
                cache_blocks,
                ..Default::default()
            };
            let mut iter = SstIterator::for_key_with_stats_initialized(
                &handle,
                b"key1",
                reader.clone(),
                options,
                None,
            )
            .await
            .expect("iterator construction should succeed")
            .expect("expected iterator for present key");
            let _ = iter.next().await.unwrap();

            assert!(meta_cache.get_filter(&filter_key).await.unwrap().is_some());
        }

        meta_cache.remove(&filter_key).await;
        let options = SstIteratorOptions {
            cache_metadata: false,
            ..Default::default()
        };
        let mut iter =
            SstIterator::for_key_with_stats_initialized(&handle, b"key1", reader, options, None)
                .await
                .expect("iterator construction should succeed")
                .expect("expected iterator for present key");
        let _ = iter.next().await.unwrap();

        assert!(meta_cache.get_filter(&filter_key).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_sst_iterator_caches_index_regardless_of_cache_blocks() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let writer = TableStore::new(
            object_store.clone(),
            format.clone(),
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = writer.table_builder();
        builder
            .add_value(b"key1", b"value1", Some(1), None)
            .await
            .unwrap();
        builder
            .add_value(b"key2", b"value2", Some(2), None)
            .await
            .unwrap();
        let sst = writer
            .write_sst(
                &SsTableId::from(Ulid::new()),
                &builder.build().await.unwrap(),
                Some(Bytes::new()),
            )
            .await
            .unwrap();
        let handle = SsTableView::identity(sst);

        let meta_cache = Arc::new(TestCache::new());
        let cache = Arc::new(
            SplitCache::new()
                .with_meta_cache(Some(meta_cache.clone()))
                .build(),
        );
        let reader = Arc::new(TableStore::new(
            object_store,
            format,
            root_path,
            Some(cache),
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));

        let index_key = (handle.sst.id, handle.sst.info.index_offset).into();

        for cache_blocks in [false, true] {
            meta_cache.remove(&index_key).await;

            let options = SstIteratorOptions {
                cache_blocks,
                ..Default::default()
            };
            let mut iter = SstIterator::for_key_with_stats_initialized(
                &handle,
                b"key1",
                reader.clone(),
                options,
                None,
            )
            .await
            .expect("iterator construction should succeed")
            .expect("expected iterator for present key");
            let _ = iter.next().await.unwrap();

            assert!(meta_cache.get_index(&index_key).await.unwrap().is_some());
        }

        meta_cache.remove(&index_key).await;
        let options = SstIteratorOptions {
            cache_metadata: false,
            ..Default::default()
        };
        let mut iter =
            SstIterator::for_key_with_stats_initialized(&handle, b"key1", reader, options, None)
                .await
                .expect("iterator construction should succeed")
                .expect("expected iterator for present key");
        let _ = iter.next().await.unwrap();

        assert!(meta_cache.get_index(&index_key).await.unwrap().is_none());
    }

    fn bloom_filter_enabled_table_store(filter_bits_per_key: u32) -> Arc<TableStore> {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 1,
            filter_policies: vec![Arc::new(BloomFilterPolicy::new(filter_bits_per_key))],
            ..SsTableFormat::default()
        };
        Arc::new(TableStore::new(
            object_store,
            format,
            root_path,
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ))
    }

    async fn build_single_block_sst(table_store: &Arc<TableStore>, keys: &[&[u8]]) -> SsTableView {
        let mut builder = table_store.table_builder();
        for key in keys {
            let value = format!("v_{}", String::from_utf8_lossy(key));
            builder
                .add_value(key, value.as_bytes(), Some(0), None)
                .await
                .unwrap();
        }
        let encoded = builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        SsTableView::identity(
            table_store
                .write_sst(&id, &encoded, Some(Bytes::new()))
                .await
                .unwrap(),
        )
    }

    #[tokio::test]
    async fn test_many_block_sst_iter() {
        test_many_block_sst_iter_with_order(IterationOrder::Ascending).await;
        test_many_block_sst_iter_with_order(IterationOrder::Descending).await;
    }

    async fn test_many_block_sst_iter_with_order(order: IterationOrder) {
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

        for i in 0..1000 {
            builder
                .add_value(
                    format!("key{}", i).as_bytes(),
                    format!("value{}", i).as_bytes(),
                    Some(i),
                    None,
                )
                .await
                .unwrap();
        }

        let encoded = builder.build().await.unwrap();
        table_store
            .write_sst(&test_sst_id(0), &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let sst_handle = table_store
            .open_sst(&test_sst_id(0), Some(Bytes::new()))
            .await
            .unwrap();
        let index = table_store
            .read_index(
                &sst_handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert_eq!(index.borrow().block_meta().len(), 8);

        let sst_iter_options = SstIteratorOptions {
            max_fetch_tasks: 3,
            target_bytes_to_fetch: 3 * 4096,
            cache_blocks: true,
            order,
            ..SstIteratorOptions::default()
        };
        let mut iter = SstIterator::new_owned_initialized(
            ..,
            SsTableView::identity(sst_handle),
            table_store.clone(),
            sst_iter_options,
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        match order {
            IterationOrder::Ascending => {
                for i in 0..1000 {
                    let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
                    assert_eq!(kv.key, format!("key{}", i));
                    assert_eq!(kv.value, format!("value{}", i));
                }
            }
            IterationOrder::Descending => {
                for i in (0..1000).rev() {
                    let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
                    assert_eq!(kv.key, format!("key{}", i));
                    assert_eq!(kv.value, format!("value{}", i));
                }
            }
        }

        let next = iter.next().await.unwrap().map(KeyValue::from);
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn test_iter_from_key() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 128,
            min_filter_keys: 1,
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
        let first_key = [b'a'; 16];
        let key_gen = OrderedBytesGenerator::new_with_byte_range(&first_key, b'a', b'z');
        let mut test_case_key_gen = key_gen.clone();
        let first_val = [1u8; 16];
        let val_gen = OrderedBytesGenerator::new_with_byte_range(&first_val, 1u8, 26u8);
        let mut test_case_val_gen = val_gen.clone();
        let (sst, nkeys) = build_sst_with_n_blocks(3, table_store.clone(), key_gen, val_gen).await;

        // iterate over all keys and make sure we iterate from that key
        for i in 0..nkeys {
            let mut expected_key_gen = test_case_key_gen.clone();
            let mut expected_val_gen = test_case_val_gen.clone();
            let from_key = test_case_key_gen.next();
            let _ = test_case_val_gen.next();
            let mut iter = SstIterator::new_borrowed_initialized(
                BytesRange::from_slice(from_key.as_ref()..),
                &sst,
                table_store.clone(),
                SstIteratorOptions::default(),
            )
            .await
            .unwrap()
            .expect("Expected Some(iter) but got None");
            for _ in 0..nkeys - i {
                let e = iter.next().await.unwrap().unwrap().into();
                assert_kv(
                    &e,
                    expected_key_gen.next().as_ref(),
                    expected_val_gen.next().as_ref(),
                );
            }
            assert!(iter.next().await.unwrap().is_none());
        }
    }

    #[tokio::test]
    async fn test_iter_from_key_smaller_than_first() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 128,
            min_filter_keys: 1,
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
        let first_key = [b'b'; 16];
        let key_gen = OrderedBytesGenerator::new_with_byte_range(&first_key, b'a', b'y');
        let mut expected_key_gen = key_gen.clone();
        let first_val = [2u8; 16];
        let val_gen = OrderedBytesGenerator::new_with_byte_range(&first_val, 1u8, 26u8);
        let mut expected_val_gen = val_gen.clone();
        let (sst, nkeys) = build_sst_with_n_blocks(2, table_store.clone(), key_gen, val_gen).await;

        let mut iter = SstIterator::new_borrowed_initialized(
            BytesRange::from_slice([b'a'; 16].as_ref()..),
            &sst,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        for _ in 0..nkeys {
            let e = iter.next().await.unwrap().unwrap().into();
            assert_kv(
                &e,
                expected_key_gen.next().as_ref(),
                expected_val_gen.next().as_ref(),
            );
        }
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_iter_from_key_larger_than_last() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 128,
            min_filter_keys: 1,
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
        let first_key = [b'b'; 16];
        let key_gen = OrderedBytesGenerator::new_with_byte_range(&first_key, b'a', b'y');
        let first_val = [2u8; 16];
        let val_gen = OrderedBytesGenerator::new_with_byte_range(&first_val, 1u8, 26u8);
        let (sst, _) = build_sst_with_n_blocks(2, table_store.clone(), key_gen, val_gen).await;

        let iter = SstIterator::new_borrowed_initialized(
            BytesRange::from_slice([b'z'; 16].as_ref()..),
            &sst,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap();

        // The SST's key range doesn't overlap with the query range starting at 'z',
        // so the iterator should be pruned (None).
        assert!(iter.is_none());
    }

    #[tokio::test]
    async fn test_descending_seek_beyond_last_key() {
        test_descending_seek_beyond_last_key_with_format(BlockFormat::V1).await;
        test_descending_seek_beyond_last_key_with_format(BlockFormat::V2).await;
        test_descending_seek_beyond_last_key_with_format(BlockFormat::Latest).await;
    }

    async fn test_descending_seek_beyond_last_key_with_format(block_format: BlockFormat) {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 128,
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

        // Build SST with specified format (keys 0-99)
        let builder = table_store.table_builder();
        let mut builder = match block_format {
            BlockFormat::V1 => builder,
            BlockFormat::V2 => builder.with_block_format(BlockFormat::V2),
            BlockFormat::Latest => builder.with_block_format(BlockFormat::Latest),
        };

        for i in 0..100 {
            builder
                .add_value(
                    format!("key{:03}", i).as_bytes(),
                    format!("value{:03}", i).as_bytes(),
                    Some(i),
                    None,
                )
                .await
                .unwrap();
        }

        let encoded = builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        let sst_handle = SsTableView::identity(
            table_store
                .write_sst(&id, &encoded, Some(Bytes::new()))
                .await
                .unwrap(),
        );

        // Initialize iterator in descending order with full range
        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            &sst_handle,
            table_store.clone(),
            SstIteratorOptions {
                order: IterationOrder::Descending,
                ..SstIteratorOptions::default()
            },
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // Seek to key999 (beyond the last key which is key099)
        iter.seek(b"key999").await.unwrap();

        // Should iterate backwards from key099
        let kv1 = iter
            .next()
            .await
            .unwrap()
            .map(KeyValue::from)
            .expect("Expected first key but got None");
        let kv2 = iter
            .next()
            .await
            .unwrap()
            .map(KeyValue::from)
            .expect("Expected second key but got None");

        assert_eq!(kv1.key.as_ref(), b"key099");
        assert_eq!(kv2.key.as_ref(), b"key098");
    }

    #[tokio::test]
    async fn test_iter_seek_through_sst() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 128,
            min_filter_keys: 1,
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
        let first_key = [b'b'; 16];
        let key_gen = OrderedBytesGenerator::new_with_byte_range(&first_key, b'a', b'y');
        let first_val = [2u8; 16];
        let val_gen = OrderedBytesGenerator::new_with_byte_range(&first_val, 1u8, 26u8);
        let (sst, nkeys) =
            build_sst_with_n_blocks(256, table_store.clone(), key_gen, val_gen).await;

        let mut iter_large_fetch = SstIterator::new_borrowed_initialized(
            ..,
            &sst,
            table_store.clone(),
            SstIteratorOptions {
                max_fetch_tasks: 32,
                target_bytes_to_fetch: 256 * 128,
                cache_blocks: true,
                cache_metadata: true,
                eager_spawn: false,
                order: IterationOrder::Ascending,
                prefix: None,
                filter_context: None,
                segment: Some(Bytes::new()),
            },
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        let mut iter_small_fetch = SstIterator::new_borrowed_initialized(
            ..,
            &sst,
            table_store.clone(),
            SstIteratorOptions {
                max_fetch_tasks: 1,
                target_bytes_to_fetch: 1,
                cache_blocks: true,
                cache_metadata: true,
                eager_spawn: false,
                order: IterationOrder::Ascending,
                prefix: None,
                filter_context: None,
                segment: Some(Bytes::new()),
            },
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        let mut key_gen = OrderedBytesGenerator::new_with_byte_range(&first_key, b'a', b'y');
        let mut val_gen = OrderedBytesGenerator::new_with_byte_range(&first_val, 1u8, 26u8);
        let mut key_values = Vec::new();
        for _ in 0..nkeys {
            key_values.push((key_gen.next(), val_gen.next()));
        }

        for i in (0..nkeys).step_by(100) {
            iter_large_fetch.seek(&key_values[i].0).await.unwrap();
            let kv_large_fetch: KeyValue = iter_large_fetch.next().await.unwrap().unwrap().into();

            iter_small_fetch.seek(&key_values[i].0).await.unwrap();
            let kv_small_fetch: KeyValue = iter_small_fetch.next().await.unwrap().unwrap().into();

            assert_eq!(kv_large_fetch.key, key_values[i].0);
            assert_eq!(kv_large_fetch.value, key_values[i].1);
            assert_eq!(kv_small_fetch.key, key_values[i].0);
            assert_eq!(kv_small_fetch.value, key_values[i].1);
        }
    }

    async fn build_sst_with_n_blocks(
        n: usize,
        ts: Arc<TableStore>,
        mut key_gen: OrderedBytesGenerator,
        mut val_gen: OrderedBytesGenerator,
    ) -> (SsTableView, usize) {
        let mut writer = ts.table_writer(test_sst_id(0), Some(Bytes::new()));
        let mut nkeys = 0usize;
        while writer.blocks_written() < n {
            let entry = RowEntry::new_value(key_gen.next().as_ref(), val_gen.next().as_ref(), 0);
            writer.add(entry).await.unwrap();
            nkeys += 1;
        }
        let (sst, _) = writer.close().await.unwrap();
        (SsTableView::identity(sst), nkeys)
    }

    #[tokio::test]
    #[cfg(feature = "moka")]
    async fn test_sst_iter_cache_blocks() {
        use crate::db_cache::moka::MokaCache;
        use crate::db_cache::DbCache;
        use crate::db_cache::SplitCache;

        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 3,
            ..SsTableFormat::default()
        };
        let block_cache = Arc::new(MokaCache::new());
        let meta_cache = Arc::new(MokaCache::new());
        let split_cache = Arc::new(
            SplitCache::new()
                .with_block_cache(Some(block_cache.clone()))
                .with_meta_cache(Some(meta_cache))
                .build(),
        );
        let table_store = Arc::new(TableStore::new(
            object_store,
            format,
            root_path.clone(),
            Some(split_cache.clone()),
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
        builder
            .add_value(b"key4", b"value4", Some(4), None)
            .await
            .unwrap();
        let encoded = builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        table_store
            .write_sst(&id, &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let sst_handle = table_store.open_sst(&id, Some(Bytes::new())).await.unwrap();

        let sst_iter_options = SstIteratorOptions {
            cache_blocks: true,
            ..SstIteratorOptions::default()
        };
        let mut iter = SstIterator::new_owned_initialized(
            ..,
            SsTableView::identity(sst_handle),
            table_store.clone(),
            sst_iter_options,
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        for i in 1..=4 {
            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            assert_eq!(kv.key, format!("key{}", i).as_bytes());
            assert_eq!(kv.value, format!("value{}", i).as_bytes());
        }

        let kv = iter.next().await.unwrap().map(KeyValue::from);
        assert!(kv.is_none());

        // verify that block was cached
        assert!(block_cache
            .get_block(&(id, 0).into())
            .await
            .unwrap_or(None)
            .is_some());

        // remove block from cache and verify that it is not cached when iterating with cache_blocks=false
        block_cache.remove(&(id, 0).into()).await;
        let sst_handle = table_store.open_sst(&id, Some(Bytes::new())).await.unwrap();
        let sst_iter_options = SstIteratorOptions {
            cache_blocks: false,
            ..SstIteratorOptions::default()
        };
        let mut iter = SstIterator::new_owned_initialized(
            ..,
            SsTableView::identity(sst_handle),
            table_store.clone(),
            sst_iter_options,
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        for i in 1..=4 {
            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            assert_eq!(kv.key, format!("key{}", i).as_bytes());
            assert_eq!(kv.value, format!("value{}", i).as_bytes());
        }

        let kv = iter.next().await.unwrap().map(KeyValue::from);
        assert!(kv.is_none());

        // verify that block is not cached
        assert!(block_cache
            .get_block(&(id, 0).into())
            .await
            .unwrap()
            .is_none());
    }

    async fn build_v2_sst(
        table_store: &Arc<TableStore>,
        keys_and_values: &[(&[u8], &[u8])],
    ) -> SsTableView {
        let mut builder = table_store
            .table_builder()
            .with_block_format(BlockFormat::V2);
        for (key, value) in keys_and_values {
            builder.add_value(key, value, Some(0), None).await.unwrap();
        }
        let encoded = builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        SsTableView::identity(
            table_store
                .write_sst(&id, &encoded, Some(Bytes::new()))
                .await
                .unwrap(),
        )
    }

    #[tokio::test]
    async fn should_iterate_v2_sst_scan() {
        // given: a V2 SST with multiple keys
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

        let keys_and_values = vec![
            (b"key1".as_slice(), b"value1".as_slice()),
            (b"key2".as_slice(), b"value2".as_slice()),
            (b"key3".as_slice(), b"value3".as_slice()),
            (b"key4".as_slice(), b"value4".as_slice()),
        ];
        let sst_handle = build_v2_sst(&table_store, &keys_and_values).await;

        // when: iterating over the SST
        let sst_iter_options = SstIteratorOptions {
            cache_blocks: true,
            ..SstIteratorOptions::default()
        };
        let mut iter = SstIterator::new_owned_initialized(
            ..,
            sst_handle,
            table_store.clone(),
            sst_iter_options,
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // then: all keys should be returned in order
        for (expected_key, expected_value) in &keys_and_values {
            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            assert_eq!(kv.key, *expected_key);
            assert_eq!(kv.value, *expected_value);
        }
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_iterate_v2_sst_for_key() {
        // given: a V2 SST with multiple keys
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

        let keys_and_values = vec![
            (b"key1".as_slice(), b"value1".as_slice()),
            (b"key2".as_slice(), b"value2".as_slice()),
            (b"key3".as_slice(), b"value3".as_slice()),
            (b"key4".as_slice(), b"value4".as_slice()),
        ];
        let sst_handle = build_v2_sst(&table_store, &keys_and_values).await;

        // when: searching for key2
        let mut iter = SstIterator::for_key_with_stats_initialized(
            &sst_handle,
            b"key2",
            table_store.clone(),
            SstIteratorOptions::default(),
            None,
        )
        .await
        .expect("iterator construction should succeed")
        .expect("expected iterator for present key");

        // then: key2 should be found
        let entry = iter
            .next()
            .await
            .expect("iteration should succeed")
            .expect("expected entry for present key");
        assert_eq!(entry.key.as_ref(), b"key2");
        match entry.value {
            ValueDeletable::Value(value) => assert_eq!(value.as_ref(), b"value2"),
            other => panic!("expected value, found {other:?}"),
        }
    }

    #[tokio::test]
    async fn should_iterate_v2_sst_with_many_keys() {
        // given: a V2 SST with many keys to test prefix compression
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 256,
            min_filter_keys: 1000,
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

        // Create keys with shared prefixes to exercise prefix compression
        let mut builder = table_store
            .table_builder()
            .with_block_format(BlockFormat::V2);

        let num_keys = 100;
        for i in 0..num_keys {
            let key = format!("prefix_{:04}", i);
            let value = format!("value_{:04}", i);
            builder
                .add_value(key.as_bytes(), value.as_bytes(), Some(i), None)
                .await
                .unwrap();
        }

        let encoded = builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        let sst_handle = SsTableView::identity(
            table_store
                .write_sst(&id, &encoded, Some(Bytes::new()))
                .await
                .unwrap(),
        );

        // when: iterating over all keys
        let sst_iter_options = SstIteratorOptions {
            cache_blocks: true,
            ..SstIteratorOptions::default()
        };
        let mut iter = SstIterator::new_owned_initialized(
            ..,
            sst_handle,
            table_store.clone(),
            sst_iter_options,
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // then: all keys should be returned in order
        for i in 0..num_keys {
            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            let expected_key = format!("prefix_{:04}", i);
            let expected_value = format!("value_{:04}", i);
            assert_eq!(kv.key, expected_key.as_bytes());
            assert_eq!(kv.value, expected_value.as_bytes());
        }
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_seek_v2_sst_across_multiple_blocks() {
        // given: a V2 SST with small block_size to force multiple blocks
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 128, // Small block size to force multiple blocks
            min_filter_keys: 1000,
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

        // Create keys that will span multiple blocks
        let mut builder = table_store
            .table_builder()
            .with_block_format(BlockFormat::V2);

        let num_keys = 50;
        for i in 0..num_keys {
            let key = format!("key_{:04}", i);
            let value = format!("value_{:04}", i);
            builder
                .add_value(key.as_bytes(), value.as_bytes(), Some(i), None)
                .await
                .unwrap();
        }

        let encoded = builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        let sst_handle = table_store
            .write_sst(&id, &encoded, Some(Bytes::new()))
            .await
            .unwrap();

        // Verify we have multiple blocks
        let index = table_store
            .read_index(
                &sst_handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert!(
            index.borrow().block_meta().len() > 1,
            "Expected multiple blocks but got {}",
            index.borrow().block_meta().len()
        );

        // when: seeking to a key in a later block (key_0030)
        let sst_view = SsTableView::identity(sst_handle);
        let seek_key = b"key_0030";
        let mut iter = SstIterator::new_borrowed_initialized(
            BytesRange::from_slice(seek_key.as_ref()..),
            &sst_view,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // then: should iterate from key_0030 onwards
        for i in 30..num_keys {
            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            let expected_key = format!("key_{:04}", i);
            let expected_value = format!("value_{:04}", i);
            assert_eq!(kv.key, expected_key.as_bytes());
            assert_eq!(kv.value, expected_value.as_bytes());
        }
        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn should_return_none_for_missing_key_in_v2_sst() {
        // given: a V2 SST with multiple blocks
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 128,
            min_filter_keys: 1000,
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

        let mut builder = table_store
            .table_builder()
            .with_block_format(BlockFormat::V2);

        // Add keys with gaps (only even numbers)
        for i in (0..50).step_by(2) {
            let key = format!("key_{:04}", i);
            let value = format!("value_{:04}", i);
            builder
                .add_value(key.as_bytes(), value.as_bytes(), Some(i), None)
                .await
                .unwrap();
        }

        let encoded = builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        let sst_handle = SsTableView::identity(
            table_store
                .write_sst(&id, &encoded, Some(Bytes::new()))
                .await
                .unwrap(),
        );

        // when: searching for a non-existent key (odd number)
        let mut iter = SstIterator::for_key_with_stats_initialized(
            &sst_handle,
            b"key_0025", // This key doesn't exist
            table_store.clone(),
            SstIteratorOptions::default(),
            None,
        )
        .await
        .expect("iterator construction should succeed")
        .expect("expected iterator");

        // then: should return None since key doesn't exist
        let entry = iter.next().await.expect("iteration should succeed");
        assert!(entry.is_none(), "expected None for missing key");
    }

    #[tokio::test]
    async fn should_seek_past_last_key_in_v2_sst() {
        // given: a V2 SST
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 128,
            min_filter_keys: 1000,
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

        let mut builder = table_store
            .table_builder()
            .with_block_format(BlockFormat::V2);

        for i in 0..20 {
            let key = format!("key_{:04}", i);
            let value = format!("value_{:04}", i);
            builder
                .add_value(key.as_bytes(), value.as_bytes(), Some(i), None)
                .await
                .unwrap();
        }

        let encoded = builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        let sst_handle = SsTableView::identity(
            table_store
                .write_sst(&id, &encoded, Some(Bytes::new()))
                .await
                .unwrap(),
        );

        // when: seeking past the last key
        let iter = SstIterator::new_borrowed_initialized(
            BytesRange::from_slice(b"zzz".as_ref()..),
            &sst_handle,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap();

        // then: the SST should be pruned entirely since "zzz" is beyond the last key
        assert!(iter.is_none());
    }

    /// Test: iteration with both start and end bounds where the keys are in the middle of blocks
    /// and neither the first nor last block of the SST is included in the range.
    #[tokio::test]
    async fn test_range_iteration_middle_blocks_ascending() {
        test_range_iteration_middle_blocks_with_order(IterationOrder::Ascending).await;
    }

    #[tokio::test]
    async fn test_range_iteration_middle_blocks_descending() {
        test_range_iteration_middle_blocks_with_order(IterationOrder::Descending).await;
    }

    async fn test_range_iteration_middle_blocks_with_order(order: IterationOrder) {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 128, // Small block size to ensure multiple blocks
            min_filter_keys: 100,
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

        // Build an SST with enough keys to span multiple blocks
        // Using key pattern: key000, key001, ..., key099
        let mut builder = table_store.table_builder();
        for i in 0..100 {
            builder
                .add_value(
                    format!("key{:03}", i).as_bytes(),
                    format!("value{:03}", i).as_bytes(),
                    Some(i),
                    None,
                )
                .await
                .unwrap();
        }
        let encoded = builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        table_store
            .write_sst(&id, &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let sst_handle = table_store.open_sst(&id, Some(Bytes::new())).await.unwrap();

        let index = table_store
            .read_index(
                &sst_handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        let num_blocks = index.borrow().block_meta().len();
        assert!(
            num_blocks >= 4,
            "Test requires at least 4 blocks, got {}",
            num_blocks
        );

        // Use start and end keys that:
        // 1. Are not at the first key of any block (using middle range key020..key079)
        // 2. Exclude the first block entirely (start > first block's last key)
        // 3. Exclude the last block entirely (end < last block's first key)
        // 4. Are guaranteed to exist in the SST (key020 through key079)
        let start_key = b"key020";
        let end_key = b"key079";

        let sst_iter_options = SstIteratorOptions {
            max_fetch_tasks: 3,
            target_bytes_to_fetch: 3 * 128,
            cache_blocks: true,
            cache_metadata: true,
            eager_spawn: false,
            order,
            prefix: None,
            filter_context: None,
            segment: Some(Bytes::new()),
        };
        let mut iter = SstIterator::new_owned_initialized(
            BytesRange::from_slice(start_key.as_ref()..=end_key.as_ref()),
            SsTableView::identity(sst_handle),
            table_store.clone(),
            sst_iter_options,
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // Expected keys based on order
        let start_idx = 20;
        let end_idx = 79; // inclusive
        let expected_count = end_idx - start_idx + 1;

        let mut count = 0;
        match order {
            IterationOrder::Ascending => {
                for i in start_idx..=end_idx {
                    let kv = iter
                        .next()
                        .await
                        .unwrap()
                        .map(KeyValue::from)
                        .unwrap_or_else(|| {
                            panic!(
                                "Expected key{:03} in ascending order, but got None. Count so far: {}",
                                i, count
                            )
                        });
                    assert_eq!(
                        kv.key,
                        format!("key{:03}", i).as_bytes(),
                        "Key mismatch in ascending order at position {}",
                        count
                    );
                    assert_eq!(kv.value, format!("value{:03}", i).as_bytes());
                    count += 1;
                }
            }
            IterationOrder::Descending => {
                for i in (start_idx..=end_idx).rev() {
                    let kv = iter
                        .next()
                        .await
                        .unwrap()
                        .map(KeyValue::from)
                        .unwrap_or_else(|| {
                            panic!(
                                "Expected key{:03} in descending order, but got None. Count so far: {}",
                                i, count
                            )
                        });
                    assert_eq!(
                        kv.key,
                        format!("key{:03}", i).as_bytes(),
                        "Key mismatch in descending order at position {}, expected key{:03}",
                        count,
                        i
                    );
                    assert_eq!(kv.value, format!("value{:03}", i).as_bytes());
                    count += 1;
                }
            }
        }

        assert_eq!(
            count, expected_count,
            "Should iterate exactly {} keys",
            expected_count
        );
        assert!(
            iter.next().await.unwrap().is_none(),
            "Should have no more keys"
        );
    }

    #[tokio::test]
    async fn test_full_descending_iteration() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 128, // Small block size to ensure multiple blocks
            min_filter_keys: 100,
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

        // Build an SST with enough data for multiple blocks
        let mut builder = table_store.table_builder();
        for i in 0..30 {
            builder
                .add_value(
                    format!("key{:03}", i).as_bytes(),
                    format!("value{:03}", i).as_bytes(),
                    Some(i),
                    None,
                )
                .await
                .unwrap();
        }
        let encoded = builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        table_store
            .write_sst(&id, &encoded, Some(Bytes::new()))
            .await
            .unwrap();
        let sst_handle = table_store.open_sst(&id, Some(Bytes::new())).await.unwrap();

        let index = table_store
            .read_index(
                &sst_handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        let num_blocks = index.borrow().block_meta().len();
        assert!(
            num_blocks >= 2,
            "Test requires at least 2 blocks, got {}",
            num_blocks
        );

        // Full iteration in descending order
        let sst_iter_options = SstIteratorOptions {
            cache_blocks: true,
            order: IterationOrder::Descending,
            ..SstIteratorOptions::default()
        };
        let mut iter = SstIterator::new_owned_initialized(
            ..,
            SsTableView::identity(sst_handle),
            table_store.clone(),
            sst_iter_options,
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // Should iterate backwards from key029 to key000
        for i in (0..30).rev() {
            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            assert_eq!(kv.key, format!("key{:03}", i).as_bytes());
            assert_eq!(kv.value, format!("value{:03}", i).as_bytes());
        }

        assert!(iter.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_duplicate_keys_iteration() {
        test_duplicate_keys_iteration_with_order(IterationOrder::Ascending).await;
        test_duplicate_keys_iteration_with_order(IterationOrder::Descending).await;
    }

    async fn test_duplicate_keys_iteration_with_order(order: IterationOrder) {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat::default();
        let table_store = Arc::new(TableStore::new(
            object_store,
            format,
            root_path.clone(),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));

        let mut writer = table_store.table_writer(test_sst_id(0), Some(Bytes::new()));
        writer
            .add(RowEntry::new_value(b"key_a", b"value_100", 100))
            .await
            .unwrap();
        writer
            .add(RowEntry::new_value(b"key_a", b"value_95", 95))
            .await
            .unwrap();
        writer
            .add(RowEntry::new_value(b"key_a", b"value_90", 90))
            .await
            .unwrap();
        writer
            .add(RowEntry::new_value(b"key_b", b"value_80", 80))
            .await
            .unwrap();
        writer
            .add(RowEntry::new_value(b"key_b", b"value_70", 70))
            .await
            .unwrap();
        writer
            .add(RowEntry::new_value(b"key_c", b"value_50", 50))
            .await
            .unwrap();
        let (handle, _) = writer.close().await.unwrap();

        let mut iter = SstIterator::new_owned_initialized(
            ..,
            SsTableView::identity(handle),
            table_store.clone(),
            SstIteratorOptions {
                order,
                ..SstIteratorOptions::default()
            },
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        let expected = match order {
            IterationOrder::Ascending => vec![
                (b"key_a".as_slice(), b"value_100".as_slice(), 100),
                (b"key_a".as_slice(), b"value_95".as_slice(), 95),
                (b"key_a".as_slice(), b"value_90".as_slice(), 90),
                (b"key_b".as_slice(), b"value_80".as_slice(), 80),
                (b"key_b".as_slice(), b"value_70".as_slice(), 70),
                (b"key_c".as_slice(), b"value_50".as_slice(), 50),
            ],
            IterationOrder::Descending => vec![
                (b"key_c".as_slice(), b"value_50".as_slice(), 50),
                (b"key_b".as_slice(), b"value_80".as_slice(), 80),
                (b"key_b".as_slice(), b"value_70".as_slice(), 70),
                (b"key_a".as_slice(), b"value_100".as_slice(), 100),
                (b"key_a".as_slice(), b"value_95".as_slice(), 95),
                (b"key_a".as_slice(), b"value_90".as_slice(), 90),
            ],
        };

        for (expected_key, expected_value, expected_seq) in expected {
            let entry = iter
                .next()
                .await
                .expect("iteration should succeed")
                .expect("expected entry");
            assert_eq!(entry.key.as_ref(), expected_key, "key mismatch");
            assert_eq!(entry.seq, expected_seq, "sequence number mismatch");
            match entry.value {
                ValueDeletable::Value(value) => {
                    assert_eq!(value.as_ref(), expected_value, "value mismatch")
                }
                other => panic!("expected value, found {other:?}"),
            }
        }

        let entry = iter.next().await.expect("iteration should succeed");
        assert!(entry.is_none(), "expected end of iteration");
    }

    #[tokio::test]
    async fn test_seek_forward_after_scan_already_fetched_block_consumed() {
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        // Small block_size so we get ~2 keys per block.
        let format = SsTableFormat {
            block_size: 64,
            min_filter_keys: 1,
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

        // Keys spaced by 10: key_000, key_010, key_020, ..., key_190.
        // Gaps like key_035 don't exist.
        let mut writer = table_store.table_writer(test_sst_id(0), Some(Bytes::new()));
        for i in 0..20 {
            let key = format!("key_{:03}", i * 10);
            let val = format!("val_{:03}", i * 10);
            writer
                .add(RowEntry::new_value(key.as_bytes(), val.as_bytes(), 0))
                .await
                .unwrap();
        }
        let (sst_handle, _) = writer.close().await.unwrap();
        let sst = SsTableView::identity(sst_handle);

        // Minimal prefetch: 1 block at a time, 1 task max.
        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            &sst,
            table_store.clone(),
            SstIteratorOptions {
                max_fetch_tasks: 1,
                target_bytes_to_fetch: 1,
                cache_blocks: true,
                cache_metadata: true,
                eager_spawn: false,
                order: IterationOrder::Ascending,
                prefix: None,
                filter_context: None,
                segment: Some(Bytes::new()),
            },
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // Scan 3 entries: key_000, key_010, key_020.
        // After this, current block has key_020, key_030. fetch_tasks is empty.
        // next_block_idx_to_fetch = 2.
        for _ in 0..3 {
            iter.next().await.unwrap().expect("should have entries");
        }

        // Seek to key_035 (doesn't exist). The index maps it to block 1
        // (which contains key_020, key_030). block_idx=1 < next_block_idx=2,
        // so already_fetched=true. But fetch_tasks is empty (consumed during
        // scan). The already_fetched loop immediately calls next_iter(false)
        // on empty tasks, hitting the assertion.
        iter.seek(b"key_035").await.unwrap();

        // Should find key_040 (first key >= key_035).
        let entry = iter.next().await.unwrap().expect("should find key_040");
        let kv: KeyValue = entry.into();
        assert_eq!(kv.key.as_ref(), b"key_040");
    }

    #[tokio::test]
    async fn test_next_iter_prefetch_task_cancelled() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let table_store = Arc::new(TableStore::new(
            object_store,
            SsTableFormat::default(),
            Path::from(""),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let sst = build_single_block_sst(&table_store, &[b"key1", b"key2"]).await;

        // A runtime that is shut down before anything is spawned on it.
        // `shutdown_background` rather than a plain drop, which would itself
        // panic inside an async context.
        let dead = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .build()
            .unwrap();
        let dead_handle = dead.handle().clone();
        dead.shutdown_background();

        // Initialization walks `advance_block` -> `next_iter(true)` ->
        // `spawn_fetches`, so the first fetch is spawned onto - and cancelled
        // by - the dead runtime while its context is entered.
        let result = {
            let _guard = dead_handle.enter();
            SstIterator::new_owned_initialized(
                ..,
                sst,
                table_store.clone(),
                SstIteratorOptions::default(),
            )
            .await
        };

        let Err(err) = result else {
            panic!("a cancelled prefetch task must be reported as an error");
        };
        assert!(matches!(err, SlateDBError::BackgroundTaskCancelled(_)));
    }

    /// A [`Filter`] backed by a closure, so a test can express a verdict rule
    /// without a real filter's encoding. The rule lives in the closure, so
    /// `encode` writes a placeholder byte and a policy's `decode` rebuilds the
    /// rule rather than reading it back.
    struct TestFilter(Arc<dyn Fn(&FilterQuery) -> bool + Send + Sync>);

    impl Filter for TestFilter {
        fn might_match(&self, query: &FilterQuery) -> bool {
            (self.0)(query)
        }

        fn encode(&self, writer: &mut dyn bytes::BufMut) {
            writer.put_u8(0);
        }

        fn size(&self) -> usize {
            1
        }

        fn clamp_allocated_size(&self) -> Arc<dyn Filter> {
            Arc::new(Self(Arc::clone(&self.0)))
        }
    }

    fn named_filter(
        name: &str,
        verdict: impl Fn(&FilterQuery) -> bool + Send + Sync + 'static,
    ) -> NamedFilter {
        NamedFilter {
            name: name.to_string(),
            filter: Arc::new(TestFilter(Arc::new(verdict))),
        }
    }

    #[rstest]
    #[case::filter_returns_true(true)]
    #[case::filter_returns_false(false)]
    #[test]
    fn test_evaluate_filter_span_fields(#[case] filter_result: bool) {
        const TRACE_ID: &str = "filter-evaluation-trace";
        const FILTER_NAME: &str = "test.filter";
        const SORTED_RUN_ID: u32 = 7;

        let span_recorder = SpanRecorder::for_name("slatedb.read.evaluate_filter");
        let subscriber = tracing_subscriber::registry().with(span_recorder.clone());
        let sst_id = test_sst_id(1);
        tracing::subscriber::with_default(subscriber, || {
            let mut evaluator = FilterEvaluator::new_point(Bytes::from_static(b"key"), None, None);
            evaluator.evaluate(
                &[named_filter(FILTER_NAME, move |_| filter_result)],
                sst_id,
                Some(&SstTraceLevel::SortedRun(SORTED_RUN_ID)),
                &ReadTrace::new(Some(TracingOptions::new(TRACE_ID))),
            );
            assert_eq!(evaluator.is_filtered_out(), !filter_result);
        });

        let span = span_recorder.only_span();
        assert_eq!(
            span.fields,
            HashMap::from([
                ("trace_id".to_string(), TRACE_ID.to_string()),
                ("sst_id".to_string(), sst_id.value().to_string()),
                (
                    "sst_level".to_string(),
                    format!("sorted_run:{SORTED_RUN_ID}"),
                ),
                ("filter_name".to_string(), FILTER_NAME.to_string()),
                ("result".to_string(), filter_result.to_string()),
            ])
        );
    }

    // Three doubles cover the range tests:
    // - `abstaining_filter` stands in for a policy that indexes keys alone,
    //   which cannot answer a range and so always matches.
    // - `context_parity_filter` answers from the query context instead of the
    //   target, so it is the one that can reject a range.
    // - `bounds_filter` checks the bounds themselves.

    /// Accepts all queries.
    fn abstaining_filter() -> NamedFilter {
        named_filter("test.abstaining", |_| true)
    }

    /// Accepts when the context byte's parity is `parity`, and abstains
    /// without a context.
    fn context_parity_filter(parity: u64) -> NamedFilter {
        named_filter("test.context_parity", move |query| match &query.context {
            Some(FilterContext::Inline(buf)) => u64::from(buf[0]) % 2 == parity,
            _ => true,
        })
    }

    /// Rejects unless the range bounds are exactly `expected`.
    fn bounds_filter(expected: (Bound<Bytes>, Bound<Bytes>)) -> NamedFilter {
        named_filter("test.bounds", move |query| match &query.target {
            FilterTarget::Range { lower, upper } => (lower.clone(), upper.clone()) == expected,
            _ => false,
        })
    }

    fn context(byte: u8) -> Option<FilterContext> {
        let mut buf = [0u8; 64];
        buf[0] = byte;
        Some(FilterContext::Inline(buf))
    }

    fn stats() -> (Arc<DefaultMetricsRecorder>, DbStats) {
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), MetricLevel::default());
        let db_stats = DbStats::new(&helper);
        (recorder, db_stats)
    }

    fn metric(recorder: &Arc<DefaultMetricsRecorder>, name: &str, kind: &str) -> Option<i64> {
        lookup_metric_with_labels(
            recorder,
            name,
            &[(crate::db_stats::FILTER_KIND_LABEL, kind)],
        )
    }

    /// `(positives, negatives, false_positives)` recorded for `kind`.
    fn verdicts(
        recorder: &Arc<DefaultMetricsRecorder>,
        kind: &str,
    ) -> (Option<i64>, Option<i64>, Option<i64>) {
        (
            metric(recorder, crate::db_stats::SST_FILTER_POSITIVE_COUNT, kind),
            metric(recorder, crate::db_stats::SST_FILTER_NEGATIVE_COUNT, kind),
            metric(
                recorder,
                crate::db_stats::SST_FILTER_FALSE_POSITIVE_COUNT,
                kind,
            ),
        )
    }

    fn range_evaluator(
        context: Option<FilterContext>,
        db_stats: Option<DbStats>,
    ) -> FilterEvaluator {
        FilterEvaluator::new_range(
            Included(Bytes::from_static(b"a")),
            Excluded(Bytes::from_static(b"z")),
            context,
            db_stats,
        )
    }

    #[tokio::test]
    async fn should_reject_range_query_when_only_one_of_several_filters_rejects() {
        let (recorder, db_stats) = stats();
        let mut evaluator = range_evaluator(context(1), Some(db_stats));

        // AND logic: one filter abstains, so the verdict is the context
        // filter's alone. This is how a range scan is pruned in practice.
        evaluator.evaluate(
            &[abstaining_filter(), context_parity_filter(0)],
            SsTableId::new(Ulid::new()),
            None,
            &ReadTrace::new(None),
        );

        assert!(evaluator.is_filtered_out());
        assert_eq!(
            verdicts(&recorder, crate::db_stats::FILTER_KIND_RANGE),
            (Some(0), Some(1), Some(0))
        );
    }

    #[tokio::test]
    async fn should_accept_range_query_when_every_filter_agrees() {
        let (recorder, db_stats) = stats();
        let mut evaluator = range_evaluator(context(0), Some(db_stats));

        evaluator.evaluate(
            &[abstaining_filter(), context_parity_filter(0)],
            SsTableId::new(Ulid::new()),
            None,
            &ReadTrace::new(None),
        );
        assert!(!evaluator.is_filtered_out());

        // Any row in the view's range confirms a range verdict, so no false
        // positive is recorded.
        evaluator.notify_key_found(b"anything");
        evaluator.notify_finished_iteration();

        assert_eq!(
            verdicts(&recorder, crate::db_stats::FILTER_KIND_RANGE),
            (Some(1), Some(0), Some(0))
        );
    }

    #[tokio::test]
    async fn should_record_range_false_positive_when_no_row_is_yielded() {
        let (recorder, db_stats) = stats();
        let mut evaluator = range_evaluator(context(0), Some(db_stats));

        evaluator.evaluate(
            &[context_parity_filter(0)],
            SsTableId::new(Ulid::new()),
            None,
            &ReadTrace::new(None),
        );
        evaluator.notify_finished_iteration();

        assert_eq!(
            verdicts(&recorder, crate::db_stats::FILTER_KIND_RANGE),
            (Some(1), Some(0), Some(1))
        );
    }

    #[tokio::test]
    async fn should_report_the_views_bounds() {
        let table_store = bloom_filter_enabled_table_store(10);
        let table = build_single_block_sst(&table_store, &[b"k1"]).await;
        let view = SstView::Owned(
            Box::new(table),
            BytesRange::new(
                Included(Bytes::from_static(b"b")),
                Excluded(Bytes::from_static(b"y")),
            ),
        );

        assert_eq!(
            view.bounds(),
            (
                Included(Bytes::from_static(b"b")),
                Excluded(Bytes::from_static(b"y"))
            )
        );
    }

    #[tokio::test]
    async fn should_pass_the_callers_range_bounds_to_the_filter() {
        let (recorder, db_stats) = stats();
        let mut evaluator = range_evaluator(context(0), Some(db_stats));

        evaluator.evaluate(
            &[bounds_filter((
                Included(Bytes::from_static(b"a")),
                Excluded(Bytes::from_static(b"z")),
            ))],
            SsTableId::new(Ulid::new()),
            None,
            &ReadTrace::new(None),
        );

        assert!(!evaluator.is_filtered_out());
        assert_eq!(
            verdicts(&recorder, crate::db_stats::FILTER_KIND_RANGE),
            (Some(1), Some(0), Some(0))
        );
    }

    /// A policy whose filter rejects every range query, registered on a real
    /// table store so a scan is pruned end to end.
    struct RangeCapablePolicy;

    impl FilterPolicy for RangeCapablePolicy {
        fn name(&self) -> &str {
            "test.range_capable"
        }

        fn builder(&self) -> Box<dyn FilterBuilder> {
            Box::new(RangeCapableBuilder)
        }

        fn decode(&self, _data: &[u8]) -> Arc<dyn Filter> {
            RangeCapableBuilder.build()
        }

        fn estimate_size(&self, _num_keys: usize) -> usize {
            1
        }

        fn supports_range_queries(&self) -> bool {
            true
        }
    }

    struct RangeCapableBuilder;

    impl FilterBuilder for RangeCapableBuilder {
        fn add_entry(&mut self, _entry: &RowEntry) {}

        fn build(&mut self) -> Arc<dyn Filter> {
            Arc::new(TestFilter(Arc::new(|query: &FilterQuery| {
                !matches!(query.target, FilterTarget::Range { .. })
            })))
        }
    }

    #[tokio::test]
    async fn should_not_read_filters_for_a_range_when_no_policy_supports_ranges() {
        // A bloom filter cannot answer a range, so a bloom-only database must
        // build no evaluator for a range scan and must not read the SST's
        // filters, with or without a caller-supplied context.
        for filter_context in [context(0), None] {
            let (recorder, db_stats) = stats();
            let table_store = bloom_filter_enabled_table_store(10);
            let table = build_single_block_sst(&table_store, &[b"k1", b"k2"]).await;
            let options = SstIteratorOptions {
                filter_context,
                ..Default::default()
            };

            let iter = SstIterator::new_owned_initialized_with_stats(
                ..,
                table,
                table_store,
                options,
                None,
                Some(db_stats),
            )
            .await
            .unwrap();
            assert!(iter.is_some(), "nothing rejected the SST");
            assert_eq!(
                verdicts(&recorder, crate::db_stats::FILTER_KIND_RANGE),
                (Some(0), Some(0), Some(0)),
                "no evaluator should have been built"
            );
        }
    }

    #[tokio::test]
    async fn should_skip_sst_when_a_range_capable_policy_rejects() {
        let (recorder, db_stats) = stats();
        let root_path = Path::from("");
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 1,
            filter_policies: vec![Arc::new(RangeCapablePolicy)],
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
        let table = build_single_block_sst(&table_store, &[b"k1", b"k2"]).await;

        // No filter context: this policy reasons from the target alone.
        let iter = SstIterator::new_owned_initialized_with_stats(
            ..,
            table,
            table_store,
            SstIteratorOptions::default(),
            None,
            Some(db_stats),
        )
        .await
        .unwrap();

        assert!(
            iter.is_none(),
            "the policy rejects every range, so the SST is skipped"
        );
        let (positives, negatives, _) = verdicts(&recorder, crate::db_stats::FILTER_KIND_RANGE);
        assert_eq!((positives, negatives), (Some(0), Some(1)));
    }
}
