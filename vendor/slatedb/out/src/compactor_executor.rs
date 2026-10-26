use std::collections::BTreeMap;
use std::mem;
use std::ops::{Bound, RangeBounds};
use std::sync::Arc;
use std::sync::atomic::{self, AtomicBool};

use bytes::Bytes;
use futures::future::{join, join_all};
use parking_lot::Mutex;
use tokio::task::JoinHandle;
use tokio_util::task::AbortOnDropHandle;

use crate::bytes_range::BytesRange;
#[cfg(feature = "compaction_filters")]
use crate::compaction_filter::CompactionFilterSupplier;
#[cfg(feature = "compaction_filters")]
use crate::compaction_filter_iterator::CompactionFilterIterator;
use crate::compaction_worker::WorkerMessage;
use crate::compactor_state::CompactionContext;
use crate::config::CompactionWorkerOptions;
use crate::db_state::{SortedRun, SsTableHandle, SsTableId, SsTableView};
use crate::error::SlateDBError;
use crate::iter::{IterationOrder, RowEntryIterator, TrackedRowEntryIterator};
use crate::manifest::store::{ManifestStore, StoredManifest};
use crate::merge_iterator::MergeIterator;
use crate::merge_operator::{
    MergeOperatorIterator, MergeOperatorRequiredIterator, MergeOperatorType,
    instrument_merge_operator,
};
use crate::peeking_iterator::PeekingIterator;
use crate::retention_iterator::RetentionIterator;
use crate::seq_tracker::SequenceTracker;
use crate::sorted_run_iterator::SortedRunIterator;
use crate::sst_iter::{SstIterator, SstIteratorOptions};
use crate::subcompaction::{Subcompaction, plan_subcompaction_ranges};
use crate::tablestore::TableStore;
use slatedb_common::DbRand;
use slatedb_common::clock::SystemClock;

use crate::compactor::stats::{CompactionStats, WorkerStats};
use crate::utils::{
    IdGenerator, build_concurrent, compute_max_parallel, estimate_bytes_before_key,
    last_written_key_and_seq, spawn_bg_task,
};
use log::{debug, error};
use tracing::instrument;
use ulid::Ulid;

/// Arguments for starting a compaction job.
///
/// - `id` is the job id (ULID) and uniquely identifies a single job. This is
///   used as the runtime key in `scheduled_compactions`.
/// - `job_id` is the canonical plan id (ULID) that ties this job back to its
///   `Compaction`
///
/// Jobs carry fully materialized inputs (L0 `ssts` and `sorted_runs`) along with execution-time
/// metadata for progress reporting, retention, and resume logic.
#[derive(Clone, PartialEq)]
pub(crate) struct StartCompactionJobArgs {
    /// Job id. Unique per job.
    pub(crate) id: Ulid,
    /// Canonical compaction job id this job belongs to.
    pub(crate) compaction_id: Ulid,
    /// Segment containing all compaction inputs and outputs.
    pub(crate) segment: Bytes,
    /// Destination sorted run id to be produced by this job.
    pub(crate) destination: u32,
    /// Input L0 SSTs for this job.
    pub(crate) l0_sst_views: Vec<SsTableView>,
    /// Input existing sorted runs for this job.
    pub(crate) sorted_runs: Vec<SortedRun>,
    /// The clock tick representing the time the compaction occurs. This is used
    /// to make decisions about retention of expiring records.
    pub(crate) compaction_clock_tick: i64,
    /// Whether the destination sorted run is the last (newest) run after compaction.
    pub(crate) is_dest_last_run: bool,
    pub(crate) complete_input_through: Option<u64>,
    /// Optional minimum sequence to retain; lower sequences may be dropped by retention. This
    /// value is used only when planning the compaction. Once the compaction has started, if it is
    /// then resumed, then the executor uses retention_min_seq from ctx
    pub(crate) retention_min_seq: Option<u64>,
    /// Optional resume context to use for resuming a compaction job
    pub(crate) ctx: Option<CompactionContext>,
}

impl std::fmt::Debug for StartCompactionJobArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StartCompactionJobArgs")
            .field("id", &self.id)
            .field("job_id", &self.compaction_id)
            .field("segment", &self.segment)
            .field("destination", &self.destination)
            .field("ssts", &self.l0_sst_views)
            .field("sorted_runs", &self.sorted_runs)
            .field("compaction_clock_tick", &self.compaction_clock_tick)
            .field("is_dest_last_run", &self.is_dest_last_run)
            .field("ctx", &self.ctx)
            .finish()
    }
}

struct SubcompactionArgs {
    // index in to ctx.subcompactions()
    index: usize,
    segment: Bytes,
    range: BytesRange,
    // Only read when the `compaction_filters` feature builds the filter's
    // `CompactionJobContext`; otherwise the executor uses the job-level
    // destination passed to `execute_subcompactions`.
    #[cfg_attr(not(feature = "compaction_filters"), allow(dead_code))]
    destination: u32,
    l0_sst_views: Vec<SsTableView>,
    sorted_runs: Vec<SortedRun>,
    compaction_clock_tick: i64,
    is_dest_last_run: bool,
    complete_input_through: Option<u64>,
    retention_min_seq: Option<u64>,
    output_ssts: Vec<SsTableHandle>,
}

/// A compaction that has been planned and is ready to be executed
struct PlannedCompaction {
    // the context where state required to resume the job is recorded
    ctx: CompactionContext,
    // args for starting each subcompaction
    subcompaction_args: Vec<SubcompactionArgs>,
}

/// Iterator adapter that can resume after a persisted compaction output SST.
struct ResumingIterator<T: RowEntryIterator> {
    iterator: PeekingIterator<T>,
    start: Option<(Bytes, u64)>,
}

impl<T: RowEntryIterator> ResumingIterator<T> {
    /// Create a new resuming iterator that wraps the provided iterator. The iterator
    /// must be initialized prior to calling this method.
    ///
    /// This method first seeks to the specified `key` using the underlying iterator's
    /// `seek` method. It then enters a loop where it peeks at the next entry without
    /// advancing the iterator. If the peeked entry's key does not match the specified
    /// `key`, the loop breaks, as we have moved past the desired key. If the keys match,
    /// it checks the sequence number of the entry. Since entries for the same key are
    /// sorted in descending order by sequence, if the entry's sequence is less than the
    /// specified `seq`, the loop breaks, indicating we have found the position to resume.
    ///
    /// ## Arguments
    /// - `iterator`: the iterator to wrap.
    /// - `resume_cursor`: a (key, seq) tuple. when present, seeks to the key and drains
    ///   entries for that key until the iterator is positioned after the last written sequence.
    ///
    /// ## Returns
    /// - `ResumingIterator<T>`: the wrapper around `iterator`.
    /// - `SlateDBError::IteratorNotInitialized` if the iterator has not been initialized.
    /// - `SlateDBError`: any error returned by the underlying iterator.
    async fn new(iterator: T, resume_cursor: Option<(Bytes, u64)>) -> Result<Self, SlateDBError> {
        let mut resuming_iter = Self {
            iterator: PeekingIterator::new(iterator),
            start: resume_cursor.clone(),
        };
        if let Some((key, seq)) = resume_cursor {
            resuming_iter.iterator.seek(key.as_ref()).await?;
            loop {
                let Some(entry) = resuming_iter.iterator.peek().await? else {
                    break;
                };
                if entry.key.as_ref() != key.as_ref() {
                    break;
                }
                if entry.seq < seq {
                    break;
                }
                resuming_iter.iterator.next().await?;
            }
        }
        Ok(resuming_iter)
    }

    /// Returns the resume start cursor, if any. This is the (key, seq) tuple
    /// used to initialize the iterator.
    fn start(&self) -> Option<&(Bytes, u64)> {
        self.start.as_ref()
    }
}

#[async_trait::async_trait]
impl<T: RowEntryIterator> RowEntryIterator for ResumingIterator<T> {
    async fn init(&mut self) -> Result<(), SlateDBError> {
        self.iterator.init().await
    }

    async fn next(&mut self) -> Result<Option<crate::types::RowEntry>, SlateDBError> {
        self.iterator.next().await
    }

    async fn seek(&mut self, next_key: &[u8]) -> Result<(), SlateDBError> {
        self.iterator.seek(next_key).await
    }
}

impl<T: TrackedRowEntryIterator> TrackedRowEntryIterator for ResumingIterator<T> {
    fn bytes_processed(&self) -> u64 {
        self.iterator.bytes_processed()
    }
}

/// Executes compaction jobs produced by the compactor.
pub(crate) trait CompactionExecutor {
    /// Starts executing a compaction job asynchronously.
    fn start_compaction_job(&self, compaction: StartCompactionJobArgs);

    /// Requests that an executing compaction job stop.
    ///
    /// Returns true if the job was active and a stop was requested. This does
    /// not wait for the task to finish winding down.
    fn stop_compaction_job(&self, id: Ulid) -> bool;

    /// Stops the executor and requests cancellation of any in-flight tasks.
    fn stop(&self);
}

/// Options for creating a [`TokioCompactionExecutor`].
pub(crate) struct TokioCompactionExecutorOptions {
    pub handle: tokio::runtime::Handle,
    pub options: Arc<CompactionWorkerOptions>,
    pub worker_tx: async_channel::Sender<WorkerMessage>,
    pub table_store: Arc<TableStore>,
    pub rand: Arc<DbRand>,
    pub stats: Arc<CompactionStats>,
    pub worker_stats: WorkerStats,
    pub clock: Arc<dyn SystemClock>,
    pub manifest_store: Arc<ManifestStore>,
    pub merge_operator: Option<MergeOperatorType>,
    #[cfg(feature = "compaction_filters")]
    pub compaction_filter_supplier: Option<Arc<dyn CompactionFilterSupplier>>,
}

pub(crate) struct TokioCompactionExecutor {
    inner: Arc<TokioCompactionExecutorInner>,
}

impl TokioCompactionExecutor {
    pub(crate) fn new(opts: TokioCompactionExecutorOptions) -> Self {
        let stats = opts.stats;
        let merge_operator = opts.merge_operator.map(|merge_operator| {
            instrument_merge_operator(
                merge_operator,
                stats.merge_operator_compact_operands.clone(),
            )
        });

        Self {
            inner: Arc::new(TokioCompactionExecutorInner {
                options: opts.options,
                handle: opts.handle,
                worker_tx: opts.worker_tx,
                table_store: opts.table_store,
                rand: opts.rand,
                tasks: Arc::new(Mutex::new(BTreeMap::new())),
                stats,
                worker_stats: opts.worker_stats,
                clock: opts.clock,
                is_stopped: AtomicBool::new(false),
                manifest_store: opts.manifest_store,
                merge_operator,
                #[cfg(feature = "compaction_filters")]
                compaction_filter_supplier: opts.compaction_filter_supplier,
            }),
        }
    }
}

impl CompactionExecutor for TokioCompactionExecutor {
    fn start_compaction_job(&self, compaction: StartCompactionJobArgs) {
        self.inner.start_compaction_job(compaction);
    }

    fn stop_compaction_job(&self, id: Ulid) -> bool {
        self.inner.stop_compaction_job(id)
    }

    fn stop(&self) {
        self.inner.stop()
    }
}

struct TokioCompactionTask {
    destination: u32,
    task: JoinHandle<Result<SortedRun, SlateDBError>>,
}

/// Progress and completion events sent from subcompaction tasks to their
/// parent compaction job (RFC-0028). `index` identifies the subcompaction
/// within the parent's plan.
enum SubcompactionEvent {
    Progress {
        index: usize,
        bytes_processed: u64,
        output_ssts: Vec<SsTableHandle>,
    },
    Finished {
        index: usize,
        result: Result<Vec<SsTableHandle>, SlateDBError>,
    },
}

pub(crate) struct TokioCompactionExecutorInner {
    options: Arc<CompactionWorkerOptions>,
    handle: tokio::runtime::Handle,
    worker_tx: async_channel::Sender<WorkerMessage>,
    table_store: Arc<TableStore>,
    tasks: Arc<Mutex<BTreeMap<Ulid, TokioCompactionTask>>>,
    rand: Arc<DbRand>,
    stats: Arc<CompactionStats>,
    worker_stats: WorkerStats,
    clock: Arc<dyn SystemClock>,
    is_stopped: AtomicBool,
    manifest_store: Arc<ManifestStore>,
    merge_operator: Option<MergeOperatorType>,
    #[cfg(feature = "compaction_filters")]
    compaction_filter_supplier: Option<Arc<dyn CompactionFilterSupplier>>,
}

impl TokioCompactionExecutorInner {
    /// Builds input iterators for all sources (L0 and SR) restricted to `range` and
    /// wraps them with optional merge, retention, and compaction filter logic.
    ///
    /// `output_ssts` are the output SSTs already written by a previous attempt of
    /// this (sub)compaction; the returned iterator is positioned just past the last
    /// entry they contain. `sequence_tracker` is the manifest's sequence tracker,
    /// loaded once per job so concurrent subcompactions don't each re-read the
    /// manifest.
    async fn load_iterators<'a>(
        &self,
        job_args: &'a SubcompactionArgs,
        sequence_tracker: Arc<SequenceTracker>,
    ) -> Result<ResumingIterator<Box<dyn TrackedRowEntryIterator + 'a>>, SlateDBError> {
        let retention_min_seq = job_args.retention_min_seq;
        let resume_cursor = match job_args.output_ssts.last() {
            Some(output_sst) => {
                last_written_key_and_seq(self.table_store.clone(), output_sst, &job_args.segment)
                    .await?
            }
            None => None,
        };
        let sst_iter_options = SstIteratorOptions {
            max_fetch_tasks: self.options.max_fetch_tasks,
            target_bytes_to_fetch: self.options.bytes_to_fetch,
            cache_blocks: false, // don't clobber the cache
            cache_metadata: false,
            eager_spawn: true,
            order: IterationOrder::Ascending,
            prefix: None,
            filter_context: None,
            segment: Some(job_args.segment.clone()),
        };

        let max_parallel =
            compute_max_parallel(job_args.l0_sst_views.len(), &job_args.sorted_runs, 4);
        // L0 (borrowed)
        let l0_iters_futures = build_concurrent(job_args.l0_sst_views.iter(), max_parallel, |h| {
            let sst_iter_options = sst_iter_options.clone();
            SstIterator::new_borrowed_initialized(
                job_args.range.clone(),
                h,
                self.table_store.clone(),
                sst_iter_options,
            )
        });

        // SR (borrowed)
        let slice_range: (Bound<&[u8]>, Bound<&[u8]>) = (
            job_args.range.start_bound().map(|b| b.as_ref()),
            job_args.range.end_bound().map(|b| b.as_ref()),
        );
        let sr_iters_futures = build_concurrent(job_args.sorted_runs.iter(), max_parallel, |sr| {
            let sst_iter_options = sst_iter_options.clone();
            async move {
                SortedRunIterator::new_borrowed(
                    slice_range,
                    sr,
                    self.table_store.clone(),
                    sst_iter_options,
                )
                .await
                .map(Some)
            }
        });

        let (l0_iters_res, sr_iters_res) = join(l0_iters_futures, sr_iters_futures).await;
        let l0_iters = l0_iters_res?;
        let sr_iters = sr_iters_res?;

        let l0_merge_iter = MergeIterator::new(l0_iters)?.with_dedup(false);
        let sr_merge_iter = MergeIterator::new(sr_iters)?.with_dedup(false);

        let merge_iter = MergeIterator::new([l0_merge_iter, sr_merge_iter])?.with_dedup(false);
        let merge_iter: Box<dyn TrackedRowEntryIterator> =
            if let Some(merge_operator) = self.merge_operator.clone() {
                Box::new(MergeOperatorIterator::new(
                    merge_operator,
                    merge_iter,
                    false,
                    retention_min_seq,
                ))
            } else {
                Box::new(MergeOperatorRequiredIterator::new(merge_iter))
            };

        let mut retention_iter = RetentionIterator::new(
            merge_iter,
            None,
            retention_min_seq,
            job_args.is_dest_last_run,
            job_args.compaction_clock_tick,
            self.clock.clone(),
            sequence_tracker,
            Some(self.stats.retention_metrics()),
        )
        .await?;
        retention_iter.init().await?;

        // Apply compaction filter if configured
        #[cfg(feature = "compaction_filters")]
        if let Some(supplier) = &self.compaction_filter_supplier {
            use crate::compaction_filter::CompactionJobContext;
            let context = CompactionJobContext {
                destination: job_args.destination,
                is_dest_last_run: job_args.is_dest_last_run,
                complete_input_through: job_args.complete_input_through,
                compaction_clock_tick: job_args.compaction_clock_tick,
                retention_min_seq,
            };
            let filter = supplier.create_compaction_filter(&context).await?;
            let filter_iter = CompactionFilterIterator::new(retention_iter, filter)
                .with_full_input(supplier.requires_full_input());
            let boxed: Box<dyn TrackedRowEntryIterator> = Box::new(filter_iter);
            let resuming_iter = ResumingIterator::new(boxed, resume_cursor).await?;
            return Ok(resuming_iter);
        }

        let boxed: Box<dyn TrackedRowEntryIterator> = Box::new(retention_iter);
        let resuming_iter = ResumingIterator::new(boxed, resume_cursor).await?;
        Ok(resuming_iter)
    }

    fn send_compaction_progress(&self, id: Ulid, bytes_processed: u64, ctx: CompactionContext) {
        // Allow send() because we are treating the executor like an external
        // component. They can do what they want. If the send fails (e.g., during
        // DB shutdown), we log it and continue with the compaction work.
        #[allow(clippy::disallowed_methods)]
        if let Err(e) = self
            .worker_tx
            .try_send(WorkerMessage::CompactionJobProgress {
                id,
                bytes_processed,
                ctx,
            }) {
            debug!(
                "failed to send compaction progress (likely DB shutdown) [error={:?}]",
                e
            );
        }
    }

    /// Awaits an in-flight SST close, records its stats, and appends the
    /// resulting handle to `output_ssts`. Handles are appended in spawn order,
    /// which preserves the ascending-key ordering required within a sorted run.
    async fn collect_close(
        &self,
        pending: AbortOnDropHandle<Result<SsTableHandle, SlateDBError>>,
        output_ssts: &mut Vec<SsTableHandle>,
    ) -> Result<(), SlateDBError> {
        let sst = pending.await.map_err(|e| {
            let name = "compactor_sst_close".to_string();
            if e.is_cancelled() {
                SlateDBError::BackgroundTaskCancelled(name)
            } else {
                SlateDBError::BackgroundTaskPanic(name)
            }
        })??;
        self.worker_stats
            .bytes_compacted
            .increment(sst.info.filter_offset);
        self.worker_stats.ssts_written.increment(1);
        output_ssts.push(sst);
        Ok(())
    }

    async fn plan_compaction_job(
        &self,
        args: StartCompactionJobArgs,
    ) -> Result<PlannedCompaction, SlateDBError> {
        let ctx = match args.ctx {
            Some(ctx) => ctx,
            None => CompactionContext::new(
                self.plan_subcompactions(&args).await?,
                args.retention_min_seq,
            ),
        };
        #[cfg(feature = "compaction_filters")]
        if self
            .compaction_filter_supplier
            .as_ref()
            .is_some_and(|s| s.requires_full_input())
            && ctx.subcompactions().len() != 1
        {
            return Err(SlateDBError::InvalidCompaction);
        }
        assert!(!ctx.subcompactions().is_empty());
        let subcompaction_args = ctx
            .subcompactions()
            .iter()
            .enumerate()
            .map(|(index, s)| SubcompactionArgs {
                index,
                segment: args.segment.clone(),
                range: s.range().clone(),
                destination: args.destination,
                l0_sst_views: args.l0_sst_views.clone(),
                sorted_runs: args.sorted_runs.clone(),
                compaction_clock_tick: args.compaction_clock_tick,
                is_dest_last_run: args.is_dest_last_run,
                complete_input_through: args.complete_input_through,
                retention_min_seq: ctx.retention_min_seq(),
                output_ssts: s.output_ssts().clone(),
            })
            .collect::<Vec<_>>();
        Ok(PlannedCompaction {
            ctx,
            subcompaction_args,
        })
    }

    /// Executes a single compaction job and returns the resulting context.
    ///
    /// ## Steps
    /// - Plans the job's subcompactions and runs the ranges concurrently
    ///   (RFC-0028)
    /// - Streams and merges input keys across all sources (per range)
    /// - Applies merge and retention policies
    /// - Writes output SSTs up to `max_sst_size`, reporting resumable progress
    ///
    /// ## Returns
    /// - The compaction context with all output SST handles.
    #[instrument(level = "debug", skip_all, fields(id = %args.id))]
    async fn plan_and_execute_compaction_job(
        self: &Arc<Self>,
        args: StartCompactionJobArgs,
    ) -> Result<SortedRun, SlateDBError> {
        debug!("executing compaction [job_args={:?}]", args);
        let id = args.id;
        let destination = args.destination;

        // Worker liveness is refreshed by the compaction worker's own heartbeat
        // ticker while planning is in flight. The executor only reports
        // progress when it has compaction context that may need to be persisted
        // for resume.
        let (sequence_tracker, planned) = self.load_manifest_and_plan(args).await?;

        self.execute_compaction_job(id, destination, planned, sequence_tracker)
            .await
    }

    /// Loads the manifest's shared sequence tracker and plans the job's ranges
    /// (RFC-0028). Every range shares the one sequence tracker rather than
    /// re-reading the manifest.
    async fn load_manifest_and_plan(
        &self,
        args: StartCompactionJobArgs,
    ) -> Result<(Arc<SequenceTracker>, PlannedCompaction), SlateDBError> {
        let stored_manifest =
            StoredManifest::load(self.manifest_store.clone(), self.clock.clone()).await?;
        let sequence_tracker = Arc::new(stored_manifest.db_state().sequence_tracker.clone());
        let planned = self.plan_compaction_job(args).await?;
        Ok((sequence_tracker, planned))
    }

    /// Plans the subcompaction ranges for a fresh job (RFC-0028).
    ///
    /// Only called when the job carries no resume context: a job that is
    /// resuming reuses its persisted [`CompactionContext`] verbatim (see
    /// [`Self::plan_compaction_job`]). Reads the input SST indexes to choose
    /// split points (see [`plan_subcompaction_ranges`]), so this is fallible.
    ///
    /// ## Returns
    /// - The planned ranges or a single unbounded range when
    ///   subcompactions are disabled
    async fn plan_subcompactions(
        &self,
        args: &StartCompactionJobArgs,
    ) -> Result<Vec<Subcompaction>, SlateDBError> {
        let max_subcompactions = self.options.max_subcompactions;
        #[cfg(feature = "compaction_filters")]
        let max_subcompactions = if self
            .compaction_filter_supplier
            .as_ref()
            .is_some_and(|s| s.requires_full_input())
        {
            1
        } else {
            max_subcompactions
        };
        let ranges = plan_subcompaction_ranges(
            &self.table_store,
            &args.l0_sst_views,
            &args.sorted_runs,
            &args.segment,
            max_subcompactions,
            self.options.max_fetch_tasks,
        )
        .await?;
        Ok(ranges.into_iter().map(Subcompaction::new).collect())
    }

    /// Executes a compaction job's subcompactions (RFC-0028), running every
    /// range concurrently.
    ///
    /// The plan is reported (and buffered by the worker for its next
    /// heartbeat to publish) before any range output is recorded against it;
    /// per-range output SSTs are then reported as they advance so an
    /// interrupted compaction can resume at range granularity. Subcompactions
    /// carry no persisted status, so every range is run: a range resumes from
    /// the output recorded against it, and one that is already complete finds
    /// nothing left to merge and finishes immediately. On the first range
    /// failure the remaining subcompactions are aborted and the job fails;
    /// progress recorded by then stays resumable.
    async fn execute_compaction_job(
        self: &Arc<Self>,
        id: Ulid,
        destination: u32,
        planned: PlannedCompaction,
        sequence_tracker: Arc<SequenceTracker>,
    ) -> Result<SortedRun, SlateDBError> {
        let PlannedCompaction {
            mut ctx,
            subcompaction_args,
        } = planned;
        let num_subcompactions = subcompaction_args.len();
        debug!(
            "executing compaction with subcompactions [id={}, subcompactions={}]",
            id, num_subcompactions
        );

        // Per-range bytes-processed, summed into the job's progress total. Each
        // range folds its own resume offset into the values it reports (see
        // `start_bytes_processed` in `run_subcompaction_merge`), so a range's
        // latest event is its full cumulative contribution across attempts —
        // including a range that resumes already-complete, which reports its
        // full estimated size up front.
        let mut bytes_processed_by_sub: Vec<u64> = vec![0; subcompaction_args.len()];

        // For a fresh job, report the plan immediately so the worker buffers
        // it before any range records output against it; bytes are genuinely
        // zero at this point. A resumed job's plan is already persisted (it
        // arrived in `args.subcompactions`), so reporting it here would only
        // republish zero bytes processed before the ranges re-report their
        // resumed progress — a momentary drop to zero. Skip it; the per-range
        // progress emitted as each range starts carries the resumed totals
        // instead.
        if ctx
            .subcompactions()
            .iter()
            .map(|s| s.output_ssts().len())
            .sum::<usize>()
            == 0
        {
            self.send_compaction_progress(id, 0, ctx.clone());
        }

        let (sub_tx, mut sub_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut sub_tasks = Vec::new();
        for args in subcompaction_args {
            // Each range runs as its own task, resuming from the output already
            // recorded against it. A range whose output is already complete
            // finds nothing left to merge and finishes immediately.
            let this = self.clone();
            let index = args.index;
            let sequence_tracker = sequence_tracker.clone();
            let event_tx = sub_tx.clone();
            let finished_tx = sub_tx.clone();
            let task = spawn_bg_task(
                format!("subcompaction:{}:{}", id, index),
                &self.handle,
                move |result: &Result<Vec<SsTableHandle>, SlateDBError>| {
                    // Allow send() because this channel is internal to the
                    // parent job: the receiver is dropped only after the
                    // parent has aborted (or collected) every subcompaction
                    // task, so a failed send needs no handling.
                    #[allow(clippy::disallowed_methods)]
                    let _ = finished_tx.send(SubcompactionEvent::Finished {
                        index,
                        result: result.clone(),
                    });
                },
                async move {
                    let progress = move |bytes_processed: u64, output_ssts: &[SsTableHandle]| {
                        // Allow send() for the same reason as the cleanup fn
                        // above: a dropped receiver means the parent job is
                        // aborting this task anyway.
                        #[allow(clippy::disallowed_methods)]
                        let _ = event_tx.send(SubcompactionEvent::Progress {
                            index,
                            bytes_processed,
                            output_ssts: output_ssts.to_vec(),
                        });
                    };
                    this.run_subcompaction_merge(args, sequence_tracker, &progress)
                        .await
                },
            );
            // Wrapping each task in an `AbortOnDropHandle` is what propagates a
            // parent abort down to the children: if the parent compaction future
            // is dropped — because the executor's `stop()` aborted it, or this
            // fn returns early on a range error — `sub_tasks` drops with it and
            // every still-running subcompaction is aborted at its await point.
            // This is the mechanism that lets `stop()` tear down in-flight
            // ranges rather than just the parent job.
            sub_tasks.push(AbortOnDropHandle::new(task));
        }
        drop(sub_tx);

        let mut completed = vec![false; num_subcompactions];
        let mut first_error: Option<SlateDBError> = None;
        while !completed.iter().all(|done| *done) {
            let Some(event) = sub_rx.recv().await else {
                break;
            };
            match event {
                SubcompactionEvent::Progress {
                    index,
                    bytes_processed,
                    output_ssts,
                } => {
                    // Each range reports monotonically increasing bytes (its
                    // resume offset is already folded in), so store the latest.
                    bytes_processed_by_sub[index] = bytes_processed;
                    ctx.set_output_ssts(index, output_ssts);
                    let total_bytes = bytes_processed_by_sub.iter().sum();
                    self.send_compaction_progress(id, total_bytes, ctx.clone());
                }
                SubcompactionEvent::Finished { index, result } => match result {
                    Ok(output_ssts) => {
                        ctx.set_output_ssts(index, output_ssts);
                        completed[index] = true;
                        let total_bytes = bytes_processed_by_sub.iter().sum();
                        self.send_compaction_progress(id, total_bytes, ctx.clone());
                    }
                    Err(e) => {
                        first_error = Some(e);
                        break;
                    }
                },
            }
        }
        // Abort any subcompactions still running after a failure, and wait
        // for them to wind down so the job stops writing to the object store
        // before it is reported failed. On success every task has already
        // completed, so this returns immediately.
        sub_tasks.iter().for_each(|task| task.abort());
        let _ = join_all(sub_tasks).await;

        if let Some(e) = first_error {
            return Err(e);
        }
        if !completed.iter().all(|done| *done) {
            // The event channel closed before every subcompaction reported a
            // successful result; treat the job as failed so it can be
            // rescheduled.
            return Err(SlateDBError::CompactorExecutorFailed);
        }

        // A sorted run requires its SSTs to be in ascending, non-overlapping
        // key order. Concatenating the ranges' output in plan order satisfies
        // that only when the ranges themselves are ascending and disjoint.
        // Verify it before committing the run rather than silently producing a
        // corrupt one: adjacent output SSTs must not overlap by boundary key.
        // The check is cheap because the boundary keys are already in memory.
        let output_ssts: Vec<&SsTableHandle> = ctx
            .subcompactions()
            .iter()
            .flat_map(|sub| sub.output_ssts().iter())
            .collect();
        let ascending = output_ssts.windows(2).all(|pair| {
            match (&pair[0].info.last_entry, &pair[1].info.first_entry) {
                (Some(prev_last), Some(next_first)) => prev_last <= next_first,
                _ => true,
            }
        });
        if !ascending {
            error!(
                "subcompaction output SSTs are not in ascending, non-overlapping key order \
                 [id={}]",
                id
            );
            return Err(SlateDBError::CompactorExecutorFailed);
        }
        Ok(SortedRun::new(
            destination,
            output_ssts.into_iter().map(|sst| {
                let id = self.rand.rng().gen_ulid(self.clock.as_ref());
                SsTableView::new(id, sst.clone())
            }),
        ))
    }

    /// Runs the merge for one key range of a compaction job and returns the
    /// output SSTs, including any `initial_output_ssts` written by a previous
    /// attempt. This is the shared execution path for whole compactions (an
    /// unbounded range) and subcompactions (RFC-0028).
    async fn run_subcompaction_merge(
        &self,
        args: SubcompactionArgs,
        sequence_tracker: Arc<SequenceTracker>,
        progress: &(dyn Fn(u64, &[SsTableHandle]) + Send + Sync),
    ) -> Result<Vec<SsTableHandle>, SlateDBError> {
        let mut all_iter = self.load_iterators(&args, sequence_tracker).await?;
        let mut output_ssts = args.output_ssts.clone();
        let mut current_writer = self.table_store.table_writer(
            SsTableId::from(self.rand.rng().gen_ulid(self.clock.as_ref())),
            Some(args.segment.clone()),
        );
        let mut bytes_written = 0usize;
        // Estimate bytes processed within this range before the resume point,
        // if any. For an unbounded range this is the estimate of everything
        // before the resume key; for a subcompaction range the bytes before
        // the range's start key are excluded since they belong to sibling
        // ranges.
        let start_bytes_processed = all_iter.start().map_or(0, |(k, _s)| {
            let before_key = estimate_bytes_before_key(args.sorted_runs.as_slice(), k);
            let before_range = match args.range.start_bound() {
                Bound::Included(s) | Bound::Excluded(s) => {
                    estimate_bytes_before_key(args.sorted_runs.as_slice(), s)
                }
                Bound::Unbounded => 0,
            };
            before_key.saturating_sub(before_range)
        });

        // Report the resume point up front so a resumed range surfaces the bytes
        // already processed in prior attempts immediately. A range that resumes
        // already-complete reports its full estimated size here (its resume
        // cursor sits at the range's last key) and never enters the loop below.
        progress(start_bytes_processed, &output_ssts);

        // At most one SST close runs in the background at a time (depth-1
        // pipeline). While a finished SST flushes to the object store we keep
        // merging input and filling the next SST, instead of stalling the loop
        // on close(). Only one buffered SST is held outstanding, which bounds
        // memory and applies backpressure when the object store is slow.
        let mut pending_close: Option<AbortOnDropHandle<Result<SsTableHandle, SlateDBError>>> =
            None;

        while let Some(kv) = all_iter.next().await? {
            // Opportunistically collect a finished background close without
            // stalling the merge loop to promptly lets us report progress
            // and persist the newly uploaded output SST in compactor state
            // as soon as the close finishes, rather than waiting until the
            // next SST fills up.
            if let Some(pending) = pending_close.take_if(|p| p.is_finished()) {
                self.collect_close(pending, &mut output_ssts).await?;
                let total_bytes = start_bytes_processed + all_iter.bytes_processed();
                progress(total_bytes, &output_ssts);
            }

            if let Some(block_size) = current_writer.add(kv).await? {
                bytes_written += block_size;
            }

            if bytes_written > self.options.max_sst_size {
                // Depth-1 backpressure: drain the previous close (collecting its
                // handle in order) before starting another.
                if let Some(pending) = pending_close.take() {
                    self.collect_close(pending, &mut output_ssts).await?;
                }
                let finished_writer = mem::replace(
                    &mut current_writer,
                    self.table_store.table_writer(
                        SsTableId::from(self.rand.rng().gen_ulid(self.clock.as_ref())),
                        Some(args.segment.clone()),
                    ),
                );
                pending_close = Some(AbortOnDropHandle::new(spawn_bg_task(
                    format!("compactor_sst_close:{:?}", finished_writer.id()),
                    &self.handle,
                    |_| {},
                    async move { finished_writer.close().await.map(|(sst, _)| sst) },
                )));
                bytes_written = 0;
                let total_bytes = start_bytes_processed + all_iter.bytes_processed();
                progress(total_bytes, &output_ssts);
            }

            // Keep cached compaction work cooperative.
            tokio::task::coop::consume_budget().await;
        }

        // Drain the in-flight close, then flush the final partial SST. Order
        // matters: the pending SST's keys all precede the final writer's keys.
        if let Some(pending) = pending_close.take() {
            self.collect_close(pending, &mut output_ssts).await?;
        }
        if !current_writer.is_drained() {
            let (sst, _) = current_writer.close().await?;

            self.worker_stats
                .bytes_compacted
                .increment(sst.info.filter_offset);
            self.worker_stats.ssts_written.increment(1);
            output_ssts.push(sst);
        }

        Ok(output_ssts)
    }

    /// Starts a background task to run the compaction job.
    fn start_compaction_job(self: &Arc<Self>, args: StartCompactionJobArgs) {
        let mut tasks = self.tasks.lock();
        if self.is_stopped.load(atomic::Ordering::SeqCst) {
            return;
        }
        let id = args.id;
        let dst = args.destination;
        assert!(!tasks.contains_key(&id));
        assert!(!tasks.values().any(|task| task.destination == dst));
        self.worker_stats.running_compactions.increment(1);

        let this = self.clone();
        let this_cleanup = self.clone();
        let task = spawn_bg_task(
            "compactor_executor".to_string(),
            &self.handle,
            move |result| {
                let removed = {
                    let mut tasks = this_cleanup.tasks.lock();
                    tasks.remove(&id).is_some()
                };

                // If the task was already removed (e.g., by `stop_compaction_job`), don't
                // send a completion message or update metrics.
                if !removed {
                    return;
                }

                let result = result.clone();
                // Allow send() because we are treating the executor like an external
                // component. They can do what they want. If the send fails (e.g., during
                // DB shutdown), we log it and continue with cleanup.
                #[allow(clippy::disallowed_methods)]
                if let Err(e) = this_cleanup
                    .worker_tx
                    .try_send(WorkerMessage::CompactionJobFinished { id, result })
                {
                    debug!(
                        "failed to send compaction finished msg (likely DB shutdown) [error={:?}]",
                        e
                    );
                }
                this_cleanup.worker_stats.running_compactions.increment(-1);
            },
            async move { this.plan_and_execute_compaction_job(args).await },
        );
        tasks.insert(
            id,
            TokioCompactionTask {
                destination: dst,
                task,
            },
        );
    }

    /// Requests cancellation of one active compaction job.
    #[allow(dead_code)]
    fn stop_compaction_job(&self, id: Ulid) -> bool {
        let task = {
            let mut tasks = self.tasks.lock();
            tasks.remove(&id)
        };
        let Some(task) = task else {
            return false;
        };

        task.task.abort();
        self.worker_stats.running_compactions.increment(-1);
        self.spawn_task_termination_logger(vec![(id, task.task)]);
        true
    }

    fn spawn_task_termination_logger(
        &self,
        task_handles: Vec<(Ulid, JoinHandle<Result<SortedRun, SlateDBError>>)>,
    ) {
        if task_handles.is_empty() {
            return;
        }
        let wait_for_task_termination = async move {
            let results = join_all(
                task_handles
                    .into_iter()
                    .map(|(id, task)| async move { (id, task.await) }),
            )
            .await;
            for (id, result) in results {
                match result {
                    Err(e) if !e.is_cancelled() => {
                        error!(
                            "compaction job failed to stop cleanly [id={}, error={:?}]",
                            id, e
                        );
                    }
                    _ => {}
                }
            }
        };
        self.handle.spawn(wait_for_task_termination);
    }

    /// Requests cancellation of all active compaction tasks.
    fn stop(&self) {
        // Drain all tasks and abort them, then release tasks lock. Draining
        // takes ownership of cancellation accounting, so task cleanup skips
        // completion messages and metric updates for these jobs.
        let task_handles = {
            let mut tasks = self.tasks.lock();
            // BTreeMap keeps abort/join order stable by job id. HashMap
            // iteration order is randomized, which can feed the DST runtime a
            // process-dependent shutdown order.
            let mut task_handles = Vec::with_capacity(tasks.len());
            while let Some((id, task)) = tasks.pop_first() {
                task.task.abort();
                self.worker_stats.running_compactions.increment(-1);
                task_handles.push((id, task.task));
            }
            task_handles
        };

        self.spawn_task_termination_logger(task_handles);
        self.is_stopped.store(true, atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;
    use crate::block_cache_policy::BlockCachePolicy;
    use crate::bytes_range::BytesRange;
    use crate::format::sst::SsTableFormat;
    use crate::manifest::ManifestCore;
    use crate::proptest_util::arbitrary;
    use crate::sst_iter::SstView;
    use crate::tablestore::TableStoreKind;
    use crate::test_utils::StringConcatMergeOperator;
    use crate::test_utils::{GatedObjectStore, build_row_entries, build_sorted_runs, write_ssts};
    use crate::types::{RowEntry, ValueDeletable};
    use bytes::Bytes;
    use object_store::ObjectStore;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use proptest::prelude::Just;
    use proptest::strategy::Strategy;
    use proptest::test_runner::Config;
    use proptest::{prop_assume, prop_oneof, proptest};
    use rstest::rstest;
    use slatedb_common::clock::DefaultSystemClock;
    use std::cmp::Ordering;
    use std::collections::HashSet;
    use std::time::Duration;

    async fn write_sst(
        table_store: &Arc<TableStore>,
        entries: &[RowEntry],
        max_sst_size: usize,
    ) -> Vec<SsTableHandle> {
        if entries.is_empty() {
            return Vec::new();
        }

        // Write entries into one or more SSTs, splitting on the same block-size
        // accounting used by the compactor's output writer.
        let mut output_ssts = Vec::new();
        let mut writer = table_store.table_writer(SsTableId::from(Ulid::new()), Some(Bytes::new()));
        let mut bytes_written = 0usize;

        for (index, entry) in entries.iter().cloned().enumerate() {
            if let Some(block_size) = writer.add(entry).await.unwrap() {
                bytes_written += block_size;
            }

            if bytes_written > max_sst_size {
                output_ssts.push(writer.close().await.unwrap().0);
                bytes_written = 0;

                if index + 1 < entries.len() {
                    writer =
                        table_store.table_writer(SsTableId::from(Ulid::new()), Some(Bytes::new()));
                } else {
                    return output_ssts;
                }
            }
        }

        output_ssts.push(writer.close().await.unwrap().0);
        output_ssts
    }

    fn has_duplicate_key_seq_specs(
        l0_specs: &[Vec<(Bytes, u64, ValueDeletable)>],
        sr_specs: &[Vec<(Bytes, u64, ValueDeletable)>],
    ) -> bool {
        let mut seen = HashSet::new();
        l0_specs
            .iter()
            .chain(sr_specs.iter())
            .flatten()
            .any(|(key, seq, _value)| !seen.insert((key.clone(), *seq)))
    }

    #[rstest]
    #[case::l0_only(
        vec![
            vec![ // l0(0)
                RowEntry::new_value(b"k00", b"k00-100", 100),
                RowEntry::new_value(b"k01", b"k01-101", 101),
                RowEntry::new_value(b"k02", b"k02-102", 102),
                RowEntry::new_value(b"k03", b"k03-30", 30),
                RowEntry::new_value(b"k03", b"k03-20", 20),
                RowEntry::new_value(b"k04", b"k04-104", 104),
            ],
            vec![ // l0(1)
                RowEntry::new_value(b"k05", b"k05-105", 105),
                RowEntry::new_value(b"k06", b"k06-106", 106),
                RowEntry::new_value(b"k07", b"k07-107", 107),
                RowEntry::new_value(b"k08", b"k08-108", 108),
                RowEntry::new_value(b"k09", b"k09-109", 109),
                RowEntry::new_value(b"k10", b"k10-110", 110),
            ],
        ],
        Vec::new(),
        4096,
        67_108_864,
        Some(3),
        None,
        vec![
            RowEntry::new_value(b"k03", b"k03-20", 20),
            RowEntry::new_value(b"k04", b"k04-104", 104),
            RowEntry::new_value(b"k05", b"k05-105", 105),
            RowEntry::new_value(b"k06", b"k06-106", 106),
            RowEntry::new_value(b"k07", b"k07-107", 107),
            RowEntry::new_value(b"k08", b"k08-108", 108),
            RowEntry::new_value(b"k09", b"k09-109", 109),
            RowEntry::new_value(b"k10", b"k10-110", 110),
        ]
    )]
    #[case::l0_only_multi_block(
        vec![
            vec![ // l0(0)
                RowEntry::new_value(b"k00", b"k00-100", 100),
                RowEntry::new_value(b"k01", b"k01-101", 101),
                RowEntry::new_value(b"k02", b"k02-102", 102),
                RowEntry::new_value(b"k03", b"k03-103", 103),
                RowEntry::new_value(b"k04", b"k04-104", 104),
                RowEntry::new_value(b"k05", b"k05-105", 105),
                RowEntry::new_value(b"k06", b"k06-106", 106),
                RowEntry::new_value(b"k07", b"k07-107", 107),
                RowEntry::new_value(b"k08", b"k08-80", 80),
                RowEntry::new_value(b"k08", b"k08-70", 70),
                RowEntry::new_value(b"k09", b"k09-109", 109),
                RowEntry::new_value(b"k10", b"k10-110", 110),
                RowEntry::new_value(b"k11", b"k11-111", 111),
                RowEntry::new_value(b"k12", b"k12-112", 112),
            ],
        ],
        Vec::new(),
        128,
        1,
        Some(9),
        None,
        vec![
            RowEntry::new_value(b"k09", b"k09-109", 109),
            RowEntry::new_value(b"k10", b"k10-110", 110),
            RowEntry::new_value(b"k11", b"k11-111", 111),
            RowEntry::new_value(b"k12", b"k12-112", 112),
        ]
    )]
    #[case::sr_only(
        Vec::new(),
        vec![
            vec![ // sr(0)
                vec![ // sr(0) sst(0)
                    RowEntry::new_value(b"k00", b"k00-300", 300),
                    RowEntry::new_value(b"k01", b"k01-301", 301),
                    RowEntry::new_value(b"k02", b"k02-302", 302),
                    RowEntry::new_value(b"k03", b"k03-303", 303),
                    RowEntry::new_value(b"k03", b"k03-33", 33),
                    RowEntry::new_value(b"k04", b"k04-304", 304),
                    RowEntry::new_value(b"k05", b"k05-305", 305),
                ],
            ],
            vec![ // sr(1)
                vec![  // sr(1) sst(0)
                    RowEntry::new_value(b"k06", b"k06-406", 406),
                    RowEntry::new_value(b"k07", b"k07-407", 407),
                    RowEntry::new_value(b"k08", b"k08-408", 408),
                    RowEntry::new_value(b"k09", b"k09-409", 409),
                    RowEntry::new_value(b"k10", b"k10-410", 410),
                    RowEntry::new_value(b"k11", b"k11-411", 411),
                ],
            ],
        ],
        4096,
        67_108_864,
        Some(4),
        None,
        vec![
            RowEntry::new_value(b"k04", b"k04-304", 304),
            RowEntry::new_value(b"k05", b"k05-305", 305),
            RowEntry::new_value(b"k06", b"k06-406", 406),
            RowEntry::new_value(b"k07", b"k07-407", 407),
            RowEntry::new_value(b"k08", b"k08-408", 408),
            RowEntry::new_value(b"k09", b"k09-409", 409),
            RowEntry::new_value(b"k10", b"k10-410", 410),
            RowEntry::new_value(b"k11", b"k11-411", 411),
        ]
    )]
    #[case::l0_and_sr(
        vec![
            vec![ // l0(0)
                RowEntry::new_value(b"k00", b"k00-500", 500),
                RowEntry::new_value(b"k01", b"k01-501", 501),
                RowEntry::new_value(b"k02", b"k02-502", 502),
                RowEntry::new_value(b"k03", b"k03-503", 503),
                RowEntry::new_value(b"k04", b"k04-504", 504),
                RowEntry::new_value(b"k05", b"k05-90", 90),
                RowEntry::new_value(b"k05", b"k05-80", 80),
            ],
            vec![ // l0(1)
                RowEntry::new_value(b"k06", b"k06-606", 606),
                RowEntry::new_value(b"k07", b"k07-607", 607),
                RowEntry::new_value(b"k08", b"k08-608", 608),
                RowEntry::new_value(b"k09", b"k09-609", 609),
                RowEntry::new_value(b"k10", b"k10-610", 610),
                RowEntry::new_value(b"k11", b"k11-611", 611),
            ],
        ],
        vec![
            vec![ // sr(0)
                vec![ // sr(0) sst(0)
                    RowEntry::new_value(b"k12", b"k12-720", 720),
                    RowEntry::new_value(b"k13", b"k13-721", 721),
                    RowEntry::new_value(b"k14", b"k14-722", 722),
                    RowEntry::new_value(b"k15", b"k15-120", 120),
                    RowEntry::new_value(b"k15", b"k15-110", 110),
                    RowEntry::new_value(b"k16", b"k16-726", 726),
                    RowEntry::new_value(b"k17", b"k17-727", 727),
                ],
            ],
            vec![ // sr(1)
                vec![ // sr(1) sst(0)
                    RowEntry::new_value(b"k18", b"k18-830", 830),
                    RowEntry::new_value(b"k19", b"k19-831", 831),
                    RowEntry::new_value(b"k20", b"k20-832", 832),
                    RowEntry::new_value(b"k21", b"k21-833", 833),
                    RowEntry::new_value(b"k22", b"k22-834", 834),
                    RowEntry::new_value(b"k23", b"k23-835", 835),
                ],
            ],
        ],
        4096,
        67_108_864,
        Some(5),
        None,
        vec![
            RowEntry::new_value(b"k05", b"k05-80", 80),
            RowEntry::new_value(b"k06", b"k06-606", 606),
            RowEntry::new_value(b"k07", b"k07-607", 607),
            RowEntry::new_value(b"k08", b"k08-608", 608),
            RowEntry::new_value(b"k09", b"k09-609", 609),
            RowEntry::new_value(b"k10", b"k10-610", 610),
            RowEntry::new_value(b"k11", b"k11-611", 611),
            RowEntry::new_value(b"k12", b"k12-720", 720),
            RowEntry::new_value(b"k13", b"k13-721", 721),
            RowEntry::new_value(b"k14", b"k14-722", 722),
            RowEntry::new_value(b"k15", b"k15-120", 120),
            RowEntry::new_value(b"k15", b"k15-110", 110),
            RowEntry::new_value(b"k16", b"k16-726", 726),
            RowEntry::new_value(b"k17", b"k17-727", 727),
            RowEntry::new_value(b"k18", b"k18-830", 830),
            RowEntry::new_value(b"k19", b"k19-831", 831),
            RowEntry::new_value(b"k20", b"k20-832", 832),
            RowEntry::new_value(b"k21", b"k21-833", 833),
            RowEntry::new_value(b"k22", b"k22-834", 834),
            RowEntry::new_value(b"k23", b"k23-835", 835),
        ]
    )]
    #[case::l0_and_sr_overlap(
        vec![
            vec![ // l0(0)
                RowEntry::new_value(b"k00", b"k00-900", 900),
                RowEntry::new_value(b"k01", b"k01-901", 901),
                RowEntry::new_value(b"k02", b"k02-902", 902),
                RowEntry::new_value(b"k03", b"k03-903", 903),
                RowEntry::new_value(b"k04", b"k04-904", 904),
                RowEntry::new_value(b"k05", b"k05-905", 905),
            ],
            vec![ // l0(1)
                RowEntry::new_value(b"k02", b"k02-920", 920),
                RowEntry::new_value(b"k03", b"k03-921", 921),
                RowEntry::new_value(b"k04", b"k04-922", 922),
                RowEntry::new_value(b"k06", b"k06-923", 923),
                RowEntry::new_value(b"k07", b"k07-924", 924),
            ],
        ],
        vec![
            vec![ // sr(0)
                vec![ // sr(0) sst(0)
                    RowEntry::new_value(b"k01", b"k01-800", 800),
                    RowEntry::new_value(b"k03", b"k03-801", 801),
                    RowEntry::new_value(b"k05", b"k05-802", 802),
                    RowEntry::new_value(b"k08", b"k08-803", 803),
                    RowEntry::new_value(b"k09", b"k09-804", 804),
                ],
            ],
            vec![ // sr(1)
                vec![ // sr(1) sst(0)
                    RowEntry::new_value(b"k02", b"k02-850", 850),
                    RowEntry::new_value(b"k04", b"k04-851", 851),
                    RowEntry::new_value(b"k05", b"k05-852", 852),
                    RowEntry::new_value(b"k07", b"k07-853", 853),
                    RowEntry::new_value(b"k10", b"k10-854", 854),
                ],
            ],
        ],
        4096,
        67_108_864,
        Some(7),
        None,
        vec![
            RowEntry::new_value(b"k03", b"k03-801", 801),
            RowEntry::new_value(b"k04", b"k04-922", 922),
            RowEntry::new_value(b"k04", b"k04-904", 904),
            RowEntry::new_value(b"k04", b"k04-851", 851),
            RowEntry::new_value(b"k05", b"k05-905", 905),
            RowEntry::new_value(b"k05", b"k05-852", 852),
            RowEntry::new_value(b"k05", b"k05-802", 802),
            RowEntry::new_value(b"k06", b"k06-923", 923),
            RowEntry::new_value(b"k07", b"k07-924", 924),
            RowEntry::new_value(b"k07", b"k07-853", 853),
            RowEntry::new_value(b"k08", b"k08-803", 803),
            RowEntry::new_value(b"k09", b"k09-804", 804),
            RowEntry::new_value(b"k10", b"k10-854", 854),
        ]
    )]
    #[case::tombstones_resume(
        vec![
            vec![ // l0(0)
                RowEntry::new_value(b"k00", b"k00-100", 100),
                RowEntry::new_value(b"k01", b"k01-101", 101),
                RowEntry::new_tombstone(b"k02", 105),
                RowEntry::new_value(b"k02", b"k02-103", 103),
                RowEntry::new_value(b"k03", b"k03-106", 106),
            ],
        ],
        vec![
            vec![ // sr(0)
                vec![ // sr(0) sst(0)
                    RowEntry::new_value(b"k01", b"k01-90", 90),
                    RowEntry::new_value(b"k02", b"k02-95", 95),
                    RowEntry::new_value(b"k04", b"k04-107", 107),
                ],
            ],
        ],
        4096,
        67_108_864,
        Some(3),
        None,
        vec![
            RowEntry::new_value(b"k02", b"k02-103", 103),
            RowEntry::new_value(b"k02", b"k02-95", 95),
            RowEntry::new_value(b"k03", b"k03-106", 106),
            RowEntry::new_value(b"k04", b"k04-107", 107),
        ]
    )]
    #[case::no_output_ssts(
        vec![
            vec![ // l0(0)
                RowEntry::new_value(b"k00", b"k00-10", 10),
                RowEntry::new_value(b"k02", b"k02-25", 25),
                RowEntry::new_value(b"k02", b"k02-20", 20),
            ],
            vec![ // l0(1)
                RowEntry::new_value(b"k01", b"k01-30", 30),
                RowEntry::new_value(b"k03", b"k03-40", 40),
            ],
        ],
        vec![
            vec![ // sr(0)
                vec![ // sr(0) sst(0)
                    RowEntry::new_value(b"k01", b"k01-15", 15),
                    RowEntry::new_value(b"k02", b"k02-22", 22),
                    RowEntry::new_value(b"k04", b"k04-50", 50),
                ],
            ],
        ],
        4096,
        67_108_864,
        None,
        None,
        vec![
            RowEntry::new_value(b"k00", b"k00-10", 10),
            RowEntry::new_value(b"k01", b"k01-30", 30),
            RowEntry::new_value(b"k01", b"k01-15", 15),
            RowEntry::new_value(b"k02", b"k02-25", 25),
            RowEntry::new_value(b"k02", b"k02-22", 22),
            RowEntry::new_value(b"k02", b"k02-20", 20),
            RowEntry::new_value(b"k03", b"k03-40", 40),
            RowEntry::new_value(b"k04", b"k04-50", 50),
        ]
    )]
    #[case::resume_at_end(
        vec![
            vec![ // l0(0)
                RowEntry::new_value(b"k00", b"k00-10", 10),
                RowEntry::new_value(b"k01", b"k01-11", 11),
            ],
        ],
        Vec::new(),
        4096,
        67_108_864,
        Some(1),
        None,
        vec![]
    )]
    #[case::merge_operator_resume(
        vec![
            vec![ // l0(0)
                RowEntry::new_value(b"k00", b"k00-100", 100),
                RowEntry::new_merge(b"k01", b"b", 5),
                RowEntry::new_merge(b"k02", b"y", 6),
                RowEntry::new_value(b"k03", b"k03-103", 103),
            ],
        ],
        vec![
            vec![ // sr(0)
                vec![ // sr(0) sst(0)
                    RowEntry::new_merge(b"k01", b"a", 4),
                    RowEntry::new_value(b"k01", b"base", 3),
                    RowEntry::new_merge(b"k02", b"x", 4),
                    RowEntry::new_value(b"k02", b"base2", 2),
                ],
            ],
        ],
        4096,
        67_108_864,
        Some(0),
        Some(Arc::new(StringConcatMergeOperator {}) as MergeOperatorType),
        vec![
            RowEntry::new_value(b"k01", b"baseab", 5),
            RowEntry::new_value(b"k02", b"base2xy", 6),
            RowEntry::new_value(b"k03", b"k03-103", 103),
        ]
    )]
    #[case::merge_operator_resume_skip_merged_operands(
        vec![
            vec![ // l0(0)
                RowEntry::new_merge(b"k00", b"10", 10),
                RowEntry::new_merge(b"k00", b"4", 4),
                RowEntry::new_merge(b"k00", b"1", 1),
                RowEntry::new_value(b"k01", b"k01-5", 5),
            ],
        ],
        Vec::new(),
        4096,
        67_108_864,
        Some(0),
        Some(Arc::new(StringConcatMergeOperator {}) as MergeOperatorType),
        vec![
            RowEntry::new_value(b"k01", b"k01-5", 5),
        ]
    )]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_load_iterators_resume(
        #[case] l0_entry_sets: Vec<Vec<RowEntry>>,
        #[case] sr_entry_sets: Vec<Vec<Vec<RowEntry>>>,
        #[case] block_size: usize,
        #[case] output_max_sst_size: usize,
        #[case] resume_index: Option<usize>,
        #[case] merge_operator: Option<MergeOperatorType>,
        #[case] expected_rows: Vec<RowEntry>,
    ) {
        let handle = tokio::runtime::Handle::current();
        let options = Arc::new(CompactionWorkerOptions::default());
        let (tx, _rx) = async_channel::unbounded::<WorkerMessage>();
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let root_path = Path::from("testdb-load-iterators");
        let clock = Arc::new(DefaultSystemClock::new());
        let table_store = Arc::new(TableStore::new(
            object_store.clone(),
            SsTableFormat {
                block_size,
                ..SsTableFormat::default()
            },
            root_path.clone(),
            None,
            TableStoreKind::Compactor,
            BlockCachePolicy::default(),
        ));
        let manifest_store = Arc::new(ManifestStore::new(&root_path, object_store.clone()));
        StoredManifest::create_new_db(manifest_store.clone(), ManifestCore::new(), clock.clone())
            .await
            .unwrap();
        let retention_min_seq = if merge_operator.is_some() {
            None
        } else {
            Some(0)
        };
        let executor = TokioCompactionExecutor::new(TokioCompactionExecutorOptions {
            handle,
            options,
            worker_tx: tx,
            table_store: table_store.clone(),
            rand: Arc::new(DbRand::new(100u64)),
            stats: {
                let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();
                Arc::new(CompactionStats::new(&recorder))
            },
            worker_stats: WorkerStats::noop(),
            clock,
            manifest_store,
            merge_operator,
            #[cfg(feature = "compaction_filters")]
            compaction_filter_supplier: None,
        });

        // Materialize L0 SSTs from the provided entry sets. Use a huge max size so
        // each entry set stays in a single SST, keeping the inputs predictable.
        let mut l0_sst_views = Vec::new();
        let mut sorted_runs = Vec::new();
        let mut all_entries = Vec::new();

        for entries in &l0_entry_sets {
            let ssts = write_sst(&table_store, entries, usize::MAX).await;
            l0_sst_views.extend(ssts.into_iter().map(SsTableView::identity));
            all_entries.extend(entries.iter().cloned());
        }

        // Materialize sorted-run SSTs and group them into multiple SortedRun inputs,
        // again keeping each entry set as a single SST by using a huge max size.
        if !sr_entry_sets.is_empty() {
            for (sr_id, sr_sst_sets) in sr_entry_sets.iter().enumerate() {
                let mut sr_ssts = Vec::new();
                for entries in sr_sst_sets {
                    let ssts = write_sst(&table_store, entries, usize::MAX).await;
                    sr_ssts.extend(ssts);
                    all_entries.extend(entries.iter().cloned());
                }
                sorted_runs.push(SortedRun::new(
                    sr_id as u32,
                    sr_ssts.into_iter().map(SsTableView::identity),
                ));
            }
        }

        // Sort the merged inputs to match the iterator order (key asc, seq desc).
        let mut sorted_entries = all_entries.clone();
        sorted_entries.sort_by(|left, right| match left.key.cmp(&right.key) {
            Ordering::Equal => right.seq.cmp(&left.seq),
            other => other,
        });
        // Persist the "already-written" output SSTs when resuming from a prefix.
        let output_ssts = if let Some(resume_index) = resume_index {
            assert!(resume_index < sorted_entries.len());
            write_sst(
                &table_store,
                &sorted_entries[..=resume_index],
                output_max_sst_size,
            )
            .await
        } else {
            Vec::new()
        };
        // Build subcompaction args that resume from the output SST starting point and
        // expect the iterator to continue at the correct next row.
        let subcompaction_args = SubcompactionArgs {
            index: 0,
            segment: Bytes::new(),
            range: BytesRange::unbounded(),
            destination: 0,
            l0_sst_views,
            sorted_runs,
            compaction_clock_tick: 0,
            is_dest_last_run: false,
                complete_input_through: None,
            retention_min_seq,
            output_ssts,
        };

        // Verify the resumed iterator yields all remaining rows, starting immediately
        // after the persisted prefix and continuing in sorted order.
        let sequence_tracker = {
            let stored_manifest = StoredManifest::load(
                executor.inner.manifest_store.clone(),
                executor.inner.clock.clone(),
            )
            .await
            .unwrap();
            Arc::new(stored_manifest.db_state().sequence_tracker.clone())
        };
        let mut iter = executor
            .inner
            .load_iterators(&subcompaction_args, sequence_tracker)
            .await
            .unwrap();
        let mut resumed_entries = Vec::new();
        while let Some(entry) = iter.next().await.unwrap() {
            resumed_entries.push(entry);
        }
        assert_eq!(resumed_entries, expected_rows);
    }

    // Property-based test to verify that executing a compaction job
    // with resume points produces the same final output as executing
    // the full compaction job in one go.
    //
    // The test generates random L0 and sorted-run entry sets,
    // writes them to SSTs, and runs the compaction job twice:
    //
    // 1. Full compaction job without resuming.
    // 2. Compaction job with resume points inserted at various
    //    positions in the output.
    //
    // The outputs of both runs are compared to ensure they match.
    #[test]
    fn test_execute_compaction_job_resume_matches_full() {
        const PROPTEST_CASES: u32 = 256;
        const KEY_SIZE: usize = 4;
        const VALUE_SIZE: usize = 6;
        const MAX_SEQ: u64 = 2000;
        const MAX_ENTRY_SET_LEN: usize = 64;
        const MAX_L0_SETS: usize = 3;
        const MAX_SR_SETS: usize = 3;
        const RESUME_RANGE_MAX: usize = 16;
        const RESUME_POINTS_MIN: usize = 1;
        const RESUME_POINTS_MAX: usize = 3;
        const MAX_SST_SIZE: usize = 128;
        const BLOCK_SIZE: usize = 64;

        proptest!(Config::with_cases(PROPTEST_CASES), |(
            l0_specs in proptest::collection::vec(
                proptest::collection::vec(
                    (
                        arbitrary::nonempty_bytes(KEY_SIZE),
                        0u64..MAX_SEQ,
                        prop_oneof![
                            arbitrary::nonempty_bytes(VALUE_SIZE).prop_map(ValueDeletable::Value),
                            arbitrary::nonempty_bytes(VALUE_SIZE).prop_map(ValueDeletable::Merge),
                            Just(ValueDeletable::Tombstone),
                        ],
                    ),
                    1..=MAX_ENTRY_SET_LEN,
                ),
                1..=MAX_L0_SETS,
            ),
            sr_specs in proptest::collection::vec(
                proptest::collection::vec(
                    (
                        arbitrary::nonempty_bytes(KEY_SIZE),
                        0u64..MAX_SEQ,
                        prop_oneof![
                            arbitrary::nonempty_bytes(VALUE_SIZE).prop_map(ValueDeletable::Value),
                            arbitrary::nonempty_bytes(VALUE_SIZE).prop_map(ValueDeletable::Merge),
                            Just(ValueDeletable::Tombstone),
                        ],
                    ),
                    1..=MAX_ENTRY_SET_LEN,
                ),
                0..=MAX_SR_SETS,
            ),
            resume_points in proptest::collection::vec(
                0usize..RESUME_RANGE_MAX,
                RESUME_POINTS_MIN..=RESUME_POINTS_MAX,
            ),
        )| {
            prop_assume!(!has_duplicate_key_seq_specs(&l0_specs, &sr_specs));
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let l0_entry_sets = l0_specs
                    .into_iter()
                    .map(build_row_entries)
                    .collect::<Vec<_>>();
                let sr_entry_sets = sr_specs
                    .into_iter()
                    .map(|entries| vec![build_row_entries(entries)])
                    .collect::<Vec<_>>();

                let has_merge = l0_entry_sets
                    .iter()
                    .flatten()
                    .chain(sr_entry_sets.iter().flatten().flatten())
                    .any(|entry| matches!(entry.value, ValueDeletable::Merge(_)));
                let merge_operator = if has_merge {
                    Some(Arc::new(StringConcatMergeOperator {}) as MergeOperatorType)
                } else {
                    None
                };
                let retention_min_seq = if merge_operator.is_some() {
                    None
                } else {
                    Some(0)
                };

                let handle = tokio::runtime::Handle::current();
                let options = CompactionWorkerOptions {
                    max_sst_size: MAX_SST_SIZE,
                    ..Default::default()
                };
                let options = Arc::new(options);
                let (tx, _rx) = async_channel::unbounded::<WorkerMessage>();
                let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
                let root_path = Path::from("testdb-exec-resume");
                let clock = Arc::new(DefaultSystemClock::new());
                let table_store = Arc::new(TableStore::new(
                    object_store.clone(),
                    SsTableFormat {
                        block_size: BLOCK_SIZE,
                        ..SsTableFormat::default()
                    },
                    root_path.clone(),
                    None,
                    TableStoreKind::Compactor, BlockCachePolicy::default()));
                let manifest_store = Arc::new(ManifestStore::new(&root_path, object_store.clone()));
                StoredManifest::create_new_db(
                    manifest_store.clone(),
                    ManifestCore::new(),
                    clock.clone(),
                )
                .await
                .unwrap();

                let executor = TokioCompactionExecutor::new(TokioCompactionExecutorOptions {
                    handle,
                    options,
                    worker_tx: tx,
                    table_store: table_store.clone(),
                    rand: Arc::new(DbRand::new(100u64)),
                    stats: {
                        let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();
                        Arc::new(CompactionStats::new(&recorder))
                    },
                    worker_stats: WorkerStats::noop(),
                    clock,
                    manifest_store,
                    merge_operator,
                    #[cfg(feature = "compaction_filters")]
                    compaction_filter_supplier: None,
                });

                let mut l0_ssts = Vec::new();
                for entries in &l0_entry_sets {
                    let ssts = write_ssts(&table_store, entries, usize::MAX).await;
                    l0_ssts.extend(ssts.into_iter().map(SsTableView::identity));
                }

                let sorted_runs = build_sorted_runs(&table_store, &sr_entry_sets, usize::MAX).await;

                let full_run = executor
                    .inner
                    .plan_and_execute_compaction_job(StartCompactionJobArgs {
                        id: Ulid::new(),
                        compaction_id: Ulid::new(),
                        segment: Bytes::new(),
                        destination: 0,
                        l0_sst_views: l0_ssts.clone(),
                        sorted_runs: sorted_runs.clone(),
                        compaction_clock_tick: 0,
                        is_dest_last_run: false,
                complete_input_through: None,
                        retention_min_seq,
                        ctx: Some(CompactionContext::new(
                            vec![Subcompaction::new(BytesRange::unbounded())],
                            retention_min_seq,
                        )),
                    })
                    .await
                    .unwrap();

                let mut expected_entries = Vec::new();
                for view in full_run.sst_views() {
                    let mut iter = SstIterator::new(
                        SstView::Owned(
                            Box::new(SsTableView::identity(view.sst.clone())),
                            BytesRange::from(..),
                        ),
                        table_store.clone(),
                        SstIteratorOptions::default(),
                    )
                    .unwrap();
                    iter.init().await.unwrap();
                    while let Some(entry) = iter.next().await.unwrap() {
                        expected_entries.push(entry);
                    }
                }

                assert!(!expected_entries.is_empty());
                let mut resume_indices = resume_points
                    .into_iter()
                    .map(|index| index % expected_entries.len())
                    .collect::<Vec<_>>();
                resume_indices.sort_unstable();
                resume_indices.dedup();

                for resume_index in resume_indices {
                    let output_ssts = write_ssts(
                        &table_store,
                        &expected_entries[..=resume_index],
                        MAX_SST_SIZE,
                    )
                    .await;
                    let resumed_run = executor
                        .inner
                        .plan_and_execute_compaction_job(StartCompactionJobArgs {
                            id: Ulid::new(),
                            compaction_id: Ulid::new(),
                            segment: Bytes::new(),
                            destination: 0,
                            l0_sst_views: l0_ssts.clone(),
                            sorted_runs: sorted_runs.clone(),
                            compaction_clock_tick: 0,
                            is_dest_last_run: false,
                complete_input_through: None,
                            retention_min_seq,
                            ctx: Some(CompactionContext::new(
                                vec![Subcompaction::new(BytesRange::unbounded())
                                    .with_output_ssts(output_ssts)],
                                retention_min_seq,
                            )),
                        })
                        .await
                        .unwrap();

                    let mut resumed_entries = Vec::new();
                    for view in resumed_run.sst_views() {
                        let mut iter = SstIterator::new(
                            SstView::Owned(
                                Box::new(SsTableView::identity(view.sst.clone())),
                                BytesRange::from(..),
                            ),
                            table_store.clone(),
                            SstIteratorOptions::default(),
                        )
                        .unwrap();
                        iter.init().await.unwrap();
                        while let Some(entry) = iter.next().await.unwrap() {
                            resumed_entries.push(entry);
                        }
                    }

                    assert_eq!(resumed_entries, expected_entries);
                }

            });
        });
    }

    /// Reads every entry out of a sorted run's output SSTs, in order, so two
    /// runs can be compared for byte-identical merged output.
    async fn read_run_entries(table_store: &Arc<TableStore>, run: &SortedRun) -> Vec<RowEntry> {
        let mut entries = Vec::new();
        for sst in run.sst_views() {
            let mut iter = SstIterator::new(
                SstView::Borrowed(sst, BytesRange::from(..)),
                table_store.clone(),
                SstIteratorOptions::default(),
            )
            .unwrap();
            iter.init().await.unwrap();
            while let Some(entry) = iter.next().await.unwrap() {
                entries.push(entry);
            }
        }
        entries
    }

    /// Four contiguous ranges that partition the `key00000`..`key00399`
    /// keyspace, used to drive the executor's parallel path with an explicit
    /// plan (the planner itself only emits a single range).
    fn four_ranges() -> Vec<Subcompaction> {
        vec![
            Subcompaction::new(BytesRange::from_slice(..b"key00100".as_slice())),
            Subcompaction::new(BytesRange::from_slice(
                b"key00100".as_slice()..b"key00200".as_slice(),
            )),
            Subcompaction::new(BytesRange::from_slice(
                b"key00200".as_slice()..b"key00300".as_slice(),
            )),
            Subcompaction::new(BytesRange::from_slice(b"key00300".as_slice()..)),
        ]
    }

    /// Output SST size the subcompaction tests run with: small enough that each
    /// range rolls several SSTs.
    const SUBCOMPACTION_SST_SIZE: usize = 512;

    /// Builds value rows `key{i:05} -> {value_prefix}-{i}` with sequence
    /// `seq_base + i`, for `i` in `range` taken every `step`.
    fn rows(
        range: std::ops::Range<u64>,
        step: usize,
        value_prefix: &str,
        seq_base: u64,
    ) -> Vec<RowEntry> {
        range
            .step_by(step)
            .map(|i| {
                RowEntry::new_value(
                    format!("key{i:05}").as_bytes(),
                    format!("{value_prefix}-{i}").as_bytes(),
                    seq_base + i,
                )
            })
            .collect()
    }

    /// A [`StartCompactionJobArgs`] over `sst_views`/`sorted_runs` running the
    /// given `subcompactions` plan, with the defaults the subcompaction tests
    /// share (destination 0, retaining everything, not the last run).
    fn job_with_subcompactions(
        l0_sst_views: Vec<SsTableView>,
        sorted_runs: Vec<SortedRun>,
        subcompactions: Vec<Subcompaction>,
    ) -> StartCompactionJobArgs {
        StartCompactionJobArgs {
            id: Ulid::new(),
            compaction_id: Ulid::new(),
            segment: Bytes::new(),
            destination: 0,
            l0_sst_views,
            sorted_runs,
            compaction_clock_tick: 0,
            is_dest_last_run: false,
                complete_input_through: None,
            retention_min_seq: Some(0),
            ctx: Some(CompactionContext::new(subcompactions, Some(0))),
        }
    }

    /// Builds a fresh-DB table store (small block size, so ranges roll several
    /// SSTs) and an executor wired to [`SUBCOMPACTION_SST_SIZE`], optionally
    /// with a compaction-filter supplier. Returns the executor, its table
    /// store, and the worker-message receiver.
    async fn subcompaction_env(
        path: &str,
        #[cfg(feature = "compaction_filters")] filter: Option<Arc<dyn CompactionFilterSupplier>>,
    ) -> (
        TokioCompactionExecutor,
        Arc<TableStore>,
        async_channel::Receiver<WorkerMessage>,
    ) {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let root_path = Path::from(path);
        let clock = Arc::new(DefaultSystemClock::new());
        let table_store = Arc::new(TableStore::new(
            object_store.clone(),
            SsTableFormat {
                block_size: 256,
                ..SsTableFormat::default()
            },
            root_path.clone(),
            None,
            TableStoreKind::Compactor,
            BlockCachePolicy::default(),
        ));
        let manifest_store = Arc::new(ManifestStore::new(&root_path, object_store.clone()));
        StoredManifest::create_new_db(manifest_store.clone(), ManifestCore::new(), clock.clone())
            .await
            .unwrap();
        let (tx, rx) = async_channel::unbounded::<WorkerMessage>();
        let executor = TokioCompactionExecutor::new(TokioCompactionExecutorOptions {
            handle: tokio::runtime::Handle::current(),
            options: Arc::new(CompactionWorkerOptions {
                max_sst_size: SUBCOMPACTION_SST_SIZE,
                ..CompactionWorkerOptions::default()
            }),
            worker_tx: tx,
            table_store: table_store.clone(),
            rand: Arc::new(DbRand::new(100u64)),
            stats: {
                let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();
                Arc::new(CompactionStats::new(&recorder))
            },
            worker_stats: WorkerStats::noop(),
            clock,
            manifest_store,
            merge_operator: None,
            #[cfg(feature = "compaction_filters")]
            compaction_filter_supplier: filter,
        });
        (executor, table_store, rx)
    }

    /// The overlapping multi-source inputs the split tests share: two L0s and
    /// two sorted runs over `key00000`..`key00399`, each spanning several SSTs.
    async fn split_inputs(table_store: &Arc<TableStore>) -> (Vec<SsTableView>, Vec<SortedRun>) {
        let mut sst_views = Vec::new();
        for entries in [
            rows(0..400, 2, "l0a", 10_000),
            rows(1..400, 2, "l0b", 20_000),
        ] {
            let ssts = write_ssts(table_store, &entries, SUBCOMPACTION_SST_SIZE).await;
            sst_views.extend(ssts.into_iter().map(SsTableView::identity));
        }
        let sorted_runs = build_sorted_runs(
            table_store,
            &[
                vec![rows(0..400, 1, "sra", 1)],
                vec![rows(0..400, 5, "srb", 5_000)],
            ],
            SUBCOMPACTION_SST_SIZE,
        )
        .await;
        (sst_views, sorted_runs)
    }

    /// Drains the non-empty subcompaction snapshots reported on `rx`.
    fn collect_snapshots(rx: &async_channel::Receiver<WorkerMessage>) -> Vec<Vec<Subcompaction>> {
        let mut snapshots = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let WorkerMessage::CompactionJobProgress { ctx, .. } = msg {
                let subcompactions = ctx.subcompactions().clone();
                if !subcompactions.is_empty() {
                    snapshots.push(subcompactions);
                }
            }
        }
        snapshots
    }

    /// A compaction run as parallel subcompactions produces exactly the same
    /// merged entries as a single-threaded merge. The split run chunks the
    /// output into different SSTs (each range rolls its own, so sizes, count,
    /// and boundaries differ), but the logical content is identical: every
    /// key's full set of versions falls in one range, so retention and
    /// merge-operator decisions are made per key regardless of where the range
    /// boundaries land.
    #[tokio::test(flavor = "multi_thread")]
    async fn should_match_single_merge_when_split_into_subcompactions() {
        let (executor, table_store, _rx) = subcompaction_env(
            "testdb-split-equivalence",
            #[cfg(feature = "compaction_filters")]
            None,
        )
        .await;
        let (sst_views, sorted_runs) = split_inputs(&table_store).await;

        // when: the inputs run as a single merge (planner falls back to one
        // range), then split into four explicit parallel ranges.
        let single = executor
            .inner
            .plan_and_execute_compaction_job(job_with_subcompactions(
                sst_views.clone(),
                sorted_runs.clone(),
                vec![Subcompaction::new(BytesRange::unbounded())],
            ))
            .await
            .unwrap();
        let expected_entries = read_run_entries(&table_store, &single).await;
        assert!(!expected_entries.is_empty());
        let split = executor
            .inner
            .plan_and_execute_compaction_job(job_with_subcompactions(
                sst_views,
                sorted_runs,
                four_ranges(),
            ))
            .await
            .unwrap();

        // then: the flattened entries are identical.
        assert_eq!(
            read_run_entries(&table_store, &split).await,
            expected_entries
        );
    }

    /// Each progress report carries a persistable snapshot of the whole plan:
    /// the range set is stable across reports, and the final snapshot accounts
    /// for every output SST the run is built from.
    #[tokio::test(flavor = "multi_thread")]
    async fn should_report_a_stable_persistable_subcompaction_plan() {
        let (executor, table_store, rx) = subcompaction_env(
            "testdb-split-plan",
            #[cfg(feature = "compaction_filters")]
            None,
        )
        .await;
        let (sst_views, sorted_runs) = split_inputs(&table_store).await;

        // when: the job runs split into four ranges.
        let split = executor
            .inner
            .plan_and_execute_compaction_job(job_with_subcompactions(
                sst_views,
                sorted_runs,
                four_ranges(),
            ))
            .await
            .unwrap();

        // then: every snapshot reports the same four-range plan, and the last
        // one accounts for every output SST in the run.
        let snapshots = collect_snapshots(&rx);
        assert!(!snapshots.is_empty(), "expected subcompaction snapshots");
        assert!(snapshots.iter().all(|s| s.len() == 4));
        let final_output: usize = snapshots
            .last()
            .unwrap()
            .iter()
            .map(|s| s.output_ssts().len())
            .sum();
        assert_eq!(
            final_output,
            split.sst_views().len(),
            "final snapshot should capture every output SST"
        );
    }

    /// Resuming from any persisted snapshot — the bare plan, a mid-flight
    /// state, or the fully-complete state — converges to the same merged output
    /// and reuses the output SSTs already recorded against each range. Resuming
    /// a completed compaction does no merge work but still reports its full
    /// processed size rather than regressing to zero.
    #[tokio::test(flavor = "multi_thread")]
    async fn should_resume_from_any_snapshot() {
        let (executor, table_store, rx) = subcompaction_env(
            "testdb-split-resume",
            #[cfg(feature = "compaction_filters")]
            None,
        )
        .await;
        let (sst_views, sorted_runs) = split_inputs(&table_store).await;

        // given: a completed split run and the snapshots reported during it.
        let baseline_run = executor
            .inner
            .plan_and_execute_compaction_job(job_with_subcompactions(
                sst_views.clone(),
                sorted_runs.clone(),
                four_ranges(),
            ))
            .await
            .unwrap();
        let baseline_entries = read_run_entries(&table_store, &baseline_run).await;
        let snapshots = collect_snapshots(&rx);
        assert!(
            snapshots.len() >= 2,
            "expected several snapshots to resume from"
        );

        // when/then: resuming from the bare plan, a mid-flight snapshot, and the
        // fully-complete snapshot each converges to the baseline output and
        // reuses the SSTs already recorded against each range.
        for index in [0, snapshots.len() / 2, snapshots.len() - 1] {
            let snapshot = snapshots[index].clone();
            // Drain buffered messages so we observe only this resume's progress.
            while rx.try_recv().is_ok() {}
            let resumed = executor
                .inner
                .plan_and_execute_compaction_job(job_with_subcompactions(
                    sst_views.clone(),
                    sorted_runs.clone(),
                    snapshot.clone(),
                ))
                .await
                .unwrap();
            let mut resume_bytes = Vec::new();
            while let Ok(msg) = rx.try_recv() {
                if let WorkerMessage::CompactionJobProgress {
                    bytes_processed, ..
                } = msg
                {
                    resume_bytes.push(bytes_processed);
                }
            }
            assert_eq!(
                read_run_entries(&table_store, &resumed).await,
                baseline_entries,
                "resume from snapshot {index} diverged"
            );
            for sst in snapshot.iter().flat_map(|s| s.output_ssts()) {
                assert!(
                    resumed.sst_views().iter().any(|v| v.sst.id == sst.id),
                    "previously recorded subcompaction output SST was not reused"
                );
            }

            // The fully-complete snapshot must do no merge work — the run is
            // rebuilt purely from recorded output — yet still report its full
            // processed size, never regressing to zero.
            if index == snapshots.len() - 1 {
                let recorded: usize = snapshot.iter().map(|s| s.output_ssts().len()).sum();
                assert_eq!(
                    resumed.sst_views().len(),
                    recorded,
                    "resuming a completed compaction must not produce new SSTs"
                );
                assert!(
                    !resume_bytes.is_empty() && resume_bytes.iter().all(|&b| b > 0),
                    "resuming a completed compaction must never report zero bytes, got {resume_bytes:?}"
                );
            }
        }
    }

    /// When one subcompaction fails, the job must abort its sibling ranges and
    /// report the failure (rather than hanging on them or committing partial
    /// output). A compaction filter errors only on keys in the upper part of
    /// the keyspace, so only the range covering those keys fails while lower
    /// ranges run normally.
    #[cfg(feature = "compaction_filters")]
    #[tokio::test(flavor = "multi_thread")]
    async fn should_fail_job_when_a_subcompaction_fails() {
        use crate::compaction_filter::{
            CompactionFilter, CompactionFilterDecision, CompactionFilterError,
            CompactionFilterSupplier, CompactionJobContext,
        };

        struct FailUpperRangeFilter;

        #[async_trait::async_trait]
        impl CompactionFilter for FailUpperRangeFilter {
            async fn filter(
                &mut self,
                entry: &RowEntry,
            ) -> Result<CompactionFilterDecision, CompactionFilterError> {
                if entry.key.as_ref() >= b"key00300".as_slice() {
                    Err(CompactionFilterError::FilterError(
                        "injected failure".into(),
                    ))
                } else {
                    Ok(CompactionFilterDecision::Keep)
                }
            }

            async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
                Ok(())
            }
        }

        struct FailUpperRangeFilterSupplier;

        #[async_trait::async_trait]
        impl CompactionFilterSupplier for FailUpperRangeFilterSupplier {
            async fn create_compaction_filter(
                &self,
                _context: &CompactionJobContext,
            ) -> Result<Box<dyn CompactionFilter>, CompactionFilterError> {
                Ok(Box::new(FailUpperRangeFilter))
            }
        }

        // given: input spanning the whole keyspace, run as four explicit ranges
        // where only the top range trips the failing filter.
        let (executor, table_store, _rx) = subcompaction_env(
            "testdb-subcompaction-failure",
            Some(Arc::new(FailUpperRangeFilterSupplier)),
        )
        .await;
        let sorted_runs = build_sorted_runs(
            &table_store,
            &[vec![rows(0..400, 1, "v", 1)]],
            SUBCOMPACTION_SST_SIZE,
        )
        .await;

        // when: the split job runs.
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            executor
                .inner
                .plan_and_execute_compaction_job(job_with_subcompactions(
                    vec![],
                    sorted_runs,
                    four_ranges(),
                )),
        )
        .await
        // then: it fails promptly rather than hanging on the aborted siblings.
        .expect("job should fail promptly rather than hang on aborted siblings");

        let err = result.expect_err("job with a failing subcompaction should fail");
        assert!(
            matches!(err, SlateDBError::CompactionFilterError(_)),
            "expected CompactionFilterError, got: {err:?}"
        );
    }

    /// Stopping the executor must abort in-flight subcompactions, not just the
    /// parent job. A compaction filter that never returns holds every range's
    /// merge open and is dropped (releasing a shared counter) only when its
    /// task future is dropped — so once the counter confirms the split job is
    /// in flight, `stop()` must drive it back to zero.
    #[cfg(feature = "compaction_filters")]
    #[tokio::test(flavor = "multi_thread")]
    async fn should_abort_in_flight_subcompactions_on_stop() {
        use crate::compaction_filter::{
            CompactionFilter, CompactionFilterDecision, CompactionFilterError,
            CompactionFilterSupplier, CompactionJobContext,
        };
        use std::sync::atomic::{AtomicI64, Ordering};

        // Each range's merge suspends on its first row and never resumes; the
        // filter counts itself as running on entry and uncounts on drop, so the
        // shared counter mirrors the number of in-flight subcompactions even
        // without a metric.
        struct BlockForeverFilter {
            running: Arc<AtomicI64>,
            counted: bool,
        }

        impl Drop for BlockForeverFilter {
            fn drop(&mut self) {
                if self.counted {
                    self.running.fetch_sub(1, Ordering::SeqCst);
                }
            }
        }

        #[async_trait::async_trait]
        impl CompactionFilter for BlockForeverFilter {
            async fn filter(
                &mut self,
                _entry: &RowEntry,
            ) -> Result<CompactionFilterDecision, CompactionFilterError> {
                if !self.counted {
                    self.counted = true;
                    self.running.fetch_add(1, Ordering::SeqCst);
                }
                std::future::pending::<Result<CompactionFilterDecision, CompactionFilterError>>()
                    .await
            }

            async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
                Ok(())
            }
        }

        struct BlockForeverFilterSupplier {
            running: Arc<AtomicI64>,
        }

        #[async_trait::async_trait]
        impl CompactionFilterSupplier for BlockForeverFilterSupplier {
            async fn create_compaction_filter(
                &self,
                _context: &CompactionJobContext,
            ) -> Result<Box<dyn CompactionFilter>, CompactionFilterError> {
                Ok(Box::new(BlockForeverFilter {
                    running: self.running.clone(),
                    counted: false,
                }))
            }
        }

        // Polls the shared running counter until `pred` holds, failing the test
        // if it never does within the timeout.
        async fn await_running(
            running: &Arc<AtomicI64>,
            pred: impl Fn(i64) -> bool,
            what: &str,
        ) -> i64 {
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    let n = running.load(Ordering::SeqCst);
                    if pred(n) {
                        return n;
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
        }

        // given: a split compaction running in the background (started via the
        // real path so its task lands in the map stop() drains), held open by a
        // filter that blocks forever and decrements `running` on drop.
        let running = Arc::new(AtomicI64::new(0));
        let (executor, table_store, _rx) = subcompaction_env(
            "testdb-stop-aborts-subcompactions",
            Some(Arc::new(BlockForeverFilterSupplier {
                running: running.clone(),
            })),
        )
        .await;
        let sorted_runs = build_sorted_runs(
            &table_store,
            &[vec![rows(0..400, 1, "v", 1)]],
            SUBCOMPACTION_SST_SIZE,
        )
        .await;
        executor.inner.start_compaction_job(job_with_subcompactions(
            vec![],
            sorted_runs,
            four_ranges(),
        ));

        let in_flight = await_running(&running, |n| n >= 2, "subcompactions to start").await;
        assert!(in_flight >= 2, "expected a split job, got {in_flight}");

        // when: the executor is stopped.
        executor.inner.stop();

        // then: every in-flight subcompaction is aborted and its filter dropped.
        let after = await_running(&running, |n| n == 0, "subcompactions to abort").await;
        assert_eq!(after, 0);
    }

    /// The planner plans a single unbounded-range subcompaction for a fresh
    /// job, and resumes a persisted multi-range plan verbatim so the output
    /// already recorded against its ranges stays valid.
    #[tokio::test(flavor = "multi_thread")]
    async fn should_resume_persisted_plan_or_plan_single_unbounded_range() {
        // given: an executor and a fresh job with no persisted plan.
        let ctx = TestContextBuilder::new("testdb-no-replan").build().await;
        let args = StartCompactionJobArgs {
            id: Ulid::new(),
            compaction_id: Ulid::new(),
            segment: Bytes::new(),
            destination: 0,
            l0_sst_views: vec![],
            sorted_runs: vec![],
            compaction_clock_tick: 0,
            is_dest_last_run: false,
                complete_input_through: None,
            retention_min_seq: Some(0),
            ctx: None,
        };

        // when/then: a fresh job with no inputs plans a single unbounded range.
        let planned = ctx
            .executor
            .inner
            .plan_compaction_job(args.clone())
            .await
            .unwrap();
        assert_eq!(planned.ctx.subcompactions().len(), 1);
        assert_eq!(
            planned.ctx.subcompactions()[0].range(),
            &BytesRange::unbounded()
        );

        // given: a job carrying a persisted multi-range plan.
        let persisted = vec![
            Subcompaction::new(BytesRange::from_slice(..b"m".as_slice())),
            Subcompaction::new(BytesRange::from_slice(b"m".as_slice()..)),
        ];
        let args = StartCompactionJobArgs {
            ctx: Some(CompactionContext::new(persisted.clone(), Some(0))),
            ..args
        };

        // when/then: the persisted plan is returned verbatim.
        let planned = ctx.executor.inner.plan_compaction_job(args).await.unwrap();
        assert_eq!(planned.ctx.subcompactions(), &persisted);
    }

    /// The planner reads the real input SST indexes and splits a sizable
    /// multi-source compaction into several ranges (RFC-0028) when given no
    /// resume context. This exercises the index-sampling path end to end,
    /// unlike the tests that supply an explicit plan.
    #[tokio::test(flavor = "multi_thread")]
    async fn should_plan_split_from_sst_indexes() {
        let (executor, table_store, _rx) = subcompaction_env(
            "testdb-plan-from-indexes",
            #[cfg(feature = "compaction_filters")]
            None,
        )
        .await;
        let (l0_sst_views, sorted_runs) = split_inputs(&table_store).await;

        // given: a fresh job (no resume context) whose total input far exceeds
        // the per-source floor (the largest input SST's worth of bytes).
        let args = StartCompactionJobArgs {
            id: Ulid::new(),
            compaction_id: Ulid::new(),
            segment: Bytes::new(),
            destination: 0,
            l0_sst_views,
            sorted_runs,
            compaction_clock_tick: 0,
            is_dest_last_run: false,
                complete_input_through: None,
            retention_min_seq: Some(0),
            ctx: None,
        };

        // when: planning the job.
        let planned = executor.inner.plan_compaction_job(args).await.unwrap();

        // then: it splits into multiple ranges, capped at max_subcompactions.
        let n = planned.ctx.subcompactions().len();
        assert!(n > 1, "expected an index-driven split, got {n} range(s)");
        assert!(n <= 4, "must not exceed max_subcompactions");
    }

    /// A blocked SST `close()` (the object-store flush of a finished output SST)
    /// must not stall the merge loop. We gate the object store's `put` so the
    /// first output SST's flush blocks indefinitely, then assert the executor
    /// still makes progress (emits a progress message) while that flush is in
    /// flight. Before `close()` was pipelined off the loop, the loop would be
    /// parked inside `close()` and emit nothing until the flush completed.
    #[tokio::test(flavor = "multi_thread")]
    async fn should_stop_single_compaction_job_without_stopping_executor() {
        // given: an executor writing through a gated object store, with a tiny
        // max_sst_size so the first compaction parks while flushing output.
        let handle = tokio::runtime::Handle::current();
        let options = Arc::new(CompactionWorkerOptions {
            max_sst_size: 1,
            ..CompactionWorkerOptions::default()
        });
        let (tx, rx) = async_channel::unbounded::<WorkerMessage>();
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let gated = Arc::new(GatedObjectStore::new(inner));
        let object_store: Arc<dyn ObjectStore> = gated.clone();
        let root_path = Path::from("testdb-stop-single-job");
        let clock = Arc::new(DefaultSystemClock::new());
        let table_store = Arc::new(TableStore::new(
            object_store.clone(),
            SsTableFormat {
                block_size: 64,
                ..SsTableFormat::default()
            },
            root_path.clone(),
            None,
            TableStoreKind::Compactor,
            BlockCachePolicy::default(),
        ));
        let manifest_store = Arc::new(ManifestStore::new(&root_path, object_store.clone()));
        StoredManifest::create_new_db(manifest_store.clone(), ManifestCore::new(), clock.clone())
            .await
            .unwrap();

        let executor = TokioCompactionExecutor::new(TokioCompactionExecutorOptions {
            handle,
            options,
            worker_tx: tx,
            table_store: table_store.clone(),
            rand: Arc::new(DbRand::new(100u64)),
            stats: {
                let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();
                Arc::new(CompactionStats::new(&recorder))
            },
            worker_stats: WorkerStats::noop(),
            clock,
            manifest_store,
            merge_operator: None,
            #[cfg(feature = "compaction_filters")]
            compaction_filter_supplier: None,
        });

        let entries: Vec<RowEntry> = (0u64..64)
            .map(|i| {
                RowEntry::new_value(
                    format!("key{i:04}").as_bytes(),
                    format!("val{i:04}").as_bytes(),
                    i + 1,
                )
            })
            .collect();
        let input_ssts = write_sst(&table_store, &entries, usize::MAX).await;
        let l0_sst_views: Vec<SsTableView> =
            input_ssts.into_iter().map(SsTableView::identity).collect();

        let setup_puts = gated.put_opts_gate.arrivals();
        gated.put_opts_gate.close();

        // when: the first job runs and reaches a blocked output flush.
        let stopped_id = Ulid::new();
        executor.start_compaction_job(StartCompactionJobArgs {
            id: stopped_id,
            compaction_id: stopped_id,
            segment: Bytes::new(),
            destination: 0,
            l0_sst_views: l0_sst_views.clone(),
            sorted_runs: vec![],
            compaction_clock_tick: 0,
            is_dest_last_run: false,
                complete_input_through: None,
            retention_min_seq: Some(0),
            ctx: Some(CompactionContext::new(
                vec![Subcompaction::new(BytesRange::unbounded())],
                Some(0),
            )),
        });
        tokio::time::timeout(
            Duration::from_secs(5),
            gated.put_opts_gate.wait_for_arrivals(setup_puts + 1),
        )
        .await
        .expect("a close() should reach the blocked put gate");
        assert!(executor.inner.tasks.lock().contains_key(&stopped_id));

        // then: stopping that job clears executor bookkeeping immediately.
        assert!(executor.stop_compaction_job(stopped_id));
        assert!(executor.inner.tasks.lock().is_empty());
        assert!(!executor.stop_compaction_job(stopped_id));

        // when: the gate is reopened and a second job is started on the same
        // executor and destination.
        gated.put_opts_gate.release();
        let second_id = Ulid::new();
        executor.start_compaction_job(StartCompactionJobArgs {
            id: second_id,
            compaction_id: second_id,
            segment: Bytes::new(),
            destination: 0,
            l0_sst_views,
            sorted_runs: vec![],
            compaction_clock_tick: 0,
            is_dest_last_run: false,
                complete_input_through: None,
            retention_min_seq: Some(0),
            ctx: Some(CompactionContext::new(
                vec![Subcompaction::new(BytesRange::unbounded())],
                Some(0),
            )),
        });

        // then: the second job completes, and the stopped job never reports a
        // completion after cancellation won ownership of task accounting.
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let WorkerMessage::CompactionJobFinished { id, result } =
                    rx.recv().await.unwrap()
                {
                    assert_ne!(id, stopped_id, "stopped job reported completion");
                    if id == second_id {
                        return result;
                    }
                }
            }
        })
        .await
        .expect("second job should finish after the gate is released")
        .expect("second compaction should succeed");
        assert!(executor.inner.tasks.lock().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn should_keep_processing_while_sst_flush_is_blocked() {
        // given: an executor writing through a gated object store, with a tiny
        // max_sst_size so each finished block rolls into a new output SST
        // (guaranteeing several SST boundaries, and thus background closes).
        let handle = tokio::runtime::Handle::current();
        let options = Arc::new(CompactionWorkerOptions {
            max_sst_size: 1,
            ..CompactionWorkerOptions::default()
        });
        let (tx, rx) = async_channel::unbounded::<WorkerMessage>();
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let gated = Arc::new(GatedObjectStore::new(inner));
        let object_store: Arc<dyn ObjectStore> = gated.clone();
        let root_path = Path::from("testdb-flush-no-block");
        let clock = Arc::new(DefaultSystemClock::new());
        let table_store = Arc::new(TableStore::new(
            object_store.clone(),
            SsTableFormat {
                block_size: 64,
                ..SsTableFormat::default()
            },
            root_path.clone(),
            None,
            TableStoreKind::Compactor,
            BlockCachePolicy::default(),
        ));
        let manifest_store = Arc::new(ManifestStore::new(&root_path, object_store.clone()));
        StoredManifest::create_new_db(manifest_store.clone(), ManifestCore::new(), clock.clone())
            .await
            .unwrap();

        let executor = TokioCompactionExecutor::new(TokioCompactionExecutorOptions {
            handle,
            options,
            worker_tx: tx,
            table_store: table_store.clone(),
            rand: Arc::new(DbRand::new(100u64)),
            stats: {
                let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();
                Arc::new(CompactionStats::new(&recorder))
            },
            worker_stats: WorkerStats::noop(),
            clock,
            manifest_store,
            merge_operator: None,
            #[cfg(feature = "compaction_filters")]
            compaction_filter_supplier: None,
        });

        // given: a single L0 SST with enough rows to span many output SSTs.
        let entries: Vec<RowEntry> = (0u64..64)
            .map(|i| {
                RowEntry::new_value(
                    format!("key{i:04}").as_bytes(),
                    format!("val{i:04}").as_bytes(),
                    i + 1,
                )
            })
            .collect();
        let input_ssts = write_sst(&table_store, &entries, usize::MAX).await;
        let l0_sst_views: Vec<SsTableView> =
            input_ssts.into_iter().map(SsTableView::identity).collect();

        // given: the object-store flush (`put`) for output SSTs is blocked. Setup
        // writes above ran with the gate open (the default), so only the
        // compaction's output flushes are affected. The gate counts every call
        // (even pass-throughs), so baseline the setup puts to reason relatively.
        let setup_puts = gated.put_opts_gate.arrivals();
        gated.put_opts_gate.close();

        // when: the compaction job runs.
        executor.start_compaction_job(StartCompactionJobArgs {
            id: Ulid::new(),
            compaction_id: Ulid::new(),
            segment: Bytes::new(),
            destination: 0,
            l0_sst_views,
            sorted_runs: vec![],
            compaction_clock_tick: 0,
            is_dest_last_run: false,
                complete_input_through: None,
            retention_min_seq: Some(0),
            ctx: Some(CompactionContext::new(
                vec![Subcompaction::new(BytesRange::unbounded())],
                Some(0),
            )),
        });

        // when: the first output SST's flush is parked at the closed gate.
        tokio::time::timeout(
            Duration::from_secs(5),
            gated.put_opts_gate.wait_for_arrivals(setup_puts + 1),
        )
        .await
        .expect("a close() should reach the blocked put gate");

        // then: the loop keeps running and emits a progress message while the
        // flush is blocked. A `Finished` message here would mean the job somehow
        // completed despite the blocked flush.
        let progressed = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match rx.recv().await.unwrap() {
                    WorkerMessage::CompactionJobProgress {
                        bytes_processed, ..
                    } => {
                        if bytes_processed > 0 {
                            break;
                        }
                    }
                    WorkerMessage::CompactionJobFinished { .. } => {
                        panic!("job finished while its SST flush was blocked")
                    }
                    _ => {}
                }
            }
        })
        .await;
        assert!(
            progressed.is_ok(),
            "expected a progress message while the SST flush was blocked"
        );

        // then: depth-1 backpressure holds — only one close is in flight at a
        // time, since the loop parks at the next boundary draining it.
        assert_eq!(gated.put_opts_gate.arrivals(), setup_puts + 1);

        // when: the blocked flush is released.
        gated.put_opts_gate.release();
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let WorkerMessage::CompactionJobFinished { result, .. } =
                    rx.recv().await.unwrap()
                {
                    return result;
                }
            }
        })
        .await
        .expect("job should finish after the gate is released")
        .expect("compaction should succeed");

        // then: multiple output SSTs were produced (proving real boundaries and
        // background closes ran) ...
        let result_ssts = result.sst_views();
        assert!(
            result_ssts.len() >= 2,
            "expected multiple output SSTs, got {}",
            result_ssts.len()
        );
        // ... and the merged output preserves every key in ascending order.
        let mut read_back = Vec::new();
        for view in result_ssts.iter() {
            let mut iter = SstIterator::new(
                SstView::Owned(
                    Box::new(SsTableView::identity(view.sst.clone())),
                    BytesRange::from(..),
                ),
                table_store.clone(),
                SstIteratorOptions::default(),
            )
            .unwrap();
            iter.init().await.unwrap();
            while let Some(entry) = iter.next().await.unwrap() {
                read_back.push(entry.key);
            }
        }
        let expected: Vec<Bytes> = entries.iter().map(|e| e.key.clone()).collect();
        assert_eq!(read_back, expected);
    }

    /// Test context for compaction executor tests.
    struct TestContext {
        executor: TokioCompactionExecutor,
        table_store: Arc<TableStore>,
        rx: async_channel::Receiver<WorkerMessage>,
    }

    /// Builder for creating test context with configurable options.
    struct TestContextBuilder {
        path: String,
        merge_operator: Option<MergeOperatorType>,
        #[cfg(feature = "compaction_filters")]
        compaction_filter_supplier: Option<Arc<dyn CompactionFilterSupplier>>,
    }

    impl TestContextBuilder {
        fn new(path: &str) -> Self {
            Self {
                path: path.to_string(),
                merge_operator: None,
                #[cfg(feature = "compaction_filters")]
                compaction_filter_supplier: None,
            }
        }

        fn with_merge_operator(mut self, merge_operator: MergeOperatorType) -> Self {
            self.merge_operator = Some(merge_operator);
            self
        }

        #[cfg(feature = "compaction_filters")]
        fn with_compaction_filter_supplier(
            mut self,
            supplier: Arc<dyn CompactionFilterSupplier>,
        ) -> Self {
            self.compaction_filter_supplier = Some(supplier);
            self
        }

        async fn build(self) -> TestContext {
            let handle = tokio::runtime::Handle::current();
            let options = Arc::new(CompactionWorkerOptions::default());
            let (tx, rx) = async_channel::unbounded();
            let os = Arc::new(InMemory::new());
            let clock = Arc::new(DefaultSystemClock::new());
            let db = Db::builder(self.path.clone(), os.clone())
                .with_system_clock(clock.clone())
                .build()
                .await
                .unwrap();
            let table_store = db.inner.table_store.clone();
            let manifest_store = Arc::new(ManifestStore::new(
                &Path::from(self.path.as_str()),
                os.clone(),
            ));

            let executor = TokioCompactionExecutor::new(TokioCompactionExecutorOptions {
                handle,
                options,
                worker_tx: tx,
                table_store: table_store.clone(),
                rand: Arc::new(DbRand::new(100u64)),
                stats: {
                    let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();
                    Arc::new(CompactionStats::new(&recorder))
                },
                worker_stats: WorkerStats::noop(),
                clock,
                manifest_store,
                merge_operator: self.merge_operator,
                #[cfg(feature = "compaction_filters")]
                compaction_filter_supplier: self.compaction_filter_supplier,
            });

            TestContext {
                executor,
                table_store,
                rx,
            }
        }
    }

    impl TestContext {
        /// Runs a compaction job and waits for the result.
        async fn run_compaction(
            self,
            ssts: Vec<SsTableHandle>,
            is_dest_last_run: bool,
            retention_min_seq: Option<u64>,
        ) -> Result<SortedRun, SlateDBError> {
            let compaction = StartCompactionJobArgs {
                id: Ulid::new(),
                compaction_id: Ulid::new(),
                segment: Bytes::new(),
                destination: 0,
                l0_sst_views: ssts.into_iter().map(SsTableView::identity).collect(),
                sorted_runs: vec![],
                compaction_clock_tick: 0,
                is_dest_last_run,
                complete_input_through: None,
                retention_min_seq,
                ctx: Some(CompactionContext::new(
                    vec![Subcompaction::new(BytesRange::unbounded())],
                    retention_min_seq,
                )),
            };
            self.executor.start_compaction_job(compaction);

            tokio::time::timeout(Duration::from_secs(5), async move {
                loop {
                    let msg = self.rx.recv().await.unwrap();
                    if let WorkerMessage::CompactionJobFinished { id: _, result } = msg {
                        return result;
                    }
                }
            })
            .await
            .unwrap()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_compaction_job_should_retain_merges_newer_than_retention_min_seq_num() {
        let ctx = TestContextBuilder::new("testdb")
            .with_merge_operator(Arc::new(StringConcatMergeOperator {}))
            .build()
            .await;
        let table_store = ctx.table_store.clone();

        // write some merges
        let mut sst_builder = table_store.table_builder();
        sst_builder
            .add(RowEntry::new_merge(b"foo", b"3", 4))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_merge(b"foo", b"2", 3))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_merge(b"foo", b"1", 2))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_merge(b"foo", b"0", 1))
            .await
            .unwrap();
        let encoded_sst = sst_builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        let l0 = table_store
            .write_sst(&id, &encoded_sst, Some(Bytes::new()))
            .await
            .unwrap();
        let retention_min_seq_num = 2;

        let result = ctx
            .run_compaction(vec![l0], false, Some(retention_min_seq_num))
            .await
            .unwrap();

        assert_eq!(1, result.sst_views().len());
        let sst = result.sst_views()[0].clone();
        let mut iter = SstIterator::new(
            SstView::Borrowed(&sst, BytesRange::from(..)),
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .unwrap();
        iter.init().await.unwrap();
        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next.key, Bytes::from(b"foo".as_slice()));
        assert_eq!(
            next.value,
            ValueDeletable::Merge(Bytes::from(b"3".as_slice()))
        );
        assert_eq!(next.seq, retention_min_seq_num + 2);
        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next.key, Bytes::from(b"foo".as_slice()));
        assert_eq!(
            next.value,
            ValueDeletable::Merge(Bytes::from(b"2".as_slice()))
        );
        assert_eq!(next.seq, retention_min_seq_num + 1);
        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next.key, Bytes::from(b"foo".as_slice()));
        assert_eq!(
            next.value,
            ValueDeletable::Merge(Bytes::from(b"01".as_slice()))
        );
        assert_eq!(next.seq, retention_min_seq_num);
        assert!(iter.next().await.unwrap().is_none());
    }

    #[cfg(feature = "compaction_filters")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_compaction_job_with_filter_success() {
        use crate::compaction_filter::{
            CompactionFilter, CompactionFilterDecision, CompactionFilterError,
            CompactionFilterSupplier, CompactionJobContext,
        };

        /// A filter that tests all decision types:
        /// - "drop:" prefix -> Drop (remove entry)
        /// - "modify:" prefix -> Modify (append "_modified" suffix)
        /// - "tombstone:" prefix -> Modify to Tombstone
        /// - other prefixes -> Keep (unchanged)
        struct TestFilter;

        #[async_trait::async_trait]
        impl CompactionFilter for TestFilter {
            async fn filter(
                &mut self,
                entry: &RowEntry,
            ) -> Result<CompactionFilterDecision, CompactionFilterError> {
                if entry.key.starts_with(b"drop:") {
                    Ok(CompactionFilterDecision::Drop)
                } else if entry.key.starts_with(b"modify:") {
                    if let Some(value) = entry.value.as_bytes() {
                        let mut new_value = value.to_vec();
                        new_value.extend_from_slice(b"_modified");
                        Ok(CompactionFilterDecision::Modify(ValueDeletable::Value(
                            Bytes::from(new_value),
                        )))
                    } else {
                        Ok(CompactionFilterDecision::Keep)
                    }
                } else if entry.key.starts_with(b"tombstone:") {
                    Ok(CompactionFilterDecision::Modify(ValueDeletable::Tombstone))
                } else {
                    Ok(CompactionFilterDecision::Keep)
                }
            }

            async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
                Ok(())
            }
        }

        struct TestFilterSupplier;

        #[async_trait::async_trait]
        impl CompactionFilterSupplier for TestFilterSupplier {
            async fn create_compaction_filter(
                &self,
                _context: &CompactionJobContext,
            ) -> Result<Box<dyn CompactionFilter>, CompactionFilterError> {
                Ok(Box::new(TestFilter))
            }
        }

        let ctx = TestContextBuilder::new("testdb_filter_all_decisions")
            .with_compaction_filter_supplier(Arc::new(TestFilterSupplier))
            .build()
            .await;
        let table_store = ctx.table_store.clone();

        // Write entries with different prefixes
        // Note: entries must be added in sorted key order
        // Lexicographic order: drop: < keep: < modify: < tombstone:
        let mut sst_builder = table_store.table_builder();
        sst_builder
            .add(RowEntry::new_value(b"drop:key1", b"value1", 1))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_value(b"drop:key2", b"value2", 2))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_value(b"keep:key3", b"value3", 3))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_value(b"keep:key4", b"value4", 4))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_value(b"modify:key5", b"value5", 5))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_value(b"modify:key6", b"value6", 6))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_value(b"tombstone:key7", b"value7", 7))
            .await
            .unwrap();
        sst_builder
            .add(RowEntry::new_value(b"tombstone:key8", b"value8", 8))
            .await
            .unwrap();
        let encoded_sst = sst_builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        let l0 = table_store
            .write_sst(&id, &encoded_sst, Some(Bytes::new()))
            .await
            .unwrap();

        let result = ctx.run_compaction(vec![l0], true, None).await.unwrap();

        // Verify the output SST
        assert_eq!(1, result.sst_views().len());
        let sst = result.sst_views()[0].clone();
        let mut iter = SstIterator::new(
            SstView::Borrowed(&sst, BytesRange::from(..)),
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .unwrap();
        iter.init().await.unwrap();

        // drop:key1 and drop:key2 should be DROPPED (not present)

        // keep:key3 - unchanged (Keep decision)
        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next.key, Bytes::from(b"keep:key3".as_slice()));
        assert_eq!(
            next.value,
            ValueDeletable::Value(Bytes::from(b"value3".as_slice()))
        );

        // keep:key4 - unchanged (Keep decision)
        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next.key, Bytes::from(b"keep:key4".as_slice()));
        assert_eq!(
            next.value,
            ValueDeletable::Value(Bytes::from(b"value4".as_slice()))
        );

        // modify:key5 - value modified (Modify decision)
        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next.key, Bytes::from(b"modify:key5".as_slice()));
        assert_eq!(
            next.value,
            ValueDeletable::Value(Bytes::from(b"value5_modified".as_slice()))
        );

        // modify:key6 - value modified (Modify decision)
        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next.key, Bytes::from(b"modify:key6".as_slice()));
        assert_eq!(
            next.value,
            ValueDeletable::Value(Bytes::from(b"value6_modified".as_slice()))
        );

        // tombstone:key7 - converted to tombstone (Modify to Tombstone decision)
        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next.key, Bytes::from(b"tombstone:key7".as_slice()));
        assert!(next.value.is_tombstone());

        // tombstone:key8 - converted to tombstone (Modify to Tombstone decision)
        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next.key, Bytes::from(b"tombstone:key8".as_slice()));
        assert!(next.value.is_tombstone());

        // No more entries
        assert!(iter.next().await.unwrap().is_none());
    }

    #[cfg(feature = "compaction_filters")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_compaction_job_aborts_on_filter_creation_error() {
        use crate::compaction_filter::{
            CompactionFilter, CompactionFilterError, CompactionFilterSupplier, CompactionJobContext,
        };

        struct FailingFilterSupplier;

        #[async_trait::async_trait]
        impl CompactionFilterSupplier for FailingFilterSupplier {
            async fn create_compaction_filter(
                &self,
                _context: &CompactionJobContext,
            ) -> Result<Box<dyn CompactionFilter>, CompactionFilterError> {
                Err(CompactionFilterError::CreationError(
                    "intentional failure".into(),
                ))
            }
        }

        let ctx = TestContextBuilder::new("testdb_filter_fail")
            .with_compaction_filter_supplier(Arc::new(FailingFilterSupplier))
            .build()
            .await;
        let table_store = ctx.table_store.clone();

        // Write a simple entry
        let mut sst_builder = table_store.table_builder();
        sst_builder
            .add(RowEntry::new_value(b"key1", b"value1", 1))
            .await
            .unwrap();
        let encoded_sst = sst_builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        let l0 = table_store
            .write_sst(&id, &encoded_sst, Some(Bytes::new()))
            .await
            .unwrap();

        let result = ctx.run_compaction(vec![l0], true, None).await;

        // The compaction should have failed
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SlateDBError::CompactionFilterError(_)));
    }

    #[cfg(feature = "compaction_filters")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_compaction_job_aborts_on_compaction_end_error() {
        use crate::compaction_filter::{
            CompactionFilter, CompactionFilterDecision, CompactionFilterError,
            CompactionFilterSupplier, CompactionJobContext,
        };

        /// A filter that keeps all entries but fails on on_compaction_end.
        struct FailOnEndFilter;

        #[async_trait::async_trait]
        impl CompactionFilter for FailOnEndFilter {
            async fn filter(
                &mut self,
                _entry: &RowEntry,
            ) -> Result<CompactionFilterDecision, CompactionFilterError> {
                Ok(CompactionFilterDecision::Keep)
            }

            async fn on_compaction_end(&mut self) -> Result<(), CompactionFilterError> {
                Err(CompactionFilterError::CompactionEndError(
                    "intentional failure on compaction end".into(),
                ))
            }
        }

        struct FailOnEndFilterSupplier;

        #[async_trait::async_trait]
        impl CompactionFilterSupplier for FailOnEndFilterSupplier {
            async fn create_compaction_filter(
                &self,
                _context: &CompactionJobContext,
            ) -> Result<Box<dyn CompactionFilter>, CompactionFilterError> {
                Ok(Box::new(FailOnEndFilter))
            }
        }

        let ctx = TestContextBuilder::new("testdb_filter_end_fail")
            .with_compaction_filter_supplier(Arc::new(FailOnEndFilterSupplier))
            .build()
            .await;
        let table_store = ctx.table_store.clone();

        // Write a simple entry
        let mut sst_builder = table_store.table_builder();
        sst_builder
            .add(RowEntry::new_value(b"key1", b"value1", 1))
            .await
            .unwrap();
        let encoded_sst = sst_builder.build().await.unwrap();
        let id = SsTableId::from(Ulid::new());
        let l0 = table_store
            .write_sst(&id, &encoded_sst, Some(Bytes::new()))
            .await
            .unwrap();

        let result = ctx.run_compaction(vec![l0], true, None).await;

        // The compaction should have failed due to on_compaction_end error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, SlateDBError::CompactionFilterError(_)),
            "Expected CompactionFilterError, got: {:?}",
            err
        );
    }
}
