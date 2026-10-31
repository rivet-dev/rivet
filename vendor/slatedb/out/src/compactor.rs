// Local leadership/lifecycle modifications; see LEADERSHIP.md in this package.
//! Compactor Orchestration and Naming Overview
//!
//! This module implements the event loop ("compactor") that orchestrates database
//! compactions. It coordinates three concerns:
//!
//! 1. deciding what to compact,
//! 2. executing the work,
//! 3. and persisting the effects.
//!
//! The key types follow a consistent naming hierarchy to distinguish “what should happen”
//! from “what is currently running”.
//!
//! Naming hierarchy (logical → runtime):
//! - [`CompactionSpec`]: Description of a compaction to perform. Pure specification of
//!   inputs and destination (e.g., L0 SST ULIDs and/or Sorted Run ids → destination SR id).
//! - [`Compaction`]: A concrete compaction entity the system decided to run. It has a
//!   stable id (ULID) and references a [`CompactionSpec`]. Compactions live in the
//!   process-local [`CompactorState`] and represent the lifecycle of a single planned
//!   transformation of the DB’s structure. Today each [`Compaction`] has exactly one
//!   runtime job; future retries would attach multiple jobs to a single [`Compaction`].
//! - Compaction Job: A single job attempt the executor runs. Currently there isn't a separate model
//!   type that tracks the Compaction Job entity. When starting a job the compactor creates a
//!   [`StartCompactionJobArgs`] instance which materializes the execution-time inputs (opened
//!   SST handles and Sorted Runs), carries runtime context like the logical compaction clock tick,
//!   and sends this to the executor.
//!   Currently there is a 1:1 mapping between [`Compaction`] and a Compaction Job, but
//!   if retries are introduced, multiple Compaction Jobs would be associated with a single
//!   [`Compaction`].
//!
//! Roles:
//! - [`CompactionScheduler`]: Policy that proposes [`CompactionSpec`] values based on the
//!   current [`CompactorState`] (e.g., size-tiered scheduling). It validates logical
//!   invariants (such as consecutive sources and destination rules).
//! - [`CompactionExecutor`]: Runs [`CompactionJobSpec`] attempts, merges inputs, applies
//!   merge and retention logic, writes output SSTs, and reports progress/finish.
//! - [`CompactorEventHandler`]: Event-driven controller. It polls the manifest, asks
//!   the scheduler for new [`CompactionSpec`] proposals, registers [`Compaction`] entities,
//!   spawns [`CompactionJobSpec`] attempts on the executor, and persists results by
//!   updating the manifest.
//!
//! Progress and GC safety:
//! - Progress is tracked per compaction as `bytes_processed` (approximate, for observability).
//! - The lowest start time among active compaction ids (ULIDs) is exported as a
//!   “low-watermark” hint so GC can avoid deleting inputs required by running work.
//!
//! High-level flow:
//! 1) Poll manifest and merge remote state into local [`CompactorState`].
//! 2) Ask the [`CompactionScheduler`] for candidate [`CompactionSpec`] values.
//! 3) For each accepted spec, create a [`Compaction`] (id + spec), derive a
//!    [`StartCompactionJobArgs`] with concrete inputs, and pass it to the executor.
//! 4) Upon completion, update [`CompactorState`], write a new manifest, and repeat.
//!
//! These names are used consistently across modules so it is clear whether a type
//! represents a description (Spec), a durable decision (Compaction), or a running
//! attempt (JobSpec).

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use fail_parallel::FailPointRegistry;
use futures::stream::BoxStream;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tracing::instrument;
use ulid::Ulid;

#[cfg(feature = "compaction_filters")]
use crate::compaction_filter::CompactionFilterSupplier;
use crate::compaction_worker::CompactionWorkerHandler;
use crate::compactions_store::{CompactionsStore, StoredCompactions};
use crate::compactor::stats::CompactionStats;
use crate::compactor_state_protocols::CompactorStateWriter;
use crate::config::CompactorOptions;
use crate::db_state::{SortedRun, SsTableView};
use crate::db_status::ClosedResultWriter;
use crate::dispatcher::{MessageHandler, MessageHandlerExecutor, MessageTickerDef, Notifier};
use crate::error::{Error, SlateDBError};
use crate::manifest::store::ManifestStore;
use crate::manifest::{LsmTreeState, ManifestCore};
use crate::merge_operator::MergeOperatorType;
use crate::sequenced_store::{WriteNotifier, MIN_WRITE_NOTICE_INTERVAL};
use crate::tablestore::TableStore;
use crate::utils::{format_bytes_si, IdGenerator};
use slatedb_common::clock::SystemClock;
use slatedb_common::metrics::{GaugeFn, MetricsRecorderHelper};
use slatedb_common::DbRand;

pub use crate::compactor_state::{
    Compaction, CompactionSpec, CompactionStatus, CompactorState, SourceId,
};
pub use crate::compactor_state_protocols::CompactorStateView;
pub use crate::db::builder::CompactorBuilder;
pub use crate::size_tiered_compaction::SizeTieredCompactionSchedulerSupplier;

pub(crate) const COMPACTOR_TASK_NAME: &str = "compactor";

/// Supplies a concrete [`CompactionScheduler`] implementation.
///
/// This indirection lets SlateDB plug different scheduling policies (e.g. size-tiered,
/// leveled, manual) without coupling the compactor to a specific strategy.
pub trait CompactionSchedulerSupplier: Send + Sync {
    /// Creates a new [`CompactionScheduler`] using the provided options.
    ///
    /// ## Arguments
    /// - `options`: Compactor runtime tuning knobs (e.g. concurrency, polling interval).
    ///
    /// ## Returns
    /// - A boxed scheduler that decides which compactions to run next.
    fn compaction_scheduler(
        &self,
        options: &CompactorOptions,
    ) -> Box<dyn CompactionScheduler + Send + Sync>;
}

/// Policy that decides when and what to compact.
///
/// The compactor periodically invokes the scheduler with the latest [`CompactorState`].
/// Implementations return one or more candidate compaction specs, which the compactor then
/// validates and submits to the executor.
pub trait CompactionScheduler: Send + Sync {
    /// Proposes compaction specs for the current state.
    ///
    /// ## Arguments
    /// - `state`: Process-local view of the DB's manifest and compactions.
    ///
    /// ## Returns
    /// - A list of [`CompactionSpec`] describing what to compact and where to write.
    fn propose(&self, state: &CompactorStateView) -> Vec<CompactionSpec>;

    /// Validates a candidate compaction spec against scheduler-specific invariants. The
    /// default implementation accepts everything. Schedulers can override to enforce
    /// policy-specific constraints prior to execution (e.g. level rules, overlaps).
    ///
    /// Not all compactions have to originate from scheduler. Admin/“manual” compactions,
    /// persisted/resumed plans, or tests can construct a [`CompactionSpec`]`. This
    /// method can be used to validate external compaction specs against scheduler
    /// specific invariants.
    ///
    /// ## Arguments
    /// - `_state`: Process-local view of the DB's manifest and compactions.
    /// - `_spec`: Proposed [`CompactionSpec`].
    ///
    /// ## Returns
    /// - `Ok(())` if valid, or an [`Error`] if invalid.
    fn validate(&self, _state: &CompactorStateView, _spec: &CompactionSpec) -> Result<(), Error> {
        Ok(())
    }

    /// Generate one or more compaction specs from a [`CompactionRequest`].
    ///
    /// - [`CompactionRequest::Spec`] returns the caller-provided spec verbatim.
    /// - [`CompactionRequest::FullSegment`] targets a single tree (root or a
    ///   named segment): every sorted run becomes a source, the tree's lowest
    ///   sorted-run id is the destination. Returns an error if the segment
    ///   is unknown or the tree has no compacted sorted runs.
    /// - [`CompactionRequest::Full`] applies the same per-tree rule to every
    ///   tree in the DB (root + every named segment), best-effort: trees
    ///   without compacted sorted runs are silently skipped and an empty
    ///   `Vec` is returned if no tree is eligible.
    ///
    /// L0 SSTs are intentionally excluded so the size-tiered scheduler can
    /// keep compacting new flush output in parallel. Including L0 would
    /// mark those SSTs as in-use for the duration of the compaction,
    /// eventually stalling writers once L0 reaches `l0_max_ssts`.
    ///
    /// ## Arguments
    /// - `state`: Process-local view of the DB's manifest and compactions.
    /// - `request`: The compaction request to plan.
    ///
    /// ## Returns
    /// - A list of [`CompactionSpec`] describing what to compact and where to write.
    /// - An [`Error`] if the request is invalid.
    fn generate(
        &self,
        state: &CompactorStateView,
        request: &CompactionRequest,
    ) -> Result<Vec<CompactionSpec>, Error> {
        match request {
            CompactionRequest::Spec(spec) => Ok(vec![spec.clone()]),
            CompactionRequest::FullSegment { segment } => {
                let manifest = state.manifest().core();
                let Some(tree) = manifest.tree_for_segment(segment) else {
                    error!(
                        "rejected full-segment compaction: unknown segment {:?}",
                        segment
                    );
                    return Err(Error::from(SlateDBError::InvalidCompaction));
                };
                match plan_full_tree(segment, tree) {
                    Some(spec) => Ok(vec![spec]),
                    None => {
                        if !tree.l0.is_empty() {
                            error!(
                                "rejected full-segment compaction: segment {:?} has L0 SSTs but no sorted runs to merge; \
                                 L0 SSTs are not eligible inputs",
                                segment
                            );
                        }
                        Err(Error::from(SlateDBError::InvalidCompaction))
                    }
                }
            }
            CompactionRequest::Full => {
                let manifest = state.manifest().core();
                let specs = manifest
                    .trees_with_prefix()
                    .filter_map(|(prefix, tree)| plan_full_tree(&prefix, tree))
                    .collect();
                Ok(specs)
            }
        }
    }
}

/// Build a [`CompactionSpec`] that compacts every sorted run in `tree` into
/// the tree's lowest-id sorted run. Returns `None` when the tree has no
/// compacted sorted runs — L0 SSTs are not eligible inputs.
fn plan_full_tree(prefix: &Bytes, tree: &LsmTreeState) -> Option<CompactionSpec> {
    if tree.compacted.is_empty() {
        return None;
    }
    let sources = tree
        .compacted
        .iter()
        .map(|sr| SourceId::SortedRun(sr.id))
        .collect::<Vec<_>>();
    let destination = sources
        .iter()
        .map(|s| s.unwrap_sorted_run())
        .min()
        .expect("at least one sorted run");
    Some(CompactionSpec::for_segment(
        prefix.clone(),
        sources,
        destination,
    ))
}

/// Request to submit a compaction for execution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CompactionRequest {
    /// Compact every tree in the DB — the root tree plus each named segment
    /// (RFC-0024) — by merging every sorted run in each tree into that
    /// tree's lowest-id sorted run. Trees with no compacted sorted runs are
    /// silently skipped; an empty `Vec` is returned when no tree is eligible.
    ///
    /// L0 SSTs are left to regular size-tiered compaction so memtable flushes
    /// can continue producing new sorted runs while this compaction is in
    /// flight. Pulling L0 into the merge would mark those SSTs as in-use
    /// for the duration of the compaction, eventually stalling writers once
    /// L0 reaches `l0_max_ssts`.
    Full,
    /// Compact a single tree: every sorted run in the tree at `segment` is
    /// merged into the tree's lowest-id sorted run. `segment: Bytes::new()`
    /// targets the compatibility-encoded root tree (the only valid segment
    /// on a database without a segment extractor); a non-empty prefix names
    /// a segment. Rejected if the segment is unknown or the tree has no
    /// compacted sorted runs. L0 SSTs are excluded for the same reason
    /// documented on `Full`.
    FullSegment { segment: Bytes },
    /// Use a caller-provided compaction spec.
    Spec(CompactionSpec),
}

/// Messages exchanged between the compactor event loop and its collaborators.
#[derive(Debug)]
pub(crate) enum CompactorMessage {
    /// Ticker-triggered message to log DB runs and in-flight job state.
    LogStats,
    /// Ticker-triggered message to refresh the manifest and schedule compactions.
    PollManifest,
    /// Fast ticker to commit any `Compacted` entries written by workers without
    /// waiting for the full `poll_interval` cycle.
    CommitCompacted,
    /// A component sharing this process's manifest store, such as the database writer
    /// after a flush, wrote manifest version `id`.
    ManifestWritten { id: u64 },
}

/// The compactor is responsible for taking groups of sorted runs (this doc uses the term
/// sorted run to refer to both sorted runs and l0 ssts) and compacting them together to
/// reduce space amplification (by removing old versions of rows that have been updated/deleted)
/// and read amplification (by reducing the number of sorted runs that need to be searched on
/// a read). It's made up of a few different components:
///
/// - [`Compactor`]: The main event loop that orchestrates the compaction process.
/// - [`CompactorEventHandler`]: The event handler that handles events from the compactor.
/// - [`CompactionScheduler`]: The scheduler that discovers compactions that should be performed.
/// - [`CompactionExecutor`]: The executor that runs the compaction tasks.
///
/// The main event loop listens on the manifest poll ticker to react to manifest poll ticks, the
/// executor worker channel to react to updates about running compactions, and the shutdown
/// channel to discover when it should terminate. It doesn't actually implement the logic for
/// reacting to these events. This is implemented by [`CompactorEventHandler`].
///
/// The Scheduler is responsible for deciding what sorted runs should be compacted together.
/// It implements the [`CompactionScheduler`] trait. The implementation is specified by providing
/// an implementation of [`crate::config::CompactionSchedulerSupplier`] so different scheduling
/// policies can be plugged into slatedb. Currently, the only implemented policy is the size-tiered
/// scheduler supplied by [`crate::size_tiered_compaction::SizeTieredCompactionSchedulerSupplier`]
///
/// The Executor does the actual work of compacting sorted runs by sort-merging them into a new
/// sorted run. It implements the [`CompactionExecutor`] trait. Currently, the only implementation
/// is the [`TokioCompactionExecutor`] which runs compaction on a local tokio runtime.
#[derive(Clone)]
pub struct Compactor {
    manifest_store: Arc<ManifestStore>,
    compactions_store: Arc<CompactionsStore>,
    table_store: Arc<TableStore>,
    options: Arc<CompactorOptions>,
    scheduler_supplier: Arc<dyn CompactionSchedulerSupplier>,
    task_executor: Arc<MessageHandlerExecutor>,
    compactor_runtime: Handle,
    rand: Arc<DbRand>,
    stats: Arc<CompactionStats>,
    recorder: MetricsRecorderHelper,
    system_clock: Arc<dyn SystemClock>,
    fp_registry: Arc<FailPointRegistry>,
    merge_operator: Option<MergeOperatorType>,
    #[cfg(feature = "compaction_filters")]
    compaction_filter_supplier: Option<Arc<dyn CompactionFilterSupplier>>,
}

impl Compactor {
    #[cfg_attr(not(feature = "compaction_filters"), allow(clippy::too_many_arguments))]
    pub(crate) fn new(
        manifest_store: Arc<ManifestStore>,
        compactions_store: Arc<CompactionsStore>,
        table_store: Arc<TableStore>,
        options: CompactorOptions,
        scheduler_supplier: Arc<dyn CompactionSchedulerSupplier>,
        compactor_runtime: Handle,
        rand: Arc<DbRand>,
        recorder: &MetricsRecorderHelper,
        system_clock: Arc<dyn SystemClock>,
        fp_registry: Arc<FailPointRegistry>,
        closed_result: Arc<dyn ClosedResultWriter>,
        merge_operator: Option<MergeOperatorType>,
        #[cfg(feature = "compaction_filters")] compaction_filter_supplier: Option<
            Arc<dyn CompactionFilterSupplier>,
        >,
    ) -> Self {
        let stats = Arc::new(CompactionStats::new(recorder));
        let task_executor = Arc::new(MessageHandlerExecutor::new(
            closed_result,
            system_clock.clone(),
        ));
        Self {
            manifest_store,
            compactions_store,
            table_store,
            options: Arc::new(options),
            scheduler_supplier,
            task_executor,
            compactor_runtime,
            rand,
            stats,
            recorder: recorder.clone(),
            system_clock,
            fp_registry,
            merge_operator,
            #[cfg(feature = "compaction_filters")]
            compaction_filter_supplier,
        }
    }

    /// Starts the compactor. This method runs the actual compaction event loop. The
    /// compactor runs until the cancellation token is cancelled. The compactor's event
    /// loop always runs on the current runtime, while the compactor executor runs on the
    /// provided runtime in the [`Compactor::new`] constructor. This is to keep
    /// long-running compaction tasks from blocking the main runtime.
    ///
    /// ## Returns
    /// - `Ok(())` when the compactor task exits cleanly, or [`SlateDBError`] on failure.
    pub async fn run(&self) -> Result<(), Error> {
        self.start().await?;
        self.join().await
    }

    pub(crate) async fn start(&self) -> Result<(), Error> {
        // The coordinator delegates compaction execution to [`crate::compaction_worker::CompactionWorker`]
        // either spawned in this process (set `worker: Some`) or running standalone (set `worker: None`).
        let (_tx, rx) = async_channel::unbounded::<CompactorMessage>();
        let scheduler = Arc::from(self.scheduler_supplier.compaction_scheduler(&self.options));
        let handler = CompactorEventHandler::new(
            self.manifest_store.clone(),
            self.compactions_store.clone(),
            self.options.clone(),
            scheduler,
            self.rand.clone(),
            self.stats.clone(),
            self.system_clock.clone(),
            self.recorder.clone(),
        )
        .await?;
        self.task_executor
            .add_handler(
                COMPACTOR_TASK_NAME.to_string(),
                Box::new(handler),
                rx,
                &Handle::current(),
            )
            .map_err(Error::from)?;

        // Spawn an in-process worker if configured. The worker runs under its
        // own cancellation token; Compactor::stop and run() are responsible for
        // shutting it down alongside the coordinator.
        if let Some(worker_options) = self.options.worker.clone() {
            let (worker_handler, worker_rx) = CompactionWorkerHandler::build_worker_handler(
                self.manifest_store.clone(),
                self.compactions_store.clone(),
                self.table_store.clone(),
                Arc::new(worker_options),
                self.compactor_runtime.clone(),
                self.rand.clone(),
                self.stats.clone(),
                self.recorder.clone(),
                self.system_clock.clone(),
                self.fp_registry.clone(),
                self.merge_operator.clone(),
                #[cfg(feature = "compaction_filters")]
                self.compaction_filter_supplier.clone(),
            );
            self.task_executor
                .add_handler(
                    crate::compaction_worker::COMPACTION_WORKER_TASK_NAME.to_string(),
                    Box::new(worker_handler),
                    worker_rx,
                    &Handle::current(),
                )
                .map_err(Error::from)?;
        }

        self.task_executor.monitor_on(&Handle::current())?;
        Ok(())
    }

    pub(crate) async fn join(&self) -> Result<(), Error> {
        self.task_executor
            .join_task(COMPACTOR_TASK_NAME)
            .await
            .map_err(Error::from)?;
        if self.options.worker.is_some() {
            self.task_executor
                .join_task(crate::compaction_worker::COMPACTION_WORKER_TASK_NAME)
                .await
                .map_err(Error::from)?;
        }
        Ok(())
    }

    /// Gracefully stops the compactor task and waits for it to finish.
    ///
    /// ## Returns
    /// - `Ok(())` once the task has shut down, or [`SlateDBError`] if shutdown fails.
    pub async fn stop(&self) -> Result<(), Error> {
        self.task_executor
            .shutdown_task(COMPACTOR_TASK_NAME)
            .await
            .map_err(Error::from)?;
        if self.options.worker.is_some() {
            self.task_executor
                .shutdown_task(crate::compaction_worker::COMPACTION_WORKER_TASK_NAME)
                .await
                .map_err(Error::from)?;
        }
        Ok(())
    }

    /// Persist a [`CompactionSpec`] as a new [`Compaction`] in the compactions store.
    ///
    /// ## Returns
    /// - `Ok(Ulid)` with the submitted compaction id.
    /// - `SlateDBError::InvalidDBState` if the compactions file doesn't exist.
    /// - `SlateDBError::InvalidCompaction` if a full compaction has no sorted run sources.
    /// - `SlateDBError` if the compaction could not be persisted.
    pub(crate) async fn submit(
        spec: CompactionSpec,
        compactions_store: Arc<CompactionsStore>,
        rand: Arc<DbRand>,
        system_clock: Arc<dyn SystemClock>,
    ) -> Result<Ulid, Error> {
        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let compaction = Compaction::new(compaction_id, spec);
        let mut stored_compactions =
            match StoredCompactions::try_load(compactions_store.clone()).await? {
                Some(stored) => stored,
                None => return Err(Error::from(SlateDBError::InvalidDBState)),
            };

        loop {
            let mut dirty = stored_compactions.prepare_dirty()?;
            dirty.value.insert(compaction.clone());
            match stored_compactions.update(dirty).await {
                Ok(()) => return Ok(compaction_id),
                Err(err) if err.is_sequenced_write_conflict() => {
                    stored_compactions.refresh().await?;
                }
                Err(err) => return Err(Error::from(err)),
            }
        }
    }
}

/// Event-driven controller for compaction orchestration.
///
/// The [`CompactorEventHandler`] implements [`MessageHandler<CompactorMessage>`] and
/// runs inside the compactor task's message loop. It reacts to periodic tickers and
/// executor callbacks, and coordinates scheduling and execution of compactions.
pub(crate) struct CompactorEventHandler {
    state_writer: CompactorStateWriter,
    options: Arc<CompactorOptions>,
    scheduler: Arc<dyn CompactionScheduler + Send + Sync>,
    rand: Arc<DbRand>,
    stats: Arc<CompactionStats>,
    system_clock: Arc<dyn SystemClock>,
    recorder: MetricsRecorderHelper,
    /// Job ids the coordinator has observed claimed by a worker (`Running` or
    /// `Compacted` with a worker assigned). Initially empty, so the first metrics
    /// update counts every inherited claim before establishing the set-difference
    /// baseline. See [`Self::update_distributed_compaction_metrics`].
    prev_claimed: HashSet<Ulid>,
    /// Cached handles for per-worker `worker_last_heartbeat_ms` gauges. Handles are
    /// retained for every worker id observed by this coordinator process.
    worker_heartbeat_gauges: HashMap<String, Arc<dyn GaugeFn>>,
}

#[async_trait]
impl MessageHandler<CompactorMessage> for CompactorEventHandler {
    fn tickers(&mut self) -> Vec<MessageTickerDef<CompactorMessage>> {
        vec![
            MessageTickerDef::new(
                self.options.poll_interval,
                Box::new(|| CompactorMessage::PollManifest),
            ),
            MessageTickerDef::new(
                Duration::from_secs(10),
                Box::new(|| CompactorMessage::LogStats),
            ),
            MessageTickerDef::new(
                self.options.commit_compacted_interval,
                Box::new(|| CompactorMessage::CommitCompacted),
            ),
        ]
    }

    fn notifiers(&mut self) -> Vec<Box<dyn Notifier<CompactorMessage>>> {
        // A flush written in this process is scheduled without waiting for the next poll.
        vec![Box::new(WriteNotifier::new(
            self.state_writer.subscribe_manifest_writes(),
            self.system_clock.clone(),
            self.options.poll_interval.min(MIN_WRITE_NOTICE_INTERVAL),
            |id| CompactorMessage::ManifestWritten { id },
        ))]
    }

    async fn handle(&mut self, message: CompactorMessage) -> Result<(), SlateDBError> {
        match message {
            CompactorMessage::LogStats => self.handle_log_ticker(),
            CompactorMessage::PollManifest => self.handle_ticker().await?,
            CompactorMessage::ManifestWritten { id } => {
                // The compactor's own manifest writes are already in its view.
                if id > self.state_writer.manifest_id() {
                    self.handle_ticker().await?;
                }
            }
            CompactorMessage::CommitCompacted => {
                // A remote worker can only produce a new Compacted result for a
                // job the coordinator already tracks as active. When there are no
                // active jobs, the regular manifest poll is sufficient to discover
                // new submissions. We can avoid an otherwise idle object-store refresh.
                if self.state().active_compactions().next().is_some() {
                    self.state_writer.load_compactions().await?;
                    self.update_distributed_compaction_metrics();
                    self.commit_compacted_entries().await?;
                }
            }
        }
        Ok(())
    }

    async fn cleanup(
        &mut self,
        mut _messages: BoxStream<'async_trait, CompactorMessage>,
        _result: Result<(), SlateDBError>,
    ) -> Result<(), SlateDBError> {
        Ok(())
    }
}

impl CompactorEventHandler {
    pub(crate) async fn new(
        manifest_store: Arc<ManifestStore>,
        compactions_store: Arc<CompactionsStore>,
        options: Arc<CompactorOptions>,
        scheduler: Arc<dyn CompactionScheduler + Send + Sync>,
        rand: Arc<DbRand>,
        stats: Arc<CompactionStats>,
        system_clock: Arc<dyn SystemClock>,
        recorder: MetricsRecorderHelper,
    ) -> Result<Self, SlateDBError> {
        Self::new_with_epoch(
            manifest_store,
            compactions_store,
            options,
            scheduler,
            rand,
            stats,
            system_clock,
            recorder,
            None,
        )
        .await
    }

    pub(crate) async fn new_with_epoch(
        manifest_store: Arc<ManifestStore>,
        compactions_store: Arc<CompactionsStore>,
        options: Arc<CompactorOptions>,
        scheduler: Arc<dyn CompactionScheduler + Send + Sync>,
        rand: Arc<DbRand>,
        stats: Arc<CompactionStats>,
        system_clock: Arc<dyn SystemClock>,
        recorder: MetricsRecorderHelper,
        leadership_epoch: Option<std::num::NonZeroU64>,
    ) -> Result<Self, SlateDBError> {
        let state_writer = CompactorStateWriter::new_with_epoch(
            manifest_store,
            compactions_store,
            system_clock.clone(),
            options.as_ref(),
            rand.clone(),
            leadership_epoch,
        )
        .await?;
        let compactor_epoch = state_writer.state.manifest().value.compactor_epoch;
        stats.compactor_epoch.set(compactor_epoch as i64);
        Ok(Self {
            state_writer,
            options,
            scheduler,
            rand,
            stats,
            system_clock,
            recorder,
            prev_claimed: HashSet::new(),
            worker_heartbeat_gauges: HashMap::new(),
        })
    }

    fn state(&self) -> &CompactorState {
        &self.state_writer.state
    }

    fn state_mut(&mut self) -> &mut CompactorState {
        &mut self.state_writer.state
    }

    /// Emits the current compaction state and per-job progress.
    fn handle_log_ticker(&self) {
        self.log_compaction_state();
        self.log_compaction_throughput();
    }

    /// Logs per-job compaction progress and updates aggregate throughput metrics.
    fn log_compaction_throughput(&self) {
        let current_time = self.system_clock.now();
        let current_time_ms = current_time.timestamp_millis() as u64;
        let db_state = self.state().db_state();
        let mut total_estimated_bytes = 0u64;
        let mut total_bytes_processed = 0u64;
        let mut total_elapsed_secs = 0.0f64;

        for compaction in self
            .state()
            .active_compactions()
            .filter(|c| c.status() != CompactionStatus::Compacted)
        {
            let Some(estimated_source_bytes) =
                Self::calculate_estimated_source_bytes(compaction, db_state)
            else {
                warn!(
                    "skipping compaction progress because a source is absent from the manifest \
                     [id={}, status={:?}, spec={}]",
                    compaction.id(),
                    compaction.status(),
                    compaction.spec(),
                );
                continue;
            };
            total_estimated_bytes += estimated_source_bytes;
            total_bytes_processed += compaction.bytes_processed();

            // Calculate elapsed time using ULID timestamp
            let start_time_ms = compaction
                .id()
                .datetime()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("invalid duration")
                .as_millis() as u64;
            let elapsed_secs = if start_time_ms > 0 {
                (current_time_ms as f64 - start_time_ms as f64) / 1000.0
            } else {
                0.0
            };
            total_elapsed_secs += elapsed_secs;

            // Per-job throughput for logging
            let throughput = if elapsed_secs > 0.0 {
                compaction.bytes_processed() as f64 / elapsed_secs
            } else {
                0.0
            };

            let percentage = if estimated_source_bytes > 0 {
                (compaction.bytes_processed() * 100 / estimated_source_bytes) as u32
            } else {
                0
            };
            debug!(
                "compaction progress [id={}, progress={}%, processed_bytes={}, estimated_source_bytes={}, elapsed={:.2}s, throughput={}/s]",
                compaction.id(),
                percentage,
                format_bytes_si(compaction.bytes_processed()),
                format_bytes_si(estimated_source_bytes),
                elapsed_secs,
                format_bytes_si(throughput as u64),
            );
        }

        let total_throughput = if total_elapsed_secs > 0.0 {
            total_bytes_processed as f64 / total_elapsed_secs
        } else {
            0.0
        };

        self.stats
            .total_bytes_being_compacted
            .set(total_estimated_bytes as i64);
        self.stats.total_throughput.set(total_throughput as i64);
    }

    /// Calculates the estimated total source bytes for a compaction.
    fn calculate_estimated_source_bytes(
        compaction: &Compaction,
        db_state: &ManifestCore,
    ) -> Option<u64> {
        let tree = db_state.tree_for_segment(compaction.spec().segment())?;

        let views_by_id: HashMap<Ulid, &SsTableView> =
            tree.l0.iter().map(|view| (view.id, view)).collect();
        let srs_by_id: HashMap<u32, &SortedRun> =
            tree.compacted.iter().map(|sr| (sr.id, sr)).collect();

        compaction
            .spec()
            .sources()
            .iter()
            .try_fold(0, |total, source| {
                let source_bytes = match source {
                    SourceId::SstView(id) => views_by_id.get(id)?.estimate_visible_size(),
                    SourceId::SortedRun(id) => srs_by_id.get(id)?.estimate_visible_size(),
                };
                Some(total + source_bytes)
            })
    }

    /// Handles a polling tick by refreshing compactions and the manifest, then possibly scheduling compactions.
    async fn handle_ticker(&mut self) -> Result<(), SlateDBError> {
        self.state_writer.refresh().await?;
        self.reclaim_stale_workers().await?;
        self.update_distributed_compaction_metrics();
        self.commit_compacted_entries().await?;
        self.maybe_schedule_compactions().await?;
        self.maybe_validate_submitted_compactions().await?;
        Ok(())
    }

    /// Reclaims `Running` compactions whose workers have not emitted a heartbeat
    /// within [`CompactorOptions::worker_heartbeat_timeout`], as well as any
    /// `Running` entry with no worker at all. Reclaimed compactions are reset to
    /// `Scheduled` (with no worker) so they can be picked up again while
    /// preserving any persisted job context needed for resume.
    ///
    /// This is the coordinator-side half of the failure-detection protocol described
    /// in RFC-0025. The worker side emits heartbeats (updating `last_heartbeat_ms`)
    /// periodically; the coordinator scans for stale entries on each tick.
    async fn reclaim_stale_workers(&mut self) -> Result<(), SlateDBError> {
        let now_ms = self.system_clock.now().timestamp_millis() as u64;
        let timeout_ms = self.options.worker_heartbeat_timeout.as_millis() as u64;

        // A Running compaction is stale if its worker's heartbeat has aged past
        // the timeout, or if it has no worker at all. The protocol shouldn't
        // produce the latter, but it would otherwise be stuck in Running forever.
        // Capture the owning worker id (None in the no-worker case) for the log.
        let stale: Vec<(Ulid, Option<String>)> = self
            .state()
            .compactions_with_status(&[CompactionStatus::Running])
            .filter(|c| match c.worker() {
                Some(w) => now_ms.saturating_sub(w.last_heartbeat_ms) > timeout_ms,
                None => true,
            })
            .map(|c| (c.id(), c.worker().map(|w| w.worker_id.clone())))
            .collect();

        if stale.is_empty() {
            return Ok(());
        }

        for (id, worker_id) in &stale {
            match worker_id {
                Some(worker_id) => info!(
                    "reclaiming stale compaction whose worker's heartbeat timed out \
                     [worker_id={}, id={}]",
                    worker_id, id
                ),
                None => {
                    let message = format!(
                        "reclaiming Running compaction that has no worker; this should \
                         not happen. please open issue. [id={}]",
                        id
                    );
                    debug_assert!(false, "{message}");
                    error!("{message}");
                }
            }
            self.state_mut().update_compaction(id, |c| {
                c.set_status(CompactionStatus::Scheduled);
                c.set_worker(None);
            });
        }

        self.state_writer.write_compactions_safely().await?;
        self.stats.jobs_reclaimed.increment(stale.len() as u64);
        Ok(())
    }

    /// Updates coordinator-side distributed-compaction metrics after each tick:
    /// - `jobs_claimed`: counts jobs claimed by a worker. A job is "claimed"
    ///   once it carries a worker and is `Running` or `Compacted` (the worker
    ///   keeps its ownership through `Compacted`), so a job that finishes
    ///   execution between two ticks is still counted. The first update counts
    ///   every inherited claim once; later updates count set differences.
    /// - `worker_last_heartbeat_ms`: a per-worker gauge of the last heartbeat
    ///   timestamp. Gauge handles are cached for every worker id observed by this
    ///   coordinator process.
    fn update_distributed_compaction_metrics(&mut self) {
        use crate::compactor::stats::{WORKER_ID_LABEL, WORKER_LAST_HEARTBEAT_MS};

        let claimed: Vec<(Ulid, crate::compactor_state::WorkerSpec)> = self
            .state()
            .compactions_with_status(&[CompactionStatus::Running, CompactionStatus::Compacted])
            .filter_map(|c| c.worker().cloned().map(|w| (c.id(), w)))
            .collect();

        let current_ids: HashSet<Ulid> = claimed.iter().map(|(id, _)| *id).collect();
        // The initially empty set counts all inherited claims once.
        // This includes work claimed while the coordinator was unavailable;
        // thereafter the counter records only newly observed claim ids.
        let newly_claimed = current_ids.difference(&self.prev_claimed).count() as u64;
        if newly_claimed > 0 {
            self.stats.jobs_claimed.increment(newly_claimed);
        }
        self.prev_claimed = current_ids;

        // Refresh the cached gauge handle for every worker that owns an in-flight job.
        let recorder = self.recorder.clone();
        let mut last_heartbeat_per_worker: HashMap<String, u64> = HashMap::new();
        for (_, w) in &claimed {
            last_heartbeat_per_worker
                .entry(w.worker_id.clone())
                .and_modify(|last| *last = (*last).max(w.last_heartbeat_ms))
                .or_insert(w.last_heartbeat_ms);
        }

        for (id, last_heartbeat_ms) in &last_heartbeat_per_worker {
            let gauge = self
                .worker_heartbeat_gauges
                .entry(id.clone())
                .or_insert_with(|| {
                    recorder
                        .gauge(WORKER_LAST_HEARTBEAT_MS)
                        .labels(&[(WORKER_ID_LABEL, id.as_str())])
                        .register()
                });
            gauge.set(*last_heartbeat_ms as i64);
        }
    }

    /// Commits any compactions in the `Compacted` state to the manifest.
    ///
    /// `Compacted` is the distributed intermediate state written by a worker after it
    /// finishes execution. The coordinator is solely responsible for transitioning
    /// `Compacted → Completed` (or `Compacted → Failed`) by writing the manifest first
    /// and then updating `.compactions`. See RFC-0025 §Manifest Commit Protocol.
    ///
    /// Recovery: on coordinator restart, `Compacted` entries are left intact in
    /// `.compactions`. The first call to this method after startup retries the manifest
    /// write. `validate_compaction` will fail if the sources are already absent (meaning
    /// the manifest was written before the crash), in which case the entry is marked
    /// `Failed` even though the compaction may have actually succeeded. This is preferred
    /// over trying to detect success after the fact because any heuristic (e.g. checking
    /// whether the destination SR is present in the manifest) is fragile: the destination
    /// SR id may coincide with a source SR id (e.g. `[L0:1, SR:1] -> SR:1`), so the
    /// destination SR can exist both before and after the compaction runs and its presence
    /// alone does not confirm the compaction's output was committed. Marking `Failed` is
    /// safe in both crash scenarios:
    /// - Crash before manifest write: sources are still present, so the scheduler
    ///   re-compacts them on the next tick.
    /// - Crash after manifest write: sources are already absent and the manifest is
    ///   already in the correct post-compaction state, so marking `Failed` has no
    ///   effect on correctness.
    async fn commit_compacted_entries(&mut self) -> Result<(), SlateDBError> {
        let compacted = self
            .state()
            .compactions_with_status(&[CompactionStatus::Compacted])
            .cloned()
            .collect::<Vec<_>>();

        if compacted.is_empty() {
            return Ok(());
        }

        let mut manifest_changed = false;
        for compaction in compacted {
            let id = compaction.id();
            match self.validate_compaction(&compaction) {
                Ok(()) => {
                    let destination = compaction
                        .spec()
                        .destination()
                        .expect("Compacted tiered compaction must have a destination SR id");
                    let output_sr = SortedRun::new(
                        destination,
                        compaction
                            .output_ssts()
                            .iter()
                            .map(|sst| SsTableView::identity(sst.clone())),
                    );
                    self.state_mut().finish_compaction(id, output_sr);
                    manifest_changed = true;
                    self.stats
                        .last_compaction_ts
                        .set(self.system_clock.now().timestamp());
                }
                Err(_) => {
                    // Validation failed against the current manifest. Mark Failed so the
                    // entry isn't retried on the next tick.
                    info!(
                        "compacted entry failed validation, marking Failed [id={}]",
                        id
                    );
                    self.state_mut()
                        .update_compaction(&id, |c| c.set_status(CompactionStatus::Failed));
                }
            }
        }

        self.log_compaction_state();
        if manifest_changed {
            self.state_writer.write_state_safely().await?;
        } else {
            // Validation failures only change `.compactions`. Avoid creating a
            // checkpoint and writing an unchanged manifest.
            self.state_writer.write_compactions_safely().await?;
        }

        Ok(())
    }

    /// Validates a Submitted compaction against the current manifest before starting it.
    ///
    /// Runs when a Submitted compaction is accepted and when a Compacted entry is
    /// committed, so this is the canonical gate for compaction-against-current-state
    /// validity. Cross-compaction conflicts (destination collisions across active
    /// compactions, concurrent drains on the same segment) are enforced upstream in
    /// [`CompactorState::add_compaction`] and are not re-checked here.
    ///
    /// Invariants checked:
    /// - Compaction has sources
    /// - Drain specs do not target the empty-prefix (root) segment
    /// - The target segment exists in the manifest
    /// - All sources exist in the target segment's tree
    /// - Submitted L0-only tiered compactions have a destination > highest SR id across all trees
    /// - Compacted L0-only tiered compactions have a destination > highest SR id in their segment
    /// - A tiered destination does not overwrite a committed SR in any tree unless the SR is among sources
    /// - Drain L0 sources cover every L0 at or below the newest drained L0 in the target tree
    /// - At most one L0 compaction is Running per segment
    /// - Scheduler-specific policy via [`CompactionScheduler::validate_compaction`]
    fn validate_compaction(&self, compaction: &Compaction) -> Result<(), SlateDBError> {
        let spec = compaction.spec();
        // Validate compaction sources exist
        if spec.sources().is_empty() {
            warn!("submitted compaction is empty: {:?}", spec.sources());
            return Err(SlateDBError::InvalidCompaction);
        }

        // Drain specs cannot target the empty-prefix segment (RFC-0024): the
        // root-tree compatibility encoding is not retired by drain.
        if spec.is_drain() && spec.segment().is_empty() {
            warn!("rejected drain compaction targeting the empty-prefix segment");
            return Err(SlateDBError::InvalidCompaction);
        }

        // Validate compaction sources exist in the spec's target segment tree.
        // RFC-0024: every spec names exactly one segment, and its sources must
        // live in that segment's tree.
        let db_state = self.state().db_state();
        let Some(tree) = db_state.tree_for_segment(spec.segment()) else {
            warn!(
                "submitted compaction targets unknown segment: {:?}",
                spec.segment()
            );
            return Err(SlateDBError::InvalidCompaction);
        };
        let l0_view_ids = tree.l0.iter().map(|view| view.id).collect::<HashSet<_>>();
        let sr_ids = tree
            .compacted
            .iter()
            .map(|sr| sr.id)
            .collect::<HashSet<_>>();

        if let Some(missing) = spec.sources().iter().find(|source| match source {
            SourceId::SstView(id) => !l0_view_ids.contains(id),
            SourceId::SortedRun(id) => !sr_ids.contains(id),
        }) {
            debug!("compaction source missing from db state: {:?}", missing);
            return Err(SlateDBError::InvalidCompaction);
        }

        // Validate L0-only compactions create a new SR after every SR that matters
        // for their lifecycle stage. Submitted specs reserve a fresh globally
        // unique SR id, so they must be above every committed SR in every segment.
        // Compacted entries are being committed into one target segment and may
        // validly finish after a different segment has already committed a higher
        // SR id, but they still must preserve local SR ordering within the target
        // segment.
        if spec.has_l0_sources() && !spec.has_sr_sources() {
            let highest_id = if compaction.status() == CompactionStatus::Submitted {
                db_state
                    .trees()
                    .flat_map(|t| t.compacted.iter())
                    .map(|sr| sr.id)
                    .max()
                    .map_or(0, |id| id + 1)
            } else if compaction.status() == CompactionStatus::Compacted {
                tree.compacted
                    .iter()
                    .map(|sr| sr.id)
                    .max()
                    .map_or(0, |id| id + 1)
            } else {
                warn!(
                    "validate_compaction called with unexpected compaction status [id={:?}, status={:?}]",
                    compaction.id(),
                    compaction.status()
                );
                return Err(SlateDBError::InvalidCompaction);
            };
            // Drain specs have no destination and aren't subject to this check.
            if let Some(dst) = spec.destination() {
                if dst < highest_id {
                    warn!(
                        "compaction destination is lesser than the expected L0-only highest_id: {:?} {:?}",
                        dst, highest_id
                    );
                    return Err(SlateDBError::InvalidCompaction);
                }
            }
        }

        Self::validate_destination_overwrite(spec, db_state)?;
        Self::validate_drain_watermark_advance(spec, tree)?;

        // Reject parallel L0 compactions within the same segment. Each
        // segment owns its own `last_compacted_l0_sst_view_id` watermark
        // (RFC-0024), so out-of-order completion is only a hazard for
        // compactions sharing a target tree. L0 compactions in disjoint
        // segments — including drain specs in different segments — are
        // safe to run concurrently.
        if spec.has_l0_sources() {
            let target_segment = spec.segment();
            // Only Scheduled and Running represent live claims; Submitted is still
            // being validated (and would see itself), Compacted is pending commit.
            let active_l0_in_same_segment = self
                .state()
                .compactions_with_status(&[CompactionStatus::Scheduled, CompactionStatus::Running])
                .any(|c| c.spec().has_l0_sources() && c.spec().segment() == target_segment);
            if active_l0_in_same_segment {
                warn!(
                    "rejected compaction: parallel L0 compaction already active in segment {:?}",
                    target_segment
                );
                return Err(SlateDBError::InvalidCompaction);
            }
        }

        self.scheduler
            .validate(&self.state().into(), spec)
            .map_err(|_e| SlateDBError::InvalidCompaction)
    }

    /// Rejects a tiered compaction whose destination SR id already exists as a
    /// committed SR anywhere in the manifest unless that SR is among the spec's
    /// sources (i.e. the compaction overwrites it as part of the merge). SR ids
    /// are globally unique across all segment trees (RFC-0024). Drain specs
    /// produce no output SR and pass through unchecked.
    fn validate_destination_overwrite(
        compaction: &CompactionSpec,
        db_state: &ManifestCore,
    ) -> Result<(), SlateDBError> {
        let Some(dst) = compaction.destination() else {
            return Ok(());
        };
        let sr_exists_anywhere = db_state
            .trees()
            .flat_map(|t| t.compacted.iter())
            .any(|sr| sr.id == dst);
        let sr_in_sources = compaction.sources().iter().any(|src| match src {
            SourceId::SortedRun(sr) => *sr == dst,
            SourceId::SstView(_) => false,
        });
        if sr_exists_anywhere && !sr_in_sources {
            warn!(
                "compaction destination overwrites committed SR not in sources: {:?}",
                dst
            );
            return Err(SlateDBError::InvalidCompaction);
        }
        Ok(())
    }

    /// Rejects a drain spec that would leave one or more L0s below the new
    /// watermark. The watermark advances to the newest drained L0; every L0
    /// at or below that point in `tree.l0` (which is sorted newest-first)
    /// must be listed among the drain sources, or it would survive in
    /// `tree.l0` while being eclipsed by the watermark — a manifest state
    /// the writer cannot merge. Tiered specs do not advance the watermark
    /// and pass through unchecked.
    fn validate_drain_watermark_advance(
        compaction: &CompactionSpec,
        tree: &LsmTreeState,
    ) -> Result<(), SlateDBError> {
        if !compaction.is_drain() {
            return Ok(());
        }
        let drained_l0_ids: HashSet<Ulid> = compaction
            .sources()
            .iter()
            .filter_map(|s| match s {
                SourceId::SstView(id) => Some(*id),
                SourceId::SortedRun(_) => None,
            })
            .collect();
        if drained_l0_ids.is_empty() {
            return Ok(());
        }
        // tree.l0 is sorted newest-first. Find the newest drained L0's index;
        // every L0 at or after that index (older or equal) must also be drained.
        let Some(newest_drained_idx) = tree
            .l0
            .iter()
            .position(|view| drained_l0_ids.contains(&view.id))
        else {
            return Ok(());
        };
        for view in tree.l0.iter().skip(newest_drained_idx) {
            if !drained_l0_ids.contains(&view.id) {
                warn!(
                    "drain spec leaves L0 below the new watermark: segment={:?}, surviving_l0={:?}",
                    compaction.segment(),
                    view.id
                );
                return Err(SlateDBError::InvalidCompaction);
            }
        }
        Ok(())
    }

    /// Requests new compactions from the scheduler, validates them, and adds them to the
    /// state up to the the max concurrency limit. This method does not actually start
    /// the compactions; that is done in [`CompactorEventHandler::maybe_start_compactions`].
    async fn maybe_schedule_compactions(&mut self) -> Result<(), SlateDBError> {
        let running_compaction_count = self.running_compaction_count();
        let available_capacity = self.options.max_concurrent_compactions - running_compaction_count;

        if available_capacity == 0 {
            debug!(
                "skipping compaction scheduling since at capacity [running_compactions={}, max_concurrent_compactions={}]",
                running_compaction_count,
                self.options.max_concurrent_compactions
            );
            return Ok(());
        }

        let mut specs = self.scheduler.propose(&self.state().into());

        // Add new compactions up to the max concurrency limit
        let num_specs_added = specs
            .drain(..available_capacity.min(specs.len()))
            .map(|spec| -> Result<(), SlateDBError> {
                let compaction_id = self.rand.rng().gen_ulid(self.system_clock.as_ref());
                debug!(
                    "scheduling new compaction [spec={:?}, id={}]",
                    spec, compaction_id
                );
                self.state_mut()
                    .add_compaction(Compaction::new(compaction_id, spec))?;
                Ok(())
            })
            .count();

        // Save newly added compactions
        if num_specs_added > 0 {
            self.state_writer.write_compactions_safely().await?;
        }

        Ok(())
    }

    /// Validates every `Submitted` compaction against the current manifest and
    /// promotes valid tiered specs to `Scheduled` — the coordinator's "ready
    /// for a worker to claim" state. Drain specs and trivial moves short-circuit
    /// the executor and are applied directly to the in-memory manifest (→
    /// `Completed`). Invalid specs are marked `Failed`. State changes are
    /// persisted before any worker (including the local executor) can act on
    /// them: when a submission changed the manifest, the manifest and
    /// `.compactions` are written together, otherwise `.compactions` alone.
    ///
    /// Workers exclusively claim `Scheduled` entries; they never act on
    /// `Submitted`. Routing validation through this single chokepoint keeps
    /// the coordinator the gatekeeper for spec-against-manifest validity and
    /// guarantees the coordinator has the entry in local state before any
    /// remote worker can transition it onward.
    async fn maybe_validate_submitted_compactions(&mut self) -> Result<(), SlateDBError> {
        let submitted_compactions = self
            .state()
            .compactions_with_status(&[CompactionStatus::Submitted])
            .cloned()
            .collect::<Vec<_>>();

        if submitted_compactions.is_empty() {
            return Ok(());
        }

        let mut manifest_changed = false;

        for compaction in &submitted_compactions {
            // Validate the candidate compaction; mark as failed if invalid.
            if let Err(e) = self.validate_compaction(compaction) {
                error!(
                    "compaction validation failed [error={:?}, compaction={:?}]",
                    compaction, e
                );
                self.state_mut().update_compaction(&compaction.id(), |c| {
                    c.set_status(CompactionStatus::Failed)
                });
                continue;
            }

            // Coordinator-local compactions never enter Scheduled because no
            // worker runs them. Everything else becomes ready to claim.
            let trivial_move_output = self
                .options
                .enable_trivial_move
                .then(|| compaction.trivial_move_output(self.state().db_state()))
                .flatten();

            if compaction.spec().is_drain() {
                self.state_mut().finish_drain_compaction(compaction.id());
                manifest_changed = true;
            } else if let Some(output_sr) = trivial_move_output {
                info!("trivially moving compaction [spec={}]", compaction.spec());
                self.state_mut()
                    .finish_compaction(compaction.id(), output_sr);
                manifest_changed = true;
            } else {
                self.state_mut().update_compaction(&compaction.id(), |c| {
                    c.clear_ctx();
                    c.set_status(CompactionStatus::Scheduled)
                });
            }
        }

        if manifest_changed {
            self.state_writer.write_state_safely().await?;
            self.stats
                .last_compaction_ts
                .set(self.system_clock.now().timestamp());
        } else {
            self.state_writer.write_compactions_safely().await?;
        }

        Ok(())
    }

    /// Records a failed compaction attempt.
    #[allow(dead_code)]
    async fn finish_failed_compaction(&mut self, id: Ulid) -> Result<(), SlateDBError> {
        self.state_mut()
            .update_compaction(&id, |c| c.set_status(CompactionStatus::Failed));
        self.state_writer.write_compactions_safely().await?;
        Ok(())
    }

    /// Records a successful compaction, persists the manifest, and checks for new compactions
    /// to schedule.
    #[allow(dead_code)]
    #[instrument(level = "debug", skip_all, fields(id = %id))]
    async fn finish_compaction(
        &mut self,
        id: Ulid,
        output_sr: SortedRun,
    ) -> Result<(), SlateDBError> {
        self.state_mut().finish_compaction(id, output_sr);
        self.log_compaction_state();
        self.state_writer.write_state_safely().await?;
        self.maybe_schedule_compactions().await?;
        self.maybe_validate_submitted_compactions().await?;
        self.stats
            .last_compaction_ts
            .set(self.system_clock.now().timestamp());
        Ok(())
    }

    /// Logs the current DB runs and in-flight compactions.
    fn log_compaction_state(&self) {
        self.state().db_state().log_db_runs();
        let compactions = self.state().active_compactions();
        for compaction in compactions {
            if log::log_enabled!(log::Level::Debug) {
                debug!("in-flight compaction [compaction={:?}]", compaction);
            } else {
                info!("in-flight compaction [compaction={}]", compaction);
            }
        }
    }

    fn running_compaction_count(&self) -> usize {
        self.state()
            .active_compactions()
            .filter(|c| c.status() == CompactionStatus::Running)
            .count()
    }
}

pub mod stats {
    use slatedb_common::metrics::{CounterFn, GaugeFn, MetricsRecorderHelper, UpDownCounterFn};
    use std::sync::Arc;

    pub use crate::merge_operator::MERGE_OPERATOR_OPERANDS;

    use crate::merge_operator::{
        MERGE_OPERATOR_COMPACT_PATH, MERGE_OPERATOR_OPERANDS_DESCRIPTION, MERGE_OPERATOR_PATH_LABEL,
    };

    macro_rules! compactor_stat_name {
        ($suffix:expr) => {
            concat!("slatedb.compactor.", $suffix)
        };
    }

    pub const BYTES_COMPACTED: &str = compactor_stat_name!("bytes_compacted");
    pub const COMPACTOR_EPOCH: &str = compactor_stat_name!("epoch");
    pub const LAST_COMPACTION_TS_SEC: &str = compactor_stat_name!("last_compaction_timestamp_sec");
    pub const RUNNING_COMPACTIONS: &str = compactor_stat_name!("running_compactions");
    pub const SSTS_WRITTEN: &str = compactor_stat_name!("ssts_written");
    pub const JOBS_CLAIMED: &str = compactor_stat_name!("jobs_claimed");
    pub const JOBS_RECLAIMED: &str = compactor_stat_name!("jobs_reclaimed");
    pub const WORKER_LAST_HEARTBEAT_MS: &str = compactor_stat_name!("worker_last_heartbeat_ms");
    /// Label key carrying a worker's id on per-worker metrics.
    pub const WORKER_ID_LABEL: &str = "worker_id";
    pub const TOTAL_BYTES_BEING_COMPACTED: &str =
        compactor_stat_name!("total_bytes_being_compacted");
    pub const TOTAL_THROUGHPUT_BYTES_PER_SEC: &str =
        compactor_stat_name!("total_throughput_bytes_per_sec");
    pub const EXPIRED_ENTRIES_PURGED: &str = compactor_stat_name!("expired_entries_purged");
    pub const EXPIRED_ENTRIES_PURGED_DESCRIPTION: &str =
        "Count of expired entries purged by the RetentionIterator. \
         `entry_type=\"value\"` counts expired value entries rewritten as tombstones; \
         `entry_type=\"merge\"` counts expired merge entries dropped without a tombstone.";
    pub const ENTRY_TYPE_LABEL: &str = "entry_type";
    pub const ENTRY_TYPE_VALUE: &str = "value";
    pub const ENTRY_TYPE_MERGE: &str = "merge";

    /// Coordinator-side compaction metrics.
    ///
    /// Per-worker throughput (`bytes_compacted`, `running_compactions`,
    /// `ssts_written`) lives in [`WorkerStats`] instead: those are emitted by the
    /// executor running inside a worker and tagged with `{worker_id}`. The
    /// coordinator runs no executor, so it does not emit them (RFC-0025
    /// Observability).
    pub(crate) struct CompactionStats {
        pub(crate) compactor_epoch: Arc<dyn GaugeFn>,
        pub(crate) last_compaction_ts: Arc<dyn GaugeFn>,
        pub(crate) total_bytes_being_compacted: Arc<dyn GaugeFn>,
        pub(crate) total_throughput: Arc<dyn GaugeFn>,
        pub(crate) merge_operator_compact_operands: Arc<dyn CounterFn>,
        pub(crate) expired_entries_purged_value: Arc<dyn CounterFn>,
        pub(crate) expired_entries_purged_merge: Arc<dyn CounterFn>,
        /// `Scheduled → Running` transitions the coordinator observed in `.compactions`.
        pub(crate) jobs_claimed: Arc<dyn CounterFn>,
        /// Stale jobs the coordinator reset `Running → Submitted`.
        pub(crate) jobs_reclaimed: Arc<dyn CounterFn>,
    }

    impl CompactionStats {
        pub(crate) fn new(recorder: &MetricsRecorderHelper) -> Self {
            Self {
                compactor_epoch: recorder.gauge(COMPACTOR_EPOCH).register(),
                last_compaction_ts: recorder.gauge(LAST_COMPACTION_TS_SEC).register(),
                jobs_claimed: recorder.counter(JOBS_CLAIMED).register(),
                jobs_reclaimed: recorder.counter(JOBS_RECLAIMED).register(),
                total_bytes_being_compacted: recorder.gauge(TOTAL_BYTES_BEING_COMPACTED).register(),
                total_throughput: recorder.gauge(TOTAL_THROUGHPUT_BYTES_PER_SEC).register(),
                merge_operator_compact_operands: recorder
                    .counter(MERGE_OPERATOR_OPERANDS)
                    .labels(&[(MERGE_OPERATOR_PATH_LABEL, MERGE_OPERATOR_COMPACT_PATH)])
                    .description(MERGE_OPERATOR_OPERANDS_DESCRIPTION)
                    .register(),
                expired_entries_purged_value: recorder
                    .counter(EXPIRED_ENTRIES_PURGED)
                    .labels(&[(ENTRY_TYPE_LABEL, ENTRY_TYPE_VALUE)])
                    .description(EXPIRED_ENTRIES_PURGED_DESCRIPTION)
                    .register(),
                expired_entries_purged_merge: recorder
                    .counter(EXPIRED_ENTRIES_PURGED)
                    .labels(&[(ENTRY_TYPE_LABEL, ENTRY_TYPE_MERGE)])
                    .description(EXPIRED_ENTRIES_PURGED_DESCRIPTION)
                    .register(),
            }
        }

        pub(crate) fn retention_metrics(&self) -> crate::retention_iterator::RetentionMetrics {
            crate::retention_iterator::RetentionMetrics {
                expired_entries_purged_value: self.expired_entries_purged_value.clone(),
                expired_entries_purged_merge: self.expired_entries_purged_merge.clone(),
            }
        }
    }

    /// Per-worker compaction throughput, tagged `{worker_id=<id>}`.
    ///
    /// Registered once per worker from the worker's recorder with the worker id
    /// baked into the label set, and incremented by the executor (which always
    /// runs inside a worker). A shared metrics backend can therefore attribute
    /// throughput to individual workers in multi-worker deployments.
    #[derive(Clone)]
    pub(crate) struct WorkerStats {
        /// Bytes written to output SSTs by this worker (cumulative).
        pub(crate) bytes_compacted: Arc<dyn CounterFn>,
        /// Compaction jobs currently executing on this worker.
        pub(crate) running_compactions: Arc<dyn UpDownCounterFn>,
        /// Output SSTs produced by this worker (cumulative).
        pub(crate) ssts_written: Arc<dyn CounterFn>,
    }

    impl WorkerStats {
        pub(crate) fn new(recorder: &MetricsRecorderHelper, worker_id: &str) -> Self {
            Self {
                bytes_compacted: recorder
                    .counter(BYTES_COMPACTED)
                    .labels(&[(WORKER_ID_LABEL, worker_id)])
                    .register(),
                running_compactions: recorder
                    .up_down_counter(RUNNING_COMPACTIONS)
                    .labels(&[(WORKER_ID_LABEL, worker_id)])
                    .register(),
                ssts_written: recorder
                    .counter(SSTS_WRITTEN)
                    .labels(&[(WORKER_ID_LABEL, worker_id)])
                    .register(),
            }
        }

        /// A no-op instance for tests that don't assert on worker metrics.
        #[cfg(test)]
        pub(crate) fn noop() -> Self {
            Self::new(&MetricsRecorderHelper::noop(), "")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};
    use std::future::Future;
    use std::ops::Bound::{Excluded, Included};
    use std::ops::Range;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime};

    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use parking_lot::Mutex;
    use rand::RngCore;
    use rstest::rstest;
    use slatedb_common::MockSystemClock;
    use ulid::Ulid;

    use super::*;
    use crate::batch::WriteBatch;
    use crate::block_cache_policy::BlockCachePolicy;
    use crate::bytes_range::BytesRange;
    use crate::compaction_worker::WorkerMessage;
    use crate::compactions_store::{FenceableCompactions, StoredCompactions};
    use crate::compactor::stats::CompactionStats;
    use crate::compactor::stats::COMPACTOR_EPOCH;
    use crate::compactor::stats::LAST_COMPACTION_TS_SEC;
    use crate::compactor_executor::{
        CompactionExecutor, TokioCompactionExecutor, TokioCompactionExecutorOptions,
    };
    use crate::compactor_state::Compaction;
    use crate::compactor_state::CompactionStatus;
    use crate::compactor_state::{SourceId, WorkerSpec};
    use crate::config::{
        CompactionWorkerOptions, FlushOptions, FlushType, MergeOptions, PutOptions, Settings,
        SizeTieredCompactionSchedulerOptions, SstBlockSize, Ttl, WriteOptions,
    };
    use crate::db::Db;
    use crate::db_cache::test_utils::TestCache;
    use crate::db_cache::CacheTarget;
    use crate::db_state::{SortedRun, SsTableHandle, SsTableId, SsTableInfo, SsTableView};
    use crate::error::SlateDBError;
    use crate::format::sst::{SsTableFormat, SST_FORMAT_VERSION_LATEST};
    use crate::iter::RowEntryIterator;
    use crate::manifest::store::{ManifestStore, StoredManifest};
    use crate::manifest::{LsmTreeState, Manifest, ManifestCore, Segment, VersionedManifest};
    use crate::merge_operator::{MergeOperator, MergeOperatorError};
    use crate::proptest_util::rng;
    use crate::sst_iter::{SstIterator, SstIteratorOptions};
    use crate::tablestore::{TableStore, TableStoreKind};
    use crate::test_utils::{
        assert_iterator, bounded_sst_view, FixedThreeBytePrefixExtractor, GatedObjectStore,
    };
    use crate::types::KeyValue;
    use crate::types::RowEntry;
    use bytes::Bytes;
    use slatedb_common::clock::{DefaultSystemClock, SystemClock};

    const PATH: &str = "/test/db";

    #[derive(Clone)]
    struct SegmentTestScheduler {
        segment: Bytes,
        min_l0_sources: usize,
    }

    impl CompactionScheduler for SegmentTestScheduler {
        fn propose(&self, state: &CompactorStateView) -> Vec<CompactionSpec> {
            let db_state = state.manifest().core();
            let Some(tree) = db_state.tree_for_segment(self.segment.as_ref()) else {
                return vec![];
            };
            if tree.l0.len() < self.min_l0_sources {
                return vec![];
            }

            let mut sources: Vec<SourceId> = tree
                .l0
                .iter()
                .map(|view| SourceId::SstView(view.id))
                .collect();
            sources.extend(tree.compacted.iter().map(|sr| SourceId::SortedRun(sr.id)));

            let destination = db_state
                .trees()
                .flat_map(|tree| tree.compacted.iter().map(|sr| sr.id))
                .max()
                .map(|id| id.saturating_add(1))
                .unwrap_or(0);

            vec![CompactionSpec::for_segment(
                self.segment.clone(),
                sources,
                destination,
            )]
        }
    }

    struct SegmentTestSchedulerSupplier {
        scheduler: SegmentTestScheduler,
    }

    impl SegmentTestSchedulerSupplier {
        fn new(segment: Bytes, min_l0_sources: usize) -> Self {
            Self {
                scheduler: SegmentTestScheduler {
                    segment,
                    min_l0_sources,
                },
            }
        }
    }

    impl CompactionSchedulerSupplier for SegmentTestSchedulerSupplier {
        fn compaction_scheduler(
            &self,
            _options: &CompactorOptions,
        ) -> Box<dyn CompactionScheduler + Send + Sync> {
            Box::new(self.scheduler.clone())
        }
    }

    /// Test scheduler that proposes a single drain spec for the target
    /// segment, listing every observed L0 and SR as a source. Used to drive
    /// the segment-drain e2e path; there is no admin-triggered drain API.
    #[derive(Clone)]
    struct SegmentDrainTestScheduler {
        segment: Bytes,
    }

    impl CompactionScheduler for SegmentDrainTestScheduler {
        fn propose(&self, state: &CompactorStateView) -> Vec<CompactionSpec> {
            let db_state = state.manifest().core();
            let Some(tree) = db_state.tree_for_segment(self.segment.as_ref()) else {
                return vec![];
            };
            if tree.l0.is_empty() && tree.compacted.is_empty() {
                return vec![];
            }
            let active_drain = state
                .compactions()
                .map(|compactions| {
                    compactions.recent_compactions().any(|c| {
                        c.active() && c.spec().is_drain() && c.spec().segment() == &self.segment
                    })
                })
                .unwrap_or(false);
            if active_drain {
                return vec![];
            }
            let mut sources: Vec<SourceId> = tree
                .l0
                .iter()
                .map(|view| SourceId::SstView(view.id))
                .collect();
            sources.extend(tree.compacted.iter().map(|sr| SourceId::SortedRun(sr.id)));
            vec![CompactionSpec::drain_segment(self.segment.clone(), sources)]
        }
    }

    struct SegmentDrainTestSchedulerSupplier {
        scheduler: SegmentDrainTestScheduler,
    }

    impl SegmentDrainTestSchedulerSupplier {
        fn new(segment: Bytes) -> Self {
            Self {
                scheduler: SegmentDrainTestScheduler { segment },
            }
        }
    }

    impl CompactionSchedulerSupplier for SegmentDrainTestSchedulerSupplier {
        fn compaction_scheduler(
            &self,
            _options: &CompactorOptions,
        ) -> Box<dyn CompactionScheduler + Send + Sync> {
            Box::new(self.scheduler.clone())
        }
    }

    struct StringConcatMergeOperator;

    impl MergeOperator for StringConcatMergeOperator {
        fn merge(
            &self,
            _key: &Bytes,
            existing_value: Option<Bytes>,
            value: Bytes,
        ) -> Result<Bytes, MergeOperatorError> {
            let mut result = existing_value.unwrap_or_default().as_ref().to_vec();
            result.extend_from_slice(&value);
            Ok(Bytes::from(result))
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_compactor_compacts_l0() {
        // given:
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let mut options = db_options(Some(compactor_options()));
        // Bigger than all writes so we keep all writes in a single SST.
        // This makes it easier to verify all KV pairs are in that SST.
        options.l0_sst_size_bytes = 512;
        let scheduler_options = SizeTieredCompactionSchedulerOptions {
            min_compaction_sources: 1,
            max_compaction_sources: 999,
            include_size_threshold: 4.0,
            sorted_run_consolidation_threshold: 0,
        }
        .into();
        options
            .compactor_options
            .as_mut()
            .expect("compactor options must be set")
            .scheduler_options = scheduler_options;

        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .build()
            .await
            .unwrap();

        let (_, _, table_store) = build_test_stores(os.clone());
        let mut expected = HashMap::<Vec<u8>, Vec<u8>>::new();
        for i in 0..4 {
            let k = vec![b'a' + i as u8; 16];
            let v = vec![b'b' + i as u8; 48];
            expected.insert(k.clone(), v.clone());
            db.put_with_options(
                &k,
                &v,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            let k = vec![b'j' + i as u8; 16];
            let v = vec![b'k' + i as u8; 48];
            db.put_with_options(
                &k,
                &v,
                &PutOptions::default(),
                &WriteOptions {
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            expected.insert(k.clone(), v.clone());
        }

        // Force all writes to a single L0 SST.
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        // when:
        let db_state = await_compaction(&db, os.clone(), Some(system_clock.clone())).await;

        // then:
        let db_state = db_state.expect("db was not compacted");
        for run in db_state.tree.compacted.iter() {
            for sst in run.sst_views() {
                let mut iter = SstIterator::new_borrowed_initialized(
                    ..,
                    sst,
                    table_store.clone(),
                    SstIteratorOptions::default(),
                )
                .await
                .unwrap()
                .expect("Expected Some(iter) but got None");

                // remove the key from the expected map and verify that the db matches
                while let Some(kv) = iter.next().await.unwrap().map(KeyValue::from) {
                    let expected_v = expected
                        .remove(kv.key.as_ref())
                        .expect("removing unexpected key");
                    let db_v = db.get(kv.key.as_ref()).await.unwrap().unwrap();
                    assert_eq!(expected_v, db_v.as_ref());
                }
            }
        }
        assert!(expected.is_empty());
    }

    /// An entry expected in the block cache for a compaction output SST.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ExpectedEntry {
        Index,
        Filter,
        Stats,
        /// The data block at this position in the SST index.
        DataBlock(usize),
    }

    fn data_blocks(positions: Range<usize>) -> Vec<ExpectedEntry> {
        positions.map(ExpectedEntry::DataBlock).collect()
    }

    #[rstest]
    #[case::filters_only(
        BlockCachePolicy::default().with_compaction_output_targets(&[CacheTarget::Filters]),
        vec![ExpectedEntry::Filter]
    )]
    #[case::default_policy(
        BlockCachePolicy::default(),
        vec![ExpectedEntry::Index, ExpectedEntry::Filter]
    )]
    #[case::cache_everything(
        BlockCachePolicy::default().with_compaction_output_targets(&[
            CacheTarget::data::<&[u8], _>(..),
            CacheTarget::Index,
            CacheTarget::Filters,
            CacheTarget::Stats,
        ]),
        [
            vec![ExpectedEntry::Index, ExpectedEntry::Filter, ExpectedEntry::Stats],
            data_blocks(0..4),
        ].concat()
    )]
    #[case::data_range(
        BlockCachePolicy::default().with_compaction_output_targets(&[
            CacheTarget::data(b"b".as_slice()..=b"c".as_slice()),
            CacheTarget::Index,
        ]),
        vec![ExpectedEntry::Index, ExpectedEntry::DataBlock(1), ExpectedEntry::DataBlock(2)]
    )]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_compactor_applies_output_cache_policy(
        #[case] policy: BlockCachePolicy,
        #[case] expected_entries: Vec<ExpectedEntry>,
    ) {
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let cache = Arc::new(TestCache::new());
        let mut options = db_options(Some(compactor_options()));
        options.flush_interval = None;
        // Ensure that filter is built.
        options.min_filter_keys = 1;
        options
            .compactor_options
            .as_mut()
            .expect("compactor options must be set")
            .scheduler_options = SizeTieredCompactionSchedulerOptions {
            // Compact even a single L0 so one flush is enough to trigger.
            min_compaction_sources: 1,
            ..Default::default()
        }
        .into();

        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            // One data block per entry, so a data range selects a subset.
            .with_sst_block_size(SstBlockSize::Other(1))
            .with_system_clock(system_clock.clone())
            .with_db_cache(cache.clone(), 0)
            .with_block_cache_policy(policy)
            .build()
            .await
            .unwrap();

        // Keep all entries in one memtable so the explicit flush produces one
        // L0 SST regardless of the configured memtable size threshold.
        let mut batch = WriteBatch::new();
        for key in [b"a", b"b", b"c", b"d"] {
            batch.put(key, b"value");
        }
        db.write_with_options(batch, &WriteOptions::default())
            .await
            .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        let db_state = await_compaction(&db, os.clone(), Some(system_clock))
            .await
            .expect("db was not compacted");

        let output_ssts: Vec<_> = db_state
            .tree
            .compacted
            .iter()
            .flat_map(|sr| sr.sst_views().iter())
            .collect();
        assert_eq!(output_ssts.len(), 1);
        let view = output_ssts[0];
        let info = &view.sst.info;

        let (_, _, table_store) = build_test_stores(os);
        let index = table_store
            .read_index(
                &view.sst,
                false,
                Some(Bytes::new()),
                &crate::reader::ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        let block_metas = index.borrow().block_meta();
        assert_eq!(block_metas.len(), 4);

        let mut expected_ids: Vec<u64> = expected_entries
            .iter()
            .map(|entry| match entry {
                ExpectedEntry::Index => info.index_offset,
                ExpectedEntry::Filter => info.filter_offset,
                ExpectedEntry::Stats => info.stats_offset,
                ExpectedEntry::DataBlock(position) => block_metas.get(*position).offset(),
            })
            .collect();
        expected_ids.sort();

        // Entries written by the flush carry the L0 SST's id, so filtering on
        // the output id leaves only compaction output entries.
        let mut cached_ids: Vec<u64> = cache
            .keys()
            .iter()
            .filter(|key| key.sst_id == view.sst.id)
            .map(|key| key.block_id)
            .collect();
        cached_ids.sort();

        assert_eq!(cached_ids, expected_ids);
        db.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_compactor_compacts_only_target_segment() {
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let path = "/tmp/test_compactor_compacts_only_target_segment";
        let mut options = db_options(Some(compactor_options()));
        options.l0_sst_size_bytes = 512;
        options.flush_interval = None;

        let db = Db::builder(path, os.clone())
            .with_settings(options.clone())
            .with_system_clock(system_clock.clone())
            .with_segment_extractor(Arc::new(FixedThreeBytePrefixExtractor))
            .with_compactor_builder(
                CompactorBuilder::new(path, os.clone())
                    .with_options(compactor_options())
                    .with_scheduler_supplier(Arc::new(SegmentTestSchedulerSupplier::new(
                        Bytes::from_static(b"aaa"),
                        // Must match the number of aaa keys flushed below. The
                        // compactor's first poll tick fires immediately and can
                        // land mid-flush; a lower threshold lets it compact a
                        // subset of the aaa L0s and strand the rest, since the
                        // scheduler never proposes with fewer than this many L0s.
                        3,
                    ))),
            )
            .build()
            .await
            .unwrap();

        // bbb gets as many L0s as aaa, so it would qualify for compaction if
        // segment scoping were ignored; it must still be left alone.
        for (key, value) in [
            (b"aaa-001".as_slice(), b"v1".as_slice()),
            (b"aaa-002".as_slice(), b"v2".as_slice()),
            (b"aaa-003".as_slice(), b"v3".as_slice()),
            (b"bbb-001".as_slice(), b"v4".as_slice()),
            (b"bbb-002".as_slice(), b"v5".as_slice()),
            (b"bbb-003".as_slice(), b"v6".as_slice()),
        ] {
            put_and_flush_memtable(&db, key, value).await;
        }

        let db_state = run_for(Duration::from_secs(10), || async {
            system_clock
                .as_ref()
                .advance(Duration::from_millis(60000))
                .await;
            let core = read_db_state_core(&db);
            let aaa = core
                .segments
                .iter()
                .find(|segment| segment.prefix.as_ref() == b"aaa")
                .expect("missing segment aaa");
            let bbb = core
                .segments
                .iter()
                .find(|segment| segment.prefix.as_ref() == b"bbb")
                .expect("missing segment bbb");
            if aaa.tree.l0.is_empty()
                && !aaa.tree.compacted.is_empty()
                && bbb.tree.l0.len() == 3
                && bbb.tree.compacted.is_empty()
            {
                Some(core)
            } else {
                None
            }
        })
        .await
        .expect("segment-scoped compaction did not complete");

        let aaa = db_state
            .segments
            .iter()
            .find(|segment| segment.prefix.as_ref() == b"aaa")
            .expect("missing segment aaa");
        let bbb = db_state
            .segments
            .iter()
            .find(|segment| segment.prefix.as_ref() == b"bbb")
            .expect("missing segment bbb");
        assert!(aaa.tree.l0.is_empty());
        assert_eq!(aaa.tree.compacted.len(), 1);
        assert_eq!(bbb.tree.l0.len(), 3);
        assert!(bbb.tree.compacted.is_empty());

        for (key, value) in [
            (b"aaa-001".as_slice(), b"v1".as_slice()),
            (b"aaa-002".as_slice(), b"v2".as_slice()),
            (b"aaa-003".as_slice(), b"v3".as_slice()),
            (b"bbb-001".as_slice(), b"v4".as_slice()),
            (b"bbb-002".as_slice(), b"v5".as_slice()),
            (b"bbb-003".as_slice(), b"v6".as_slice()),
        ] {
            assert_eq!(
                db.get(key).await.unwrap(),
                Some(Bytes::copy_from_slice(value))
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_compactor_compacts_all_segments_with_size_tiered_scheduler() {
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let path = "/tmp/test_compactor_compacts_all_segments_with_size_tiered_scheduler";
        let mut options = db_options(Some(compactor_options()));
        options.l0_sst_size_bytes = 512;
        options.flush_interval = None;
        let scheduler_options = SizeTieredCompactionSchedulerOptions {
            min_compaction_sources: 2,
            max_compaction_sources: 999,
            include_size_threshold: 4.0,
            sorted_run_consolidation_threshold: 0,
        }
        .into();
        let compactor_opts = options
            .compactor_options
            .as_mut()
            .expect("compactor options must be set");
        compactor_opts.scheduler_options = scheduler_options;
        compactor_opts.max_concurrent_compactions = 3;

        let db = Db::builder(path, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .with_segment_extractor(Arc::new(FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();

        let entries: &[(&[u8], &[u8])] = &[
            (b"aaa-001", b"v1"),
            (b"aaa-002", b"v2"),
            (b"bbb-001", b"v3"),
            (b"bbb-002", b"v4"),
            (b"ccc-001", b"v5"),
            (b"ccc-002", b"v6"),
        ];

        for (key, value) in entries {
            put_and_flush_memtable(&db, key, value).await;
        }

        let prefixes: [&[u8]; 3] = [b"aaa", b"bbb", b"ccc"];

        let db_state = run_for(Duration::from_secs(10), || async {
            system_clock
                .as_ref()
                .advance(Duration::from_millis(60000))
                .await;
            let core = read_db_state_core(&db);
            let all_compacted = prefixes.iter().all(|prefix| {
                core.segments
                    .iter()
                    .find(|segment| segment.prefix.as_ref() == *prefix)
                    .map(|segment| segment.tree.l0.is_empty() && !segment.tree.compacted.is_empty())
                    .unwrap_or(false)
            });
            if all_compacted {
                Some(core)
            } else {
                None
            }
        })
        .await
        .expect("not every segment compacted via size-tiered scheduler");

        for prefix in prefixes {
            let segment = db_state
                .segments
                .iter()
                .find(|segment| segment.prefix.as_ref() == prefix)
                .unwrap_or_else(|| panic!("missing segment {:?}", prefix));
            assert!(
                segment.tree.l0.is_empty(),
                "segment {:?} still has L0 SSTs",
                prefix
            );
            assert_eq!(
                segment.tree.compacted.len(),
                1,
                "segment {:?} should have a single sorted run",
                prefix
            );
        }

        for (key, value) in entries {
            assert_eq!(
                db.get(key).await.unwrap(),
                Some(Bytes::copy_from_slice(value))
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_compactor_drains_segment() {
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let path = "/tmp/test_compactor_drains_segment";
        let mut options = db_options(Some(compactor_options()));
        options.l0_sst_size_bytes = 512;
        options.flush_interval = None;

        let db = Db::builder(path, os.clone())
            .with_settings(options.clone())
            .with_system_clock(system_clock.clone())
            .with_segment_extractor(Arc::new(FixedThreeBytePrefixExtractor))
            .with_compactor_builder(
                CompactorBuilder::new(path, os.clone())
                    .with_options(compactor_options())
                    .with_scheduler_supplier(Arc::new(SegmentDrainTestSchedulerSupplier::new(
                        Bytes::from_static(b"aaa"),
                    ))),
            )
            .build()
            .await
            .unwrap();

        for (key, value) in [
            (b"aaa-001".as_slice(), b"v1".as_slice()),
            (b"aaa-002".as_slice(), b"v2".as_slice()),
            (b"bbb-001".as_slice(), b"v3".as_slice()),
        ] {
            put_and_flush_memtable(&db, key, value).await;
        }

        // The marker is pruned on the writer's next commit, so periodically
        // write a fresh bbb-* key to nudge the writer-side merge.
        let nudge = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let db_state = run_for(Duration::from_secs(10), || {
            let db = &db;
            let system_clock = system_clock.clone();
            let nudge_value = nudge.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            async move {
                system_clock
                    .as_ref()
                    .advance(Duration::from_millis(60000))
                    .await;
                let key = format!("bbb-nudge-{:04}", nudge_value);
                put_and_flush_memtable(db, key.as_bytes(), b"nudge").await;
                let core = read_db_state_core(db);
                let aaa_gone = !core
                    .segments
                    .iter()
                    .any(|segment| segment.prefix.as_ref() == b"aaa");
                let bbb_present = core
                    .segments
                    .iter()
                    .any(|segment| segment.prefix.as_ref() == b"bbb");
                if aaa_gone && bbb_present {
                    Some(core)
                } else {
                    None
                }
            }
        })
        .await
        .expect("drain marker for segment aaa was not pruned");

        assert!(
            !db_state
                .segments
                .iter()
                .any(|segment| segment.prefix.as_ref() == b"aaa"),
            "aaa segment should be gone from manifest"
        );
        let bbb = db_state
            .segments
            .iter()
            .find(|segment| segment.prefix.as_ref() == b"bbb")
            .expect("missing segment bbb");
        assert!(
            !bbb.tree.l0.is_empty() || !bbb.tree.compacted.is_empty(),
            "bbb segment should still hold data"
        );

        assert_eq!(db.get(b"aaa-001").await.unwrap(), None);
        assert_eq!(db.get(b"aaa-002").await.unwrap(), None);
        assert_eq!(
            db.get(b"bbb-001").await.unwrap(),
            Some(Bytes::from_static(b"v3"))
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_scan_across_segments_after_compaction() {
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let path = "/tmp/test_scan_across_segments_after_compaction";
        let mut options = db_options(Some(compactor_options()));
        options.l0_sst_size_bytes = 512;
        options.flush_interval = None;
        let scheduler_options = SizeTieredCompactionSchedulerOptions {
            min_compaction_sources: 2,
            max_compaction_sources: 999,
            include_size_threshold: 4.0,
            sorted_run_consolidation_threshold: 0,
        }
        .into();
        let compactor_opts = options
            .compactor_options
            .as_mut()
            .expect("compactor options must be set");
        compactor_opts.scheduler_options = scheduler_options;
        compactor_opts.max_concurrent_compactions = 3;

        let db = Db::builder(path, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .with_segment_extractor(Arc::new(FixedThreeBytePrefixExtractor))
            .build()
            .await
            .unwrap();

        // Two keys per segment, each flushed to its own L0 SST. Written in
        // intra-segment lex order so the post-compaction scan order is
        // deterministic without re-sorting on the test side.
        let entries: Vec<(Vec<u8>, Vec<u8>)> = [
            b"aaa-001".as_slice(),
            b"aaa-002",
            b"bbb-001",
            b"bbb-002",
            b"ccc-001",
            b"ccc-002",
        ]
        .iter()
        .enumerate()
        .map(|(i, k)| (k.to_vec(), format!("v{}", i + 1).into_bytes()))
        .collect();

        for (key, value) in &entries {
            put_and_flush_memtable(&db, key, value).await;
        }

        let prefixes: [&[u8]; 3] = [b"aaa", b"bbb", b"ccc"];
        let _db_state = run_for(Duration::from_secs(10), || async {
            system_clock
                .as_ref()
                .advance(Duration::from_millis(60000))
                .await;
            let core = read_db_state_core(&db);
            let all_compacted = prefixes.iter().all(|prefix| {
                core.segments
                    .iter()
                    .find(|segment| segment.prefix.as_ref() == *prefix)
                    .map(|segment| segment.tree.l0.is_empty() && !segment.tree.compacted.is_empty())
                    .unwrap_or(false)
            });
            if all_compacted {
                Some(core)
            } else {
                None
            }
        })
        .await
        .expect("not every segment compacted before scan");

        // Full ascending scan crosses every segment in prefix order.
        let mut iter = db.scan(..).await.unwrap();
        let mut collected: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        while let Some(kv) = iter.next().await.unwrap() {
            collected.push((kv.key.to_vec(), kv.value.to_vec()));
        }
        assert_eq!(
            collected, entries,
            "full scan should yield every key in order"
        );

        // Per-segment prefix scan should return only that segment's keys.
        for prefix in prefixes {
            let mut iter = db.scan_prefix(prefix, ..).await.unwrap();
            let mut keys: Vec<Vec<u8>> = Vec::new();
            while let Some(kv) = iter.next().await.unwrap() {
                keys.push(kv.key.to_vec());
            }
            let expected: Vec<Vec<u8>> = entries
                .iter()
                .filter(|(k, _)| k.starts_with(prefix))
                .map(|(k, _)| k.clone())
                .collect();
            assert_eq!(keys, expected, "prefix scan for {:?}", prefix);
        }

        // Cross-segment range scan: spans the tail of aaa, all of bbb, and stops
        // before the head of ccc.
        let mut iter = db
            .scan(b"aaa-002".to_vec()..b"ccc-001".to_vec())
            .await
            .unwrap();
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while let Some(kv) = iter.next().await.unwrap() {
            keys.push(kv.key.to_vec());
        }
        assert_eq!(
            keys,
            vec![
                b"aaa-002".to_vec(),
                b"bbb-001".to_vec(),
                b"bbb-002".to_vec(),
            ],
            "range scan spanning aaa and bbb"
        );
    }

    #[cfg(feature = "wal_disable")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_should_tombstones_in_l0() {
        use crate::test_utils::OnDemandCompactionSchedulerSupplier;

        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());

        let scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            |state| {
                // compact when there are at least 2 SSTs in L0 (one for key 'a' and one for key 'b')
                state.manifest().core().tree.l0.len() == 2 ||
                // or when there is one SST in L0 and one in L1 (one for delete key 'a' and one for compacted key 'a'+'b')
                (state.manifest().core().tree.l0.len() == 1
                    && state.manifest().core().tree.compacted.len() == 1)
            },
        )));

        let mut options = db_options(None);
        options.wal_enabled = false;
        options.l0_sst_size_bytes = 128;

        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .with_compactor_builder(compactor_builder_with_scheduler(
                os.clone(),
                scheduler.clone(),
                system_clock.clone(),
            ))
            .build()
            .await
            .unwrap();

        let (manifest_store, _, table_store) = build_test_stores(os.clone());

        // Put key 'a' into L1 (and key 'b' so that when we delete 'a' the SST is non-empty).
        // The explicit flush below makes both writes durable.
        db.put(&[b'a'; 16], &[b'a'; 32]).await.unwrap();
        db.put(&[b'b'; 16], &[b'a'; 32]).await.unwrap();
        db.flush().await.unwrap();
        let db_state = await_compaction(&db, os.clone(), Some(system_clock.clone()))
            .await
            .unwrap();
        assert_eq!(db_state.tree.compacted.len(), 1);
        assert_eq!(db_state.tree.l0.len(), 0, "{:?}", db_state.tree.l0);

        // put tombstone for key a into L0
        db.delete_with_options(
            &[b'a'; 16],
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush().await.unwrap();

        // Then:
        // we should now have a tombstone in L0 and a value in L1
        let db_state = get_db_state(manifest_store.clone()).await;
        assert_eq!(db_state.tree.l0.len(), 1, "{:?}", db_state.tree.l0);
        assert_eq!(db_state.tree.compacted.len(), 1);

        let l0 = db_state.tree.l0.front().unwrap();
        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            l0,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        let tombstone = iter.next().await.unwrap();
        assert!(tombstone.unwrap().value.is_tombstone());

        let db_state = await_compacted_compaction(
            manifest_store.clone(),
            db_state.tree.compacted.clone(),
            Some(system_clock.clone()),
        )
        .await
        .unwrap();
        assert_eq!(db_state.tree.compacted.len(), 1);

        let compacted = db_state.tree.compacted.first().unwrap().sst_views();
        assert_eq!(compacted.len(), 1);
        let handle = compacted.first().unwrap();

        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            handle,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // should be no tombstone for key 'a' because it was filtered
        // out of the last run
        let next = iter.next().await.unwrap().map(KeyValue::from);
        assert_eq!(next.unwrap().key.as_ref(), &[b'b'; 16]);
        let next = iter.next().await.unwrap().map(KeyValue::from);
        assert!(next.is_none());
    }

    #[cfg(feature = "wal_disable")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_should_not_filter_tombstone_with_snapshot() {
        use crate::test_utils::OnDemandCompactionSchedulerSupplier;

        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());

        let scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            |state| {
                // compact when there are at least 2 SSTs in L0
                state.manifest().core().tree.l0.len() == 2 ||
                // or when there is one SST in L0 and one in L1
                (state.manifest().core().tree.l0.len() == 1
                    && state.manifest().core().tree.compacted.len() == 1)
            },
        )));

        let mut options = db_options(None);
        options.wal_enabled = false;
        options.l0_sst_size_bytes = 128;

        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .with_compactor_builder(compactor_builder_with_scheduler(
                os.clone(),
                scheduler.clone(),
                system_clock.clone(),
            ))
            .build()
            .await
            .unwrap();

        let (manifest_store, _, table_store) = build_test_stores(os.clone());

        // Write and flush key 'a' first (seq=1)
        db.put(&[b'a'; 16], &[b'a'; 32]).await.unwrap();
        db.put(&[b'b'; 16], &[b'a'; 32]).await.unwrap();

        // Create a snapshot after first flush. This protects seq >= 1
        let _snapshot = db.snapshot().await.unwrap();
        db.flush().await.unwrap();

        // Compact L0 to L1
        let db_state = await_compaction(&db, os.clone(), Some(system_clock.clone()))
            .await
            .unwrap();
        assert_eq!(db_state.tree.compacted.len(), 1);
        assert_eq!(db_state.tree.l0.len(), 0, "{:?}", db_state.tree.l0);

        // Now delete key 'a', creating a tombstone (seq=3)
        db.delete_with_options(
            &[b'a'; 16],
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush().await.unwrap();

        // We should now have a tombstone for 'a' in L0 and both 'a' and 'b' values in L1
        let db_state = get_db_state(manifest_store.clone()).await;
        assert_eq!(db_state.tree.l0.len(), 1, "{:?}", db_state.tree.l0);
        assert_eq!(db_state.tree.compacted.len(), 1);

        // Trigger compaction of L0 (tombstone) + L1 (values)
        let db_state = await_compacted_compaction(
            manifest_store.clone(),
            db_state.tree.compacted.clone(),
            Some(system_clock.clone()),
        )
        .await
        .unwrap();
        assert_eq!(db_state.tree.compacted.len(), 1);

        let compacted = db_state.tree.compacted.first().unwrap().sst_views();
        assert_eq!(compacted.len(), 1);
        let handle = compacted.first().unwrap();

        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            handle,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // After compaction with an active snapshot (protecting seq >= 1):
        // - Key 'a': Both the tombstone(seq=3) and original value(seq=1) are protected
        //   The SST contains both versions
        // - Key 'b': value(seq=2) is protected
        // Expected result: both 'a' (as tombstone) and 'b' (as value) should be present
        // Note: next() returns all entries including tombstones and duplicates,
        // so we need to track whether we've seen each key and verify the expected type
        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next.key.as_ref(), &[b'a'; 16]);
        assert!(next.value.is_tombstone());

        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next.key.as_ref(), &[b'a'; 16]);
        assert_eq!(next.value.as_bytes().unwrap().as_ref(), &[b'a'; 32]);

        let next = iter.next().await.unwrap().unwrap();
        assert_eq!(next.key.as_ref(), &[b'b'; 16]);
        assert_eq!(next.value.as_bytes().unwrap().as_ref(), &[b'a'; 32]);

        let next = iter.next().await.unwrap();
        assert!(next.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_apply_merge_during_l0_compaction() {
        use crate::test_utils::OnDemandCompactionSchedulerSupplier;

        // given:
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            |state| state.manifest().core().tree.l0.len() >= 2,
        )));
        let options = db_options(None);

        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .with_compactor_builder(compactor_builder_with_scheduler(
                os.clone(),
                compaction_scheduler.clone(),
                system_clock.clone(),
            ))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        let (_manifest_store, _, table_store) = build_test_stores(os.clone());

        // write merge operations across multiple L0 SSTs
        db.merge_with_options(
            b"key1",
            b"a",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.merge_with_options(
            b"key1",
            b"b",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            &vec![b'x'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush().await.unwrap();

        db.merge_with_options(
            b"key1",
            b"c",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.merge_with_options(
            b"key2",
            b"x",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            &vec![b'y'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush().await.unwrap();

        // when:
        let db_state = await_compaction(&db, os.clone(), Some(system_clock)).await;

        // then:
        let db_state = db_state.expect("db was not compacted");
        assert_eq!(db_state.tree.compacted.len(), 1);
        let compacted = db_state.tree.compacted.first().unwrap().sst_views();
        assert_eq!(compacted.len(), 1);
        let handle = compacted.first().unwrap();

        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            handle,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_merge(b"key1", b"abc", 4).with_create_ts(0),
                RowEntry::new_merge(b"key2", b"x", 5).with_create_ts(0),
                RowEntry::new_value(&[b'x'; 16], &[b'p'; 128], 3).with_create_ts(0),
                RowEntry::new_value(&[b'y'; 16], &[b'p'; 128], 6).with_create_ts(0),
            ],
        )
        .await;

        // verify the merged values are readable from the db
        let result = db.get(b"key1").await.unwrap();
        assert_eq!(result, Some(Bytes::from("abc")));
        let result = db.get(b"key2").await.unwrap();
        assert_eq!(result, Some(Bytes::from("x")));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_record_merge_operator_operands_on_compact_path() {
        use crate::merge_operator::MERGE_OPERATOR_COMPACT_PATH;
        use crate::test_utils::{
            lookup_merge_operator_operands, OnDemandCompactionSchedulerSupplier,
        };
        use slatedb_common::metrics::DefaultMetricsRecorder;

        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            |state| state.manifest().core().tree.l0.len() >= 2,
        )));
        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());

        let db = Db::builder(PATH, os.clone())
            .with_settings(db_options(None))
            .with_system_clock(system_clock.clone())
            .with_metrics_recorder(metrics_recorder.clone())
            .with_compactor_builder(compactor_builder_with_scheduler(
                os.clone(),
                compaction_scheduler,
                system_clock.clone(),
            ))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        assert_eq!(
            lookup_merge_operator_operands(&metrics_recorder, MERGE_OPERATOR_COMPACT_PATH),
            Some(0)
        );

        db.merge_with_options(
            b"key1",
            b"a",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        db.merge_with_options(
            b"key1",
            b"b",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();

        let db_state = await_compaction(&db, os.clone(), Some(system_clock)).await;
        assert!(db_state.is_some(), "db was not compacted");
        assert!(
            lookup_merge_operator_operands(&metrics_recorder, MERGE_OPERATOR_COMPACT_PATH)
                .is_some_and(|value| value > 0)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_apply_merge_across_l0_and_sorted_runs() {
        use crate::test_utils::OnDemandCompactionSchedulerSupplier;

        // given:
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            |state| !state.manifest().core().tree.l0.is_empty(),
        )));
        let options = db_options(None);

        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .with_compactor_builder(compactor_builder_with_scheduler(
                os.clone(),
                compaction_scheduler.clone(),
                system_clock.clone(),
            ))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        let (_manifest_store, _, table_store) = build_test_stores(os.clone());

        // write initial merge operations and compact to L1
        db.merge_with_options(
            b"key1",
            b"a",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.merge_with_options(
            b"key1",
            b"b",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            &vec![b'x'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap(); // padding to exceed 256 bytes
        db.flush().await.unwrap();

        let db_state = await_compaction(&db, os.clone(), Some(system_clock.clone())).await;
        let db_state = db_state.expect("db was not compacted");
        assert_eq!(db_state.tree.compacted.len(), 1);
        // Save current tick since we advanced it in `await_compaction`. We'll use it
        // later to verify the create_ts of the merged and normal entries.
        let expected_tick = system_clock.now().timestamp_millis();

        // write more merge operations to L0
        db.merge_with_options(
            b"key1",
            b"c",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.merge_with_options(
            b"key1",
            b"d",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            &vec![b'y'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap(); // padding to exceed 256 bytes
        db.flush().await.unwrap();

        // when: compact L0 with the existing sorted run
        let db_state = await_compaction(&db, os.clone(), Some(system_clock)).await;

        // then:
        let db_state = db_state.expect("db was not compacted");
        assert_eq!(db_state.tree.compacted.len(), 1);
        let compacted = db_state.tree.compacted.first().unwrap().sst_views();
        assert_eq!(compacted.len(), 1);
        let handle = compacted.first().unwrap();

        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            handle,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_merge(b"key1", b"abcd", 5).with_create_ts(expected_tick),
                RowEntry::new_value(&[b'x'; 16], &[b'p'; 128], 3).with_create_ts(0),
                RowEntry::new_value(&[b'y'; 16], &[b'p'; 128], 6).with_create_ts(expected_tick),
            ],
        )
        .await;

        // verify the merged value is readable from the db
        let result = db.get(b"key1").await.unwrap();
        assert_eq!(result, Some(Bytes::from("abcd")));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_merge_without_base_value() {
        use crate::test_utils::OnDemandCompactionSchedulerSupplier;

        // given:
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            |state| state.manifest().core().tree.l0.len() >= 2,
        )));
        let options = db_options(None);

        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .with_compactor_builder(compactor_builder_with_scheduler(
                os.clone(),
                compaction_scheduler.clone(),
                system_clock.clone(),
            ))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        let (_manifest_store, _, table_store) = build_test_stores(os.clone());

        // write only merge operations without any base value
        db.merge_with_options(
            b"key1",
            b"x",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap(); // padding to exceed 256 bytes
        db.put_with_options(
            &vec![b'x'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap(); // padding to exceed 256 bytes
        db.flush().await.unwrap();

        db.merge_with_options(
            b"key1",
            b"y",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.merge_with_options(
            b"key1",
            b"z",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            &vec![b'y'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap(); // padding to exceed 256 bytes
        db.flush().await.unwrap();

        // when:
        let db_state = await_compaction(&db, os.clone(), Some(system_clock)).await;

        // then:
        let db_state = db_state.expect("db was not compacted");
        assert_eq!(db_state.tree.compacted.len(), 1);
        let compacted = db_state.tree.compacted.first().unwrap().sst_views();
        assert_eq!(compacted.len(), 1);
        let handle = compacted.first().unwrap();

        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            handle,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_merge(b"key1", b"xyz", 4).with_create_ts(0),
                RowEntry::new_value(&[b'x'; 16], &[b'p'; 128], 2).with_create_ts(0),
                RowEntry::new_value(&[b'y'; 16], &[b'p'; 128], 5).with_create_ts(0),
            ],
        )
        .await;

        // verify the merged value is readable from the db
        let result = db.get(b"key1").await.unwrap();
        assert_eq!(result, Some(Bytes::from("xyz")));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_preserve_merge_order_across_multiple_ssts() {
        use crate::test_utils::OnDemandCompactionSchedulerSupplier;

        // given:
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            |state| state.manifest().core().tree.l0.len() >= 3,
        )));
        let options = db_options(None);

        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .with_compactor_builder(compactor_builder_with_scheduler(
                os.clone(),
                compaction_scheduler.clone(),
                system_clock.clone(),
            ))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        let (_manifest_store, _, table_store) = build_test_stores(os.clone());

        // write merge operations across three L0 SSTs in specific order
        db.merge_with_options(
            b"key1",
            b"1",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            &vec![b'a'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap(); // padding to exceed 256 bytes
        db.flush().await.unwrap();

        db.merge_with_options(
            b"key1",
            b"2",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            &vec![b'b'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap(); // padding to exceed 256 bytes
        db.flush().await.unwrap();

        db.merge_with_options(
            b"key1",
            b"3",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            &vec![b'c'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap(); // padding to exceed 256 bytes
        db.flush().await.unwrap();

        // when:
        let db_state = await_compaction(&db, os.clone(), Some(system_clock)).await;

        // then:
        let db_state = db_state.expect("db was not compacted");
        assert_eq!(db_state.tree.compacted.len(), 1);
        let compacted = db_state.tree.compacted.first().unwrap().sst_views();
        assert_eq!(compacted.len(), 1);
        let handle = compacted.first().unwrap();

        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            handle,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // merges should be applied in chronological order: 1, 2, 3
        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_value(&[b'a'; 16], &[b'p'; 128], 2).with_create_ts(0),
                RowEntry::new_value(&[b'b'; 16], &[b'p'; 128], 4).with_create_ts(0),
                RowEntry::new_value(&[b'c'; 16], &[b'p'; 128], 6).with_create_ts(0),
                RowEntry::new_merge(b"key1", b"123", 5).with_create_ts(0),
            ],
        )
        .await;

        // verify the merged value is readable from the db
        let result = db.get(b"key1").await.unwrap();
        assert_eq!(result, Some(Bytes::from("123")));
    }

    #[cfg(feature = "wal_disable")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_not_compact_expired_merge_operations_in_last_run() {
        use crate::test_utils::OnDemandCompactionSchedulerSupplier;

        // given:
        let os = Arc::new(InMemory::new());
        let insert_clock = Arc::new(MockSystemClock::new());

        let scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            |state| state.manifest().core().tree.l0.len() >= 2,
        )));

        let mut options = db_options(None);
        options.wal_enabled = false;
        options.l0_sst_size_bytes = 128;

        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(insert_clock.clone())
            .with_compactor_builder(compactor_builder_with_scheduler(
                os.clone(),
                scheduler.clone(),
                insert_clock.clone(),
            ))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        let (_manifest_store, _, table_store) = build_test_stores(os.clone());

        // ticker time = 0, expire time = 10
        insert_clock.set(0);
        db.merge_with_options(
            b"key1",
            &[b'a'; 32],
            &MergeOptions {
                ttl: Ttl::ExpireAfterMillis(10),
            },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .await_durable()
        .await
        .unwrap();

        // ticker time = 20, no expire time
        insert_clock.set(20);
        db.merge_with_options(
            b"key1",
            &[b'b'; 32],
            &MergeOptions { ttl: Ttl::NoExpiry },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .await_durable()
        .await
        .unwrap();

        let db_state = await_compaction(&db, os.clone(), Some(insert_clock.clone()))
            .await
            .unwrap();
        assert_eq!(db_state.tree.compacted.len(), 1);
        assert_eq!(db_state.last_l0_clock_tick, 20);

        // then: the compacted SST should only contain the non-expired merge
        let compacted = db_state.tree.compacted.first().unwrap().sst_views();
        assert_eq!(compacted.len(), 1);
        let handle = compacted.first().unwrap();

        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            handle,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // only the non-expired merge "b" should be present
        assert_iterator(
            &mut iter,
            vec![RowEntry::new_merge(b"key1", &[b'b'; 32], 2).with_create_ts(20)],
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_merge_and_then_overwrite_with_put() {
        use crate::test_utils::OnDemandCompactionSchedulerSupplier;

        // given:
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            |state| state.manifest().core().tree.l0.len() >= 2,
        )));
        let options = db_options(None);

        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .with_compactor_builder(compactor_builder_with_scheduler(
                os.clone(),
                compaction_scheduler.clone(),
                system_clock.clone(),
            ))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        let (manifest_store, _compactions_store, _table_store) = build_test_stores(os.clone());

        // write merge operations
        db.merge_with_options(
            b"key1",
            b"a",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.merge_with_options(
            b"key1",
            b"b",
            &MergeOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            &vec![b'x'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap(); // padding
        db.flush().await.unwrap();

        // then overwrite with a put
        db.put_with_options(
            b"key1",
            b"new_value",
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            &vec![b'y'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap(); // padding
        db.flush().await.unwrap();

        // when:
        let _ = await_compaction(&db, os.clone(), Some(system_clock)).await;

        // then: verify the put value overwrote the merge
        let result = db.get(b"key1").await.unwrap();
        assert_eq!(result, Some(Bytes::from("new_value")));

        // verify the compacted state in manifest
        let stored_manifest =
            StoredManifest::load(manifest_store.clone(), Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let db_state = stored_manifest.db_state();
        assert!(
            !db_state.tree.compacted.is_empty(),
            "compaction should have occurred"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_not_merge_operations_with_different_expire_times() {
        use crate::test_utils::OnDemandCompactionSchedulerSupplier;

        // given:
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            |state| state.manifest().core().tree.l0.len() >= 2,
        )));
        let options = db_options(None);

        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .with_compactor_builder(compactor_builder_with_scheduler(
                os.clone(),
                compaction_scheduler.clone(),
                system_clock.clone(),
            ))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        let (manifest_store, _compactions_store, table_store) = build_test_stores(os.clone());

        // write merge operations with different TTLs
        db.merge_with_options(
            b"key1",
            b"a",
            &MergeOptions {
                ttl: Ttl::ExpireAfterMillis(100),
            },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            &vec![b'x'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap(); // padding
        db.flush().await.unwrap();

        db.merge_with_options(
            b"key1",
            b"b",
            &MergeOptions {
                ttl: Ttl::ExpireAfterMillis(200),
            },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.put_with_options(
            &vec![b'y'; 16],
            &vec![b'p'; 128],
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap(); // padding
        db.flush().await.unwrap();

        // when:
        // Wait a bit for compaction to occur
        let _ = await_compaction(&db, os.clone(), Some(system_clock)).await;

        // then: verify that merges with different expire times are NOT merged together
        // Reading should get only the latest merge since they weren't combined
        let stored_manifest =
            StoredManifest::load(manifest_store.clone(), Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let db_state = stored_manifest.db_state();
        assert!(
            !db_state.tree.compacted.is_empty(),
            "compaction should have occurred"
        );

        // The compacted sorted run should contain both merge operations separately
        let compacted = db_state.tree.compacted.first().unwrap().sst_views();
        assert_eq!(compacted.len(), 1);
        let handle = compacted.first().unwrap();

        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            handle,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // merge operations should be kept separate due to different expire times
        let mut key1_entries = vec![];
        while let Some(entry) = iter.next().await.unwrap() {
            if entry.key.as_ref() == b"key1" {
                key1_entries.push(entry);
            }
        }
        // We should have at least 1 merge operation for key1
        assert!(
            !key1_entries.is_empty(),
            "should have merge operations for key1"
        );
        // All entries for key1 should be merge operations
        assert!(key1_entries
            .iter()
            .all(|e| matches!(e.value, crate::types::ValueDeletable::Merge(_))));
        // If there are 2 entries, they should have different expire times
        if key1_entries.len() == 2 {
            assert_ne!(
                key1_entries[0].expire_ts, key1_entries[1].expire_ts,
                "separate merge operations should have different expire times"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_merge_operations_with_same_expire_at() {
        use crate::test_utils::OnDemandCompactionSchedulerSupplier;

        // given:
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let compaction_scheduler = Arc::new(OnDemandCompactionSchedulerSupplier::new(Arc::new(
            |state| state.manifest().core().tree.l0.len() >= 2,
        )));
        let options = db_options(None);

        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .with_compactor_builder(compactor_builder_with_scheduler(
                os.clone(),
                compaction_scheduler.clone(),
                system_clock.clone(),
            ))
            .with_merge_operator(Arc::new(StringConcatMergeOperator))
            .build()
            .await
            .unwrap();

        let (manifest_store, _compactions_store, table_store) = build_test_stores(os.clone());

        let flush_opts = FlushOptions {
            flush_type: FlushType::MemTable,
        };

        // write merge operations with the SAME ExpireAtMillis timestamp at different clock times
        system_clock.set(100);
        db.merge_with_options(
            b"key1",
            b"a",
            &MergeOptions {
                ttl: Ttl::ExpireAtMillis(1000),
            },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush_with_options(flush_opts.clone()).await.unwrap();

        system_clock.set(200);
        db.merge_with_options(
            b"key1",
            b"b",
            &MergeOptions {
                ttl: Ttl::ExpireAtMillis(1000),
            },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush_with_options(flush_opts.clone()).await.unwrap();

        // when:
        let _ = await_compaction(&db, os.clone(), Some(system_clock)).await;

        // then: verify in the compacted SST that all merge operations were combined
        let stored_manifest =
            StoredManifest::load(manifest_store.clone(), Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        let db_state = stored_manifest.db_state();
        assert!(
            !db_state.tree.compacted.is_empty(),
            "compaction should have occurred"
        );

        let compacted = db_state.tree.compacted.first().unwrap().sst_views();
        assert_eq!(compacted.len(), 1);
        let handle = compacted.first().unwrap();

        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            handle,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        // collect key1 entries from the compacted SST
        let mut key1_entries = vec![];
        while let Some(entry) = iter.next().await.unwrap() {
            if entry.key.as_ref() == b"key1" {
                key1_entries.push(entry);
            }
        }

        // there should be exactly one merged entry for key1
        assert_eq!(
            key1_entries.len(),
            1,
            "expected a single merged entry for key1, got {}",
            key1_entries.len()
        );
        let merged = &key1_entries[0];
        assert_eq!(merged.expire_ts, Some(1000));
        assert!(
            matches!(&merged.value, crate::types::ValueDeletable::Merge(v) if v.as_ref() == b"ab"),
            "expected merged value 'ab', got {:?}",
            merged.value
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_compact_expired_expire_at_entries() {
        // given:
        let os = Arc::new(InMemory::new());
        let insert_clock = Arc::new(MockSystemClock::new());

        let scheduler_options = SizeTieredCompactionSchedulerOptions {
            min_compaction_sources: 2,
            max_compaction_sources: 2,
            include_size_threshold: 4.0,
            sorted_run_consolidation_threshold: 0,
        }
        .into();
        let mut options = db_options(Some(compactor_options()));
        options
            .compactor_options
            .as_mut()
            .expect("compactor options missing")
            .scheduler_options = scheduler_options;
        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(insert_clock.clone())
            .build()
            .await
            .unwrap();

        let (_, _, table_store) = build_test_stores(os.clone());

        let value = &[b'a'; 32];
        let flush_opts = FlushOptions {
            flush_type: FlushType::MemTable,
        };

        // ticker time = 0, expire at 10
        insert_clock.set(0);
        db.put_with_options(
            &[1; 16],
            value,
            &PutOptions {
                ttl: Ttl::ExpireAtMillis(10),
            },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        // ticker time = 5, expire at far future (well beyond compaction time)
        insert_clock.set(5);
        db.put_with_options(
            &[2; 16],
            value,
            &PutOptions {
                ttl: Ttl::ExpireAtMillis(i64::MAX),
            },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        db.flush_with_options(flush_opts.clone()).await.unwrap();

        // ticker time = 10, no expire time
        insert_clock.set(10);
        db.put_with_options(
            &[3; 16],
            value,
            &PutOptions { ttl: Ttl::NoExpiry },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        db.flush_with_options(flush_opts.clone()).await.unwrap();

        // when: await_compaction advances clock by 60s per iteration,
        // so compaction_start_ts will be well past expire_at=10
        let db_state =
            await_compaction_matching(&db, os.clone(), Some(insert_clock), has_single_output_sst)
                .await;

        // then: key 1 should be expired (expire_at=10 < compaction_time),
        //       key 2 should survive (expire_at=i64::MAX), key 3 has no expiry
        let db_state = db_state.expect("db was not compacted");
        assert!(db_state.tree.last_compacted_l0_sst_view_id.is_some());
        assert_eq!(db_state.tree.compacted.len(), 1);
        let compacted = db_state.tree.compacted.first().unwrap().sst_views();
        assert_eq!(compacted.len(), 1);
        let handle = compacted.first().unwrap();
        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            handle,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_value(&[2; 16], value, 2)
                    .with_create_ts(5)
                    .with_expire_ts(i64::MAX),
                RowEntry::new_value(&[3; 16], value, 3).with_create_ts(10),
            ],
        )
        .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_should_compact_expired_entries() {
        // given:
        let os = Arc::new(InMemory::new());
        let insert_clock = Arc::new(MockSystemClock::new());

        let scheduler_options = SizeTieredCompactionSchedulerOptions {
            min_compaction_sources: 2,
            max_compaction_sources: 2,
            include_size_threshold: 4.0,
            sorted_run_consolidation_threshold: 0,
        }
        .into();
        let mut options = db_options(Some(compactor_options()));
        options.default_ttl_millis = Some(50);
        options
            .compactor_options
            .as_mut()
            .expect("compactor options missing")
            .scheduler_options = scheduler_options;
        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(insert_clock.clone())
            .build()
            .await
            .unwrap();

        let (_, _, table_store) = build_test_stores(os.clone());

        let value = &[b'a'; 64];

        // ticker time = 0, expire time = 10
        insert_clock.set(0);
        db.put_with_options(
            &[1; 16],
            value,
            &PutOptions {
                ttl: Ttl::ExpireAfterMillis(10),
            },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        // ticker time = 10, expire time = 60 (using default TTL)
        insert_clock.set(10);
        db.put_with_options(
            &[2; 16],
            value,
            &PutOptions { ttl: Ttl::Default },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        db.flush().await.unwrap();

        // ticker time = 30, no expire time
        insert_clock.set(30);
        db.put_with_options(
            &[3; 16],
            value,
            &PutOptions { ttl: Ttl::NoExpiry },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        // this revives key 1
        // ticker time = 70, expire time 80
        insert_clock.set(70);
        db.put_with_options(
            &[1; 16],
            value,
            &PutOptions {
                ttl: Ttl::ExpireAfterMillis(80),
            },
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();

        db.flush().await.unwrap();

        // when:
        let db_state =
            await_compaction_matching(&db, os.clone(), Some(insert_clock), has_single_output_sst)
                .await;

        // then:
        let db_state = db_state.expect("db was not compacted");
        assert!(db_state.tree.last_compacted_l0_sst_view_id.is_some());
        assert_eq!(db_state.tree.compacted.len(), 1);
        assert_eq!(db_state.last_l0_clock_tick, 70);
        let compacted = db_state.tree.compacted.first().unwrap().sst_views();
        assert_eq!(compacted.len(), 1);
        let handle = compacted.first().unwrap();
        let mut iter = SstIterator::new_borrowed_initialized(
            ..,
            handle,
            table_store.clone(),
            SstIteratorOptions::default(),
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");

        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_value(&[1; 16], value, 4)
                    .with_create_ts(70)
                    .with_expire_ts(150),
                // no tombstone for &[2; 16] because this is the last layer of the tree,
                RowEntry::new_value(&[3; 16], value, 3).with_create_ts(30),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_should_track_total_bytes_and_throughput() {
        use crate::compactor::stats::{
            TOTAL_BYTES_BEING_COMPACTED, TOTAL_THROUGHPUT_BYTES_PER_SEC,
        };
        use chrono::DateTime;

        let mut fixture = CompactorEventHandlerTestFixture::new().await;

        let current_time = fixture.handler.system_clock.now();
        let current_time_ms = current_time.timestamp_millis() as u64;
        let start_time_1 =
            DateTime::from_timestamp_millis((current_time_ms - 2000) as i64).unwrap();
        let start_time_2 =
            DateTime::from_timestamp_millis((current_time_ms - 1000) as i64).unwrap();

        let mut compaction_1 = Compaction::new(
            Ulid::from_parts(start_time_1.timestamp_millis() as u64, 0),
            CompactionSpec::new(vec![], 10),
        );
        compaction_1.set_bytes_processed(500);

        let mut compaction_2 = Compaction::new(
            Ulid::from_parts(start_time_2.timestamp_millis() as u64, 0),
            CompactionSpec::new(vec![], 11),
        );
        compaction_2.set_bytes_processed(1000);

        fixture
            .handler
            .state_mut()
            .add_compaction(compaction_1)
            .expect("failed to add compaction 1");
        fixture
            .handler
            .state_mut()
            .add_compaction(compaction_2)
            .expect("failed to add compaction 2");

        fixture.handler.handle_log_ticker();

        let total_bytes = slatedb_common::metrics::lookup_metric(
            &fixture.test_recorder,
            TOTAL_BYTES_BEING_COMPACTED,
        )
        .expect("metric not found");
        assert_eq!(total_bytes, 0);

        let throughput = slatedb_common::metrics::lookup_metric(
            &fixture.test_recorder,
            TOTAL_THROUGHPUT_BYTES_PER_SEC,
        )
        .expect("metric not found");
        assert!(
            throughput > 0,
            "Expected throughput > 0, got {}",
            throughput
        );
    }

    #[test]
    fn test_calculate_estimated_source_bytes_uses_target_segment_tree() {
        let segment_l0 = SsTableView::identity(SsTableHandle::new(
            SsTableId::from(Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo {
                first_entry: Some(Bytes::from_static(b"seg/a")),
                index_offset: 11,
                index_len: 13,
                ..SsTableInfo::default()
            },
        ));
        let segment_sr_view = SsTableView::identity(SsTableHandle::new(
            SsTableId::from(Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo {
                first_entry: Some(Bytes::from_static(b"seg/m")),
                index_offset: 17,
                index_len: 19,
                ..SsTableInfo::default()
            },
        ));
        let segment_sr = SortedRun::new(7, [segment_sr_view.clone()]);

        let mut core = ManifestCore::new();
        core.segments = vec![Segment {
            prefix: Bytes::from_static(b"seg"),
            tree: Arc::new(LsmTreeState {
                l0: VecDeque::from(vec![segment_l0.clone()]),
                compacted: vec![segment_sr.clone()],
                ..LsmTreeState::default()
            }),
        }];

        let compaction = Compaction::new(
            Ulid::new(),
            CompactionSpec::for_segment(
                Bytes::from_static(b"seg"),
                vec![
                    SourceId::SstView(segment_l0.id),
                    SourceId::SortedRun(segment_sr.id),
                ],
                9,
            ),
        );

        let expected = segment_l0.estimate_visible_size() + segment_sr.estimate_visible_size();
        let actual = CompactorEventHandler::calculate_estimated_source_bytes(&compaction, &core);
        assert_eq!(actual, Some(expected));
    }

    #[test]
    fn test_calculate_estimated_source_bytes_uses_visible_size_for_borrowed_views() {
        let raw_size = 1_000_000u64;
        let l0_view = SsTableView::identity(SsTableHandle::new(
            SsTableId::new(Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo {
                first_entry: Some(Bytes::from_static(b"seg/a")),
                last_entry: Some(Bytes::from_static(b"seg/z")),
                index_offset: raw_size,
                ..SsTableInfo::default()
            },
        ))
        .with_visible_range(BytesRange::new(
            Included(Bytes::from_static(b"seg/a")),
            Excluded(Bytes::from_static(b"seg/m")),
        ));

        let mut core = ManifestCore::new();
        core.segments = vec![Segment {
            prefix: Bytes::from_static(b"seg"),
            tree: Arc::new(LsmTreeState {
                l0: VecDeque::from(vec![l0_view.clone()]),
                ..LsmTreeState::default()
            }),
        }];

        let compaction = Compaction::new(
            Ulid::new(),
            CompactionSpec::for_segment(
                Bytes::from_static(b"seg"),
                vec![SourceId::SstView(l0_view.id)],
                9,
            ),
        );

        let expected = l0_view.estimate_visible_size();
        let actual = CompactorEventHandler::calculate_estimated_source_bytes(&compaction, &core);
        assert_eq!(actual, Some(expected));
        assert!(expected < raw_size);
    }

    #[test]
    fn test_calculate_estimated_source_bytes_returns_none_for_missing_source() {
        let missing_l0 = Ulid::new();
        let compaction = Compaction::new(
            Ulid::new(),
            CompactionSpec::new(vec![SourceId::SstView(missing_l0)], 1),
        );

        let actual = CompactorEventHandler::calculate_estimated_source_bytes(
            &compaction,
            &ManifestCore::new(),
        );

        assert_eq!(actual, None);
    }

    #[tokio::test]
    async fn test_should_track_per_job_throughput() {
        let start_time_ms = 1000u64;
        let current_time_ms = 3000u64;
        let processed_bytes = 1000u64;

        let mut compaction = Compaction::new(
            Ulid::from_parts(start_time_ms, 0),
            CompactionSpec::new(vec![], 10),
        );
        compaction.set_bytes_processed(processed_bytes);

        // Calculate throughput manually using ULID timestamp
        let elapsed_secs = (current_time_ms as f64 - start_time_ms as f64) / 1000.0;
        let throughput = processed_bytes as f64 / elapsed_secs;
        assert_eq!(throughput, 500.0);

        // At start time, throughput should be 0
        let elapsed_zero = (start_time_ms as f64 - start_time_ms as f64) / 1000.0;
        let throughput_zero = if elapsed_zero > 0.0 {
            processed_bytes as f64 / elapsed_zero
        } else {
            0.0
        };
        assert_eq!(throughput_zero, 0.0);
    }

    #[tokio::test]
    async fn test_should_track_running_compactions_count() {
        use crate::compactor::stats::RUNNING_COMPACTIONS;

        let mut fixture = CompactorEventHandlerTestFixture::new().await;

        let running =
            slatedb_common::metrics::lookup_metric(&fixture.test_recorder, RUNNING_COMPACTIONS)
                .expect("metric not found");
        assert_eq!(running, 0);

        let compaction = Compaction::new(Ulid::new(), CompactionSpec::new(vec![], 10));
        fixture
            .handler
            .state_mut()
            .add_compaction(compaction)
            .expect("failed to add compaction");

        assert_eq!(fixture.handler.state().active_compactions().count(), 1);
    }

    #[tokio::test]
    async fn test_should_record_compactor_epoch() {
        let fixture = CompactorEventHandlerTestFixture::new().await;

        let compactor_epoch =
            slatedb_common::metrics::lookup_metric(&fixture.test_recorder, COMPACTOR_EPOCH)
                .expect("metric not found");

        assert_eq!(
            compactor_epoch,
            fixture.handler.state().manifest().value.compactor_epoch as i64
        );
    }

    #[tokio::test]
    async fn test_submit_persists_compaction() {
        let os = Arc::new(InMemory::new());
        let (manifest_store, compactions_store, _table_store) = build_test_stores(os.clone());
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();
        let stored_manifest = StoredManifest::load(manifest_store.clone(), system_clock.clone())
            .await
            .unwrap();
        StoredCompactions::create(
            compactions_store.clone(),
            stored_manifest.manifest().compactor_epoch,
        )
        .await
        .unwrap();

        let spec = CompactionSpec::new(vec![SourceId::SortedRun(0)], 0);
        let compaction_id = Compactor::submit(
            spec.clone(),
            compactions_store.clone(),
            Arc::new(DbRand::default()),
            system_clock.clone(),
        )
        .await
        .unwrap();

        let compactions = compactions_store.read_latest_compactions().await.unwrap();
        let stored = compactions
            .compactions
            .get(&compaction_id)
            .expect("missing submitted compaction");

        assert_eq!(stored.spec(), &spec);
        assert_eq!(stored.status(), CompactionStatus::Submitted);
    }

    #[tokio::test]
    async fn test_submit_retries_on_boundary_conflict() {
        // Set up a database with an initial manifest and compactions record.
        let raw_os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let gated_os = Arc::new(GatedObjectStore::new(Arc::clone(&raw_os)));
        let (manifest_store, compactions_store, _table_store) = build_test_stores(gated_os.clone());
        let external_compactions_store = Arc::new(CompactionsStore::new(
            &Path::from(PATH),
            Arc::clone(&raw_os),
        ));
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();
        let stored_manifest = StoredManifest::load(manifest_store.clone(), system_clock.clone())
            .await
            .unwrap();
        StoredCompactions::create(
            compactions_store.clone(),
            stored_manifest.manifest().compactor_epoch,
        )
        .await
        .unwrap();

        // Pause submit before its first write. While it is stale, an external writer creates
        // a live newer version, then GC deletes and fences the stale writer's next id.
        let start_id = compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .id;
        let spec = CompactionSpec::new(vec![SourceId::SortedRun(0)], 0);
        let arrivals = gated_os.put_opts_gate.arrivals();
        gated_os.put_opts_gate.close();
        let submit_task = tokio::spawn({
            let spec = spec.clone();
            let compactions_store = compactions_store.clone();
            let system_clock = system_clock.clone();
            async move {
                Compactor::submit(
                    spec,
                    compactions_store,
                    Arc::new(DbRand::default()),
                    system_clock,
                )
                .await
            }
        });

        gated_os.put_opts_gate.wait_for_arrivals(arrivals + 1).await;

        // The external writer wins the stale writer's intended id.
        let mut external = StoredCompactions::load(external_compactions_store.clone())
            .await
            .unwrap();
        external
            .update(external.prepare_dirty().unwrap())
            .await
            .unwrap();
        assert_eq!(external.id(), start_id + 1);

        // A second external write leaves a live latest version above the future boundary,
        // so the stale writer can make progress after refreshing.
        external
            .update(external.prepare_dirty().unwrap())
            .await
            .unwrap();
        assert_eq!(external.id(), start_id + 2);

        // GC fences and removes the stale writer's next id, but preserves the live latest
        // version at start_id + 2.
        external_compactions_store
            .advance_boundary(start_id + 1)
            .await
            .unwrap();
        external_compactions_store
            .delete_compactions(start_id + 1)
            .await
            .unwrap();
        let latest = external_compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .id;
        assert_eq!(latest, start_id + 2);

        gated_os.put_opts_gate.release();

        // Submit should refresh to the live newer version and retry at the next safe id.
        let compaction_id = submit_task.await.unwrap().unwrap();

        let compactions = compactions_store.read_latest_compactions().await.unwrap();
        let stored = compactions
            .compactions
            .get(&compaction_id)
            .expect("missing submitted compaction");

        assert_eq!(compactions.id, start_id + 3);
        assert_eq!(stored.spec(), &spec);
        assert_eq!(stored.status(), CompactionStatus::Submitted);
    }

    #[tokio::test]
    async fn test_submit_full_compaction_uses_sorted_run_sources_only() {
        let os = Arc::new(InMemory::new());
        let (manifest_store, compactions_store, _table_store) = build_test_stores(os.clone());
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        StoredManifest::create_new_db(
            manifest_store.clone(),
            ManifestCore::new(),
            system_clock.clone(),
        )
        .await
        .unwrap();
        let mut stored_manifest =
            StoredManifest::load(manifest_store.clone(), system_clock.clone())
                .await
                .unwrap();

        let mut dirty = stored_manifest.prepare_dirty().unwrap();
        let l0_info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"a")),
            ..SsTableInfo::default()
        };
        let sr_info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"m")),
            ..SsTableInfo::default()
        };
        let l0_view_newest: SsTableView = SsTableView::identity(SsTableHandle::new(
            SsTableId::from(Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            l0_info.clone(),
        ));
        let l0_view_oldest: SsTableView = SsTableView::identity(SsTableHandle::new(
            SsTableId::from(Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            l0_info.clone(),
        ));
        Arc::make_mut(&mut dirty.value.core.tree).l0 =
            VecDeque::from(vec![l0_view_newest, l0_view_oldest]);
        Arc::make_mut(&mut dirty.value.core.tree).compacted = vec![
            SortedRun::new(
                2,
                [SsTableView::identity(SsTableHandle::new(
                    SsTableId::from(Ulid::new()),
                    SST_FORMAT_VERSION_LATEST,
                    sr_info.clone(),
                ))],
            ),
            SortedRun::new(
                1,
                [SsTableView::identity(SsTableHandle::new(
                    SsTableId::from(Ulid::new()),
                    SST_FORMAT_VERSION_LATEST,
                    sr_info.clone(),
                ))],
            ),
        ];
        stored_manifest.update(dirty).await.unwrap();

        StoredCompactions::create(
            compactions_store.clone(),
            stored_manifest.manifest().compactor_epoch,
        )
        .await
        .unwrap();

        let scheduler = MockScheduler::new();
        let specs = scheduler
            .generate(
                &CompactorStateView {
                    compactions: None,
                    manifest: VersionedManifest::from_manifest(
                        0,
                        stored_manifest.manifest().clone(),
                    ),
                },
                &CompactionRequest::FullSegment {
                    segment: Bytes::new(),
                },
            )
            .unwrap();
        assert_eq!(specs.len(), 1);
        let compaction_id = Compactor::submit(
            specs[0].clone(),
            compactions_store.clone(),
            Arc::new(DbRand::default()),
            system_clock.clone(),
        )
        .await
        .unwrap();

        let compactions = compactions_store.read_latest_compactions().await.unwrap();
        let stored = compactions
            .compactions
            .get(&compaction_id)
            .expect("missing submitted compaction");
        let expected_sources = vec![SourceId::SortedRun(2), SourceId::SortedRun(1)];

        assert_eq!(stored.spec().sources(), &expected_sources);
        assert_eq!(stored.spec().destination(), Some(1));
        assert_eq!(stored.status(), CompactionStatus::Submitted);
    }

    #[test]
    fn test_plan_spec_returns_spec_clone() {
        let scheduler = MockScheduler::new();
        let state = CompactorStateView {
            compactions: None,
            manifest: VersionedManifest::from_manifest(0, Manifest::initial(ManifestCore::new())),
        };
        let spec = CompactionSpec::new(vec![SourceId::SortedRun(7)], 7);

        let planned = scheduler
            .generate(&state, &CompactionRequest::Spec(spec.clone()))
            .unwrap();

        assert_eq!(planned, vec![spec]);
    }

    #[test]
    fn test_plan_full_segment_root_uses_sorted_runs_and_min_destination() {
        let scheduler = MockScheduler::new();
        let mut core = ManifestCore::new();
        let l0_info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"a")),
            ..SsTableInfo::default()
        };
        let sr_info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"m")),
            ..SsTableInfo::default()
        };
        let l0_view_first: SsTableView = SsTableView::identity(SsTableHandle::new(
            SsTableId::from(Ulid::from_parts(1, 0)),
            SST_FORMAT_VERSION_LATEST,
            l0_info.clone(),
        ));
        let l0_view_second: SsTableView = SsTableView::identity(SsTableHandle::new(
            SsTableId::from(Ulid::from_parts(2, 0)),
            SST_FORMAT_VERSION_LATEST,
            l0_info,
        ));
        Arc::make_mut(&mut core.tree).l0 = VecDeque::from(vec![l0_view_first, l0_view_second]);
        Arc::make_mut(&mut core.tree).compacted = vec![
            SortedRun::new(
                5,
                [SsTableView::identity(SsTableHandle::new(
                    SsTableId::from(Ulid::from_parts(10, 0)),
                    SST_FORMAT_VERSION_LATEST,
                    sr_info.clone(),
                ))],
            ),
            SortedRun::new(
                2,
                [SsTableView::identity(SsTableHandle::new(
                    SsTableId::from(Ulid::from_parts(11, 0)),
                    SST_FORMAT_VERSION_LATEST,
                    sr_info,
                ))],
            ),
        ];
        let state = CompactorStateView {
            compactions: None,
            manifest: VersionedManifest::from_manifest(0, Manifest::initial(core)),
        };

        let planned = scheduler
            .generate(
                &state,
                &CompactionRequest::FullSegment {
                    segment: Bytes::new(),
                },
            )
            .unwrap();

        let expected_sources = vec![SourceId::SortedRun(5), SourceId::SortedRun(2)];
        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].sources(), &expected_sources);
        assert_eq!(planned[0].destination(), Some(2));
    }

    #[test]
    fn test_plan_full_segment_root_without_sorted_runs_is_invalid() {
        let scheduler = MockScheduler::new();
        let mut core = ManifestCore::new();
        let l0_info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"a")),
            ..SsTableInfo::default()
        };
        Arc::make_mut(&mut core.tree).l0 = VecDeque::from(vec![
            SsTableView::identity(SsTableHandle::new(
                SsTableId::from(Ulid::from_parts(1, 0)),
                SST_FORMAT_VERSION_LATEST,
                l0_info.clone(),
            )),
            SsTableView::identity(SsTableHandle::new(
                SsTableId::from(Ulid::from_parts(2, 0)),
                SST_FORMAT_VERSION_LATEST,
                l0_info,
            )),
        ]);
        let state = CompactorStateView {
            compactions: None,
            manifest: VersionedManifest::from_manifest(0, Manifest::initial(core)),
        };

        let err = scheduler
            .generate(
                &state,
                &CompactionRequest::FullSegment {
                    segment: Bytes::new(),
                },
            )
            .expect_err(
                "full-segment should reject empty or L0-only inputs because L0 SSTs are excluded",
            );

        assert_eq!(err.kind(), crate::ErrorKind::Invalid);
        assert_eq!(err.to_string(), "Invalid error: invalid compaction");
    }

    #[test]
    fn test_plan_full_segment_targets_named_segment() {
        // FullSegment against a named segment must pull sources from that
        // segment's tree, not the root tree. RFC-0024 routes named-segment
        // SRs through `core.segments[..].tree.compacted`, so a request
        // against `b"key"` should ignore root-tree SRs entirely.
        let scheduler = MockScheduler::new();
        let mut core = ManifestCore::new();
        core.segment_extractor_name = Some("test".into());
        let sr_info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"key1")),
            ..SsTableInfo::default()
        };
        core.segments = vec![Segment {
            prefix: Bytes::from_static(b"key"),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::new(),
                compacted: vec![
                    SortedRun::new(
                        7,
                        [SsTableView::identity(SsTableHandle::new(
                            SsTableId::from(Ulid::from_parts(70, 0)),
                            SST_FORMAT_VERSION_LATEST,
                            sr_info.clone(),
                        ))],
                    ),
                    SortedRun::new(
                        3,
                        [SsTableView::identity(SsTableHandle::new(
                            SsTableId::from(Ulid::from_parts(30, 0)),
                            SST_FORMAT_VERSION_LATEST,
                            sr_info,
                        ))],
                    ),
                ],
            }),
        }];
        let state = CompactorStateView {
            compactions: None,
            manifest: VersionedManifest::from_manifest(0, Manifest::initial(core)),
        };

        let planned = scheduler
            .generate(
                &state,
                &CompactionRequest::FullSegment {
                    segment: Bytes::from_static(b"key"),
                },
            )
            .unwrap();

        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].segment().as_ref(), b"key");
        assert_eq!(
            planned[0].sources(),
            &vec![SourceId::SortedRun(7), SourceId::SortedRun(3)]
        );
        assert_eq!(planned[0].destination(), Some(3));
    }

    #[test]
    fn test_plan_full_segment_unknown_segment_is_invalid() {
        // Asking to compact a non-empty segment prefix on an unsegmented DB
        // (root has sorted runs, no segments configured) must be rejected.
        // A segment-blind implementation that ignored the prefix would
        // happily return root SRs here — this asserts the routing actually
        // looks the segment up.
        let scheduler = MockScheduler::new();
        let mut core = ManifestCore::new();
        let sr_info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"r")),
            ..SsTableInfo::default()
        };
        Arc::make_mut(&mut core.tree).compacted = vec![SortedRun::new(
            9,
            [SsTableView::identity(SsTableHandle::new(
                SsTableId::from(Ulid::from_parts(90, 0)),
                SST_FORMAT_VERSION_LATEST,
                sr_info,
            ))],
        )];
        let state = CompactorStateView {
            compactions: None,
            manifest: VersionedManifest::from_manifest(0, Manifest::initial(core)),
        };

        let err = scheduler
            .generate(
                &state,
                &CompactionRequest::FullSegment {
                    segment: Bytes::from_static(b"missing"),
                },
            )
            .expect_err("full-segment against unknown segment should fail");

        assert_eq!(err.kind(), crate::ErrorKind::Invalid);
    }

    #[test]
    fn test_plan_full_segment_single_sorted_run_is_allowed() {
        // A tree with exactly one SR is a legitimate target — the merge
        // pipeline still applies retention, TTL, compaction filters, and
        // merge-operator collapses. Reject only when there are no SRs at all.
        let scheduler = MockScheduler::new();
        let mut core = ManifestCore::new();
        let sr_info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"a")),
            ..SsTableInfo::default()
        };
        Arc::make_mut(&mut core.tree).compacted = vec![SortedRun::new(
            4,
            [SsTableView::identity(SsTableHandle::new(
                SsTableId::from(Ulid::from_parts(40, 0)),
                SST_FORMAT_VERSION_LATEST,
                sr_info,
            ))],
        )];
        let state = CompactorStateView {
            compactions: None,
            manifest: VersionedManifest::from_manifest(0, Manifest::initial(core)),
        };

        let planned = scheduler
            .generate(
                &state,
                &CompactionRequest::FullSegment {
                    segment: Bytes::new(),
                },
            )
            .unwrap();

        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].sources(), &vec![SourceId::SortedRun(4)]);
        assert_eq!(planned[0].destination(), Some(4));
    }

    #[test]
    fn test_plan_full_sweeps_root_and_every_named_segment() {
        // Full must produce one spec per eligible tree, ordered as
        // `trees_with_prefix()` yields them: root first, then segments by
        // ascending prefix.
        let scheduler = MockScheduler::new();
        let mut core = ManifestCore::new();
        let info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"x")),
            ..SsTableInfo::default()
        };
        Arc::make_mut(&mut core.tree).compacted = vec![
            SortedRun::new(
                8,
                [SsTableView::identity(SsTableHandle::new(
                    SsTableId::from(Ulid::from_parts(80, 0)),
                    SST_FORMAT_VERSION_LATEST,
                    info.clone(),
                ))],
            ),
            SortedRun::new(
                4,
                [SsTableView::identity(SsTableHandle::new(
                    SsTableId::from(Ulid::from_parts(40, 0)),
                    SST_FORMAT_VERSION_LATEST,
                    info.clone(),
                ))],
            ),
        ];
        core.segment_extractor_name = Some("test".into());
        core.segments = vec![
            Segment {
                prefix: Bytes::from_static(b"a/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::new(),
                    compacted: vec![SortedRun::new(
                        3,
                        [SsTableView::identity(SsTableHandle::new(
                            SsTableId::from(Ulid::from_parts(30, 0)),
                            SST_FORMAT_VERSION_LATEST,
                            info.clone(),
                        ))],
                    )],
                }),
            },
            Segment {
                prefix: Bytes::from_static(b"b/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::new(),
                    compacted: vec![
                        SortedRun::new(
                            9,
                            [SsTableView::identity(SsTableHandle::new(
                                SsTableId::from(Ulid::from_parts(90, 0)),
                                SST_FORMAT_VERSION_LATEST,
                                info.clone(),
                            ))],
                        ),
                        SortedRun::new(
                            6,
                            [SsTableView::identity(SsTableHandle::new(
                                SsTableId::from(Ulid::from_parts(60, 0)),
                                SST_FORMAT_VERSION_LATEST,
                                info,
                            ))],
                        ),
                    ],
                }),
            },
        ];
        let state = CompactorStateView {
            compactions: None,
            manifest: VersionedManifest::from_manifest(0, Manifest::initial(core)),
        };

        let planned = scheduler
            .generate(&state, &CompactionRequest::Full)
            .unwrap();

        assert_eq!(planned.len(), 3);
        assert_eq!(planned[0].segment().as_ref(), b"");
        assert_eq!(
            planned[0].sources(),
            &vec![SourceId::SortedRun(8), SourceId::SortedRun(4)]
        );
        assert_eq!(planned[0].destination(), Some(4));
        assert_eq!(planned[1].segment().as_ref(), b"a/");
        assert_eq!(planned[1].sources(), &vec![SourceId::SortedRun(3)]);
        assert_eq!(planned[1].destination(), Some(3));
        assert_eq!(planned[2].segment().as_ref(), b"b/");
        assert_eq!(
            planned[2].sources(),
            &vec![SourceId::SortedRun(9), SourceId::SortedRun(6)]
        );
        assert_eq!(planned[2].destination(), Some(6));
    }

    #[test]
    fn test_plan_full_skips_trees_without_sorted_runs() {
        // Best-effort semantics: a tree with no compacted SRs is silently
        // skipped (even if it has L0 SSTs), but eligible peers still produce
        // specs. This is the key behavioral difference from FullSegment,
        // which errors when explicitly asked to compact such a tree.
        let scheduler = MockScheduler::new();
        let mut core = ManifestCore::new();
        let info = SsTableInfo {
            first_entry: Some(Bytes::from_static(b"x")),
            ..SsTableInfo::default()
        };
        Arc::make_mut(&mut core.tree).compacted = vec![SortedRun::new(
            5,
            [SsTableView::identity(SsTableHandle::new(
                SsTableId::from(Ulid::from_parts(50, 0)),
                SST_FORMAT_VERSION_LATEST,
                info.clone(),
            ))],
        )];
        core.segment_extractor_name = Some("test".into());
        core.segments = vec![
            // L0-only: still skipped because L0 SSTs are ineligible inputs.
            Segment {
                prefix: Bytes::from_static(b"l0only/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::from(vec![SsTableView::identity(SsTableHandle::new(
                        SsTableId::from(Ulid::from_parts(1, 0)),
                        SST_FORMAT_VERSION_LATEST,
                        info,
                    ))]),
                    compacted: vec![],
                }),
            },
            // Fully empty tree: also skipped.
            Segment {
                prefix: Bytes::from_static(b"none/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::new(),
                    compacted: vec![],
                }),
            },
        ];
        let state = CompactorStateView {
            compactions: None,
            manifest: VersionedManifest::from_manifest(0, Manifest::initial(core)),
        };

        let planned = scheduler
            .generate(&state, &CompactionRequest::Full)
            .unwrap();

        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].segment().as_ref(), b"");
        assert_eq!(planned[0].sources(), &vec![SourceId::SortedRun(5)]);
    }

    #[test]
    fn test_plan_full_returns_empty_when_no_tree_is_eligible() {
        // Empty DB / no compacted SRs anywhere → Ok(vec![]), not an error.
        let scheduler = MockScheduler::new();
        let state = CompactorStateView {
            compactions: None,
            manifest: VersionedManifest::from_manifest(0, Manifest::initial(ManifestCore::new())),
        };

        let planned = scheduler
            .generate(&state, &CompactionRequest::Full)
            .unwrap();

        assert!(planned.is_empty(), "expected empty plan, got {:?}", planned);
    }

    struct CompactorEventHandlerTestFixture {
        manifest: StoredManifest,
        manifest_store: Arc<ManifestStore>,
        compactions_store: Arc<CompactionsStore>,
        options: Settings,
        db: Db,
        scheduler: Arc<MockScheduler>,
        real_executor: Arc<TokioCompactionExecutor>,
        real_executor_rx: async_channel::Receiver<WorkerMessage>,
        test_recorder: Arc<slatedb_common::metrics::DefaultMetricsRecorder>,
        handler: CompactorEventHandler,
    }

    impl CompactorEventHandlerTestFixture {
        async fn new() -> Self {
            let compactor_options = Arc::new(compactor_options());
            let worker_options = Arc::new(CompactionWorkerOptions::default());
            let options = db_options(None);

            let os = Arc::new(InMemory::new());
            let (manifest_store, compactions_store, table_store) = build_test_stores(os.clone());
            let db = Db::builder(PATH, os.clone())
                .with_settings(options.clone())
                .build()
                .await
                .unwrap();

            let scheduler = Arc::new(MockScheduler::new());
            let (real_executor_tx, real_executor_rx) = async_channel::unbounded();
            let rand = Arc::new(DbRand::default());
            let test_recorder = Arc::new(slatedb_common::metrics::DefaultMetricsRecorder::new());
            let recorder = MetricsRecorderHelper::new(
                test_recorder.clone() as Arc<dyn slatedb_common::metrics::MetricsRecorder>,
                slatedb_common::metrics::MetricLevel::default(),
            );
            let compactor_stats = Arc::new(CompactionStats::new(&recorder));
            let real_executor = Arc::new(TokioCompactionExecutor::new(
                TokioCompactionExecutorOptions {
                    handle: Handle::current(),
                    options: worker_options.clone(),
                    worker_tx: real_executor_tx,
                    table_store,
                    rand: rand.clone(),
                    stats: compactor_stats.clone(),
                    worker_stats: stats::WorkerStats::new(&recorder, "test-worker"),
                    clock: Arc::new(DefaultSystemClock::new()),
                    manifest_store: manifest_store.clone(),
                    merge_operator: None,
                    #[cfg(feature = "compaction_filters")]
                    compaction_filter_supplier: None,
                },
            ));
            let handler = CompactorEventHandler::new(
                manifest_store.clone(),
                compactions_store.clone(),
                compactor_options.clone(),
                scheduler.clone(),
                rand.clone(),
                compactor_stats.clone(),
                Arc::new(DefaultSystemClock::new()),
                MetricsRecorderHelper::noop(),
            )
            .await
            .unwrap();
            let manifest =
                StoredManifest::load(manifest_store.clone(), Arc::new(DefaultSystemClock::new()))
                    .await
                    .unwrap();
            Self {
                manifest,
                manifest_store,
                compactions_store,
                options,
                db,
                scheduler,
                real_executor_rx,
                real_executor,
                test_recorder,
                handler,
            }
        }

        /// Like `new()` but accepts an explicit `system_clock` so tests can
        /// control time (e.g. to exercise heartbeat-timeout reclamation).
        async fn new_with_clock(
            system_clock: Arc<dyn SystemClock>,
            compactor_options: Arc<CompactorOptions>,
        ) -> Self {
            let options = db_options(None);

            let os = Arc::new(InMemory::new());
            let (manifest_store, compactions_store, table_store) = build_test_stores(os.clone());
            let db = Db::builder(PATH, os.clone())
                .with_settings(options.clone())
                .build()
                .await
                .unwrap();

            let scheduler = Arc::new(MockScheduler::new());
            let (real_executor_tx, real_executor_rx) = async_channel::unbounded();
            let rand = Arc::new(DbRand::default());
            let test_recorder = Arc::new(slatedb_common::metrics::DefaultMetricsRecorder::new());
            let recorder = MetricsRecorderHelper::new(
                test_recorder.clone() as Arc<dyn slatedb_common::metrics::MetricsRecorder>,
                slatedb_common::metrics::MetricLevel::default(),
            );
            let compactor_stats = Arc::new(CompactionStats::new(&recorder));
            let worker_options = Arc::new(CompactionWorkerOptions::default());
            let real_executor = Arc::new(TokioCompactionExecutor::new(
                TokioCompactionExecutorOptions {
                    handle: Handle::current(),
                    options: worker_options,
                    worker_tx: real_executor_tx,
                    table_store,
                    rand: rand.clone(),
                    stats: compactor_stats.clone(),
                    worker_stats: stats::WorkerStats::noop(),
                    clock: system_clock.clone(),
                    manifest_store: manifest_store.clone(),
                    merge_operator: None,
                    #[cfg(feature = "compaction_filters")]
                    compaction_filter_supplier: None,
                },
            ));
            let handler = CompactorEventHandler::new(
                manifest_store.clone(),
                compactions_store.clone(),
                compactor_options.clone(),
                scheduler.clone(),
                rand.clone(),
                compactor_stats.clone(),
                system_clock.clone(),
                recorder.clone(),
            )
            .await
            .unwrap();
            let manifest = StoredManifest::load(manifest_store.clone(), system_clock)
                .await
                .unwrap();
            Self {
                manifest,
                manifest_store,
                compactions_store,
                options,
                db,
                scheduler,
                real_executor_rx,
                real_executor,
                test_recorder,
                handler,
            }
        }

        async fn latest_db_state(&mut self) -> ManifestCore {
            self.manifest.refresh().await.unwrap().core.clone()
        }

        async fn write_l0(&mut self) {
            let mut rng = rng::new_test_rng(None);
            let manifest = self.manifest.refresh().await.unwrap();
            let l0s = manifest.core.tree.l0.len();
            // TODO: add an explicit flush_memtable fn to db and use that instead
            let mut k = vec![0u8; self.options.l0_sst_size_bytes];
            rng.fill_bytes(&mut k);
            self.db.put(&k, &[b'x'; 10]).await.unwrap();
            self.db.flush().await.unwrap();
            loop {
                let manifest = self.manifest.refresh().await.unwrap().clone();
                if manifest.core.tree.l0.len() > l0s {
                    break;
                }
            }
        }

        async fn build_l0_compaction(&mut self) -> CompactionSpec {
            let db_state = self.latest_db_state().await;
            let l0_ids_to_compact: Vec<SourceId> = db_state
                .tree
                .l0
                .iter()
                .map(|h| SourceId::SstView(h.id))
                .collect();
            CompactionSpec::new(l0_ids_to_compact, 0)
        }

        /// Returns all `Scheduled` compactions currently in `.compactions`.
        async fn get_scheduled_compactions(&self) -> Vec<Compaction> {
            self.compactions_store
                .read_latest_compactions()
                .await
                .unwrap()
                .compactions
                .iter()
                .filter(|c| c.status() == CompactionStatus::Scheduled)
                .cloned()
                .collect()
        }

        /// Simulates a worker claiming each `Scheduled` compaction, executing it
        /// with the real `TokioCompactionExecutor`, and writing `Compacted` + output
        /// SSTs back to `.compactions`. Call `handle_ticker()` afterward so the
        /// coordinator commits the results via `commit_compacted_entries`.
        async fn simulate_worker_completes(&self) {
            use crate::compactor_executor::StartCompactionJobArgs;

            let manifest = self.manifest_store.read_latest_manifest().await.unwrap();
            let db_state = manifest.core();

            let scheduled = self.get_scheduled_compactions().await;
            for compaction in scheduled {
                let destination = compaction.spec().destination().expect("tiered spec");
                let l0_sst_views = compaction.get_l0_sst_views(db_state);
                let sorted_runs = compaction.get_sorted_runs(db_state);
                let is_dest_last_run = match db_state.tree_for_segment(compaction.spec().segment())
                {
                    Some(tree) => {
                        tree.compacted.is_empty()
                            || tree.compacted.last().is_some_and(|sr| destination == sr.id)
                    }
                    None => false,
                };
                let args = StartCompactionJobArgs {
                    id: compaction.id(),
                    compaction_id: compaction.id(),
                    segment: compaction.spec().segment().clone(),
                    destination,
                    l0_sst_views,
                    sorted_runs,
                    compaction_clock_tick: db_state.last_l0_clock_tick,
                    is_dest_last_run,
                    complete_input_through: None,
                    retention_min_seq: None,
                    ctx: compaction.ctx().cloned(),
                };

                self.real_executor.start_compaction_job(args);

                let result = tokio::time::timeout(Duration::from_millis(500), async {
                    loop {
                        match self.real_executor_rx.recv().await.expect("channel closed") {
                            WorkerMessage::CompactionJobFinished { result, .. } => {
                                return result.unwrap()
                            }
                            _ => continue,
                        }
                    }
                })
                .await
                .expect("timeout waiting for compaction result");

                let mut stored = StoredCompactions::try_load(self.compactions_store.clone())
                    .await
                    .unwrap()
                    .unwrap();
                loop {
                    stored.refresh().await.unwrap();
                    let mut dirty = stored.prepare_dirty().unwrap();
                    let completed = compaction
                        .clone()
                        .with_status(CompactionStatus::Compacted)
                        .with_output_ssts(
                            result.sst_views().iter().map(|v| v.sst.clone()).collect(),
                        )
                        .with_ctx(None);
                    dirty.value.insert(completed);
                    match stored.update(dirty).await {
                        Ok(()) => break,
                        Err(e) if e.is_sequenced_write_conflict() => continue,
                        Err(e) => panic!("write_compacted failed: {e}"),
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_should_record_last_compaction_ts() {
        // given:
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.write_l0().await;
        let compaction = fixture.build_l0_compaction().await;
        fixture.scheduler.inject_compaction(compaction.clone());
        fixture.handler.handle_ticker().await.unwrap();
        fixture.simulate_worker_completes().await;

        let starting_last_ts =
            slatedb_common::metrics::lookup_metric(&fixture.test_recorder, LAST_COMPACTION_TS_SEC)
                .expect("metric not found");

        // when: coordinator commits the Compacted entry
        fixture.handler.handle_ticker().await.unwrap();

        // then:
        let last_ts =
            slatedb_common::metrics::lookup_metric(&fixture.test_recorder, LAST_COMPACTION_TS_SEC)
                .expect("metric not found");
        assert!(last_ts > starting_last_ts);
    }

    #[tokio::test]
    async fn test_should_write_manifest_safely() {
        // given:
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.write_l0().await;
        let compaction = fixture.build_l0_compaction().await;
        fixture.scheduler.inject_compaction(compaction.clone());
        fixture.handler.handle_ticker().await.unwrap();
        fixture.simulate_worker_completes().await;
        // write an l0 before the coordinator commits
        fixture.write_l0().await;

        // when: coordinator commits the Compacted entry
        fixture.handler.handle_ticker().await.unwrap();

        // then:
        let db_state = fixture.latest_db_state().await;
        assert_eq!(db_state.tree.l0.len(), 1);
        assert_eq!(db_state.tree.compacted.len(), 1);
        let l0_id = db_state.tree.l0.front().unwrap().sst.id.value();
        let compacted_l0s: Vec<Ulid> = db_state
            .tree
            .compacted
            .first()
            .unwrap()
            .sst_views()
            .iter()
            .map(|view| view.sst.id.value())
            .collect();
        assert!(!compacted_l0s.contains(&l0_id));
        assert_eq!(
            db_state.tree.last_compacted_l0_sst_view_id.unwrap(),
            compaction
                .sources()
                .first()
                .and_then(|id| id.maybe_unwrap_sst_view())
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_should_persist_compactions_on_start_and_finish() {
        // given:
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.write_l0().await;
        let compaction = fixture.build_l0_compaction().await;
        fixture.scheduler.inject_compaction(compaction.clone());

        // when: schedule compaction → Scheduled persisted
        fixture.handler.handle_ticker().await.unwrap();

        let stored_compactions = fixture
            .compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions;
        assert_eq!(
            stored_compactions
                .iter()
                .collect::<Vec<&Compaction>>()
                .len(),
            1
        );
        let scheduled_id = stored_compactions
            .iter()
            .next()
            .expect("compaction should be persisted")
            .id();
        let state_id = fixture
            .handler
            .state()
            .active_compactions()
            .next()
            .expect("state missing compaction")
            .id();
        assert_eq!(scheduled_id, state_id);

        // worker executes, writes Compacted; coordinator commits on next tick
        fixture.simulate_worker_completes().await;
        fixture.handler.handle_ticker().await.unwrap();

        // then: finished compaction is retained (one entry for GC)
        let stored_compactions = fixture
            .compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions;
        let mut iter = stored_compactions.iter();
        assert_eq!(
            iter.next()
                .expect("compactions should not be empty after finish")
                .id(),
            scheduled_id,
        );
        assert!(
            iter.next().is_none(),
            "expected only one retained finished compaction for GC"
        );
    }

    #[tokio::test]
    async fn test_maybe_schedule_compactions_only_submits() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.write_l0().await;
        // Refresh so add_compaction sees the just-flushed L0; the source-
        // isolation check needs the spec's source view to be present in the
        // compactor's local tree.
        fixture.handler.state_writer.refresh().await.unwrap();
        let compaction = fixture.build_l0_compaction().await;
        fixture.scheduler.inject_compaction(compaction);

        fixture.handler.maybe_schedule_compactions().await.unwrap();

        let mut compactions = fixture.handler.state().active_compactions();
        let scheduled = compactions.next().expect("missing compaction");
        assert_eq!(scheduled.status(), CompactionStatus::Submitted);
        assert!(compactions.next().is_none());

        let stored_compactions = fixture
            .compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions;
        let stored = stored_compactions
            .iter()
            .next()
            .expect("compaction should be persisted");
        assert_eq!(stored.status(), CompactionStatus::Submitted);
    }

    #[tokio::test]
    async fn test_maybe_validate_submitted_compactions_promotes_to_scheduled() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.write_l0().await;
        fixture.handler.state_writer.refresh().await.unwrap();
        let spec = fixture.build_l0_compaction().await;
        let compaction_id = Ulid::new();
        fixture
            .handler
            .state_mut()
            .add_compaction(Compaction::new(compaction_id, spec))
            .expect("failed to add compaction");

        fixture
            .handler
            .maybe_validate_submitted_compactions()
            .await
            .unwrap();

        // Scheduled is "ready to claim", not "running" — no local execution happens.
        let compactions = &fixture.handler.state_writer.state.compactions().value;
        assert_eq!(
            compactions
                .get(&compaction_id)
                .expect("missing compaction")
                .status(),
            CompactionStatus::Scheduled
        );
        // Scheduled is persisted so remote workers (and a restarted coordinator)
        // observe the same view.
        let stored_compactions = fixture
            .compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions;
        assert_eq!(
            stored_compactions
                .get(&compaction_id)
                .expect("missing stored compaction")
                .status(),
            CompactionStatus::Scheduled
        );
    }

    #[tokio::test]
    async fn test_maybe_validate_submitted_compactions_completes_trivial_move() {
        let options = Arc::new(CompactorOptions {
            enable_trivial_move: true,
            ..compactor_options()
        });
        let mut fixture = CompactorEventHandlerTestFixture::new_with_clock(
            Arc::new(DefaultSystemClock::new()),
            options,
        )
        .await;
        let l0 = bounded_sst_view(2, b"m", b"n");
        let sr_first = bounded_sst_view(1, b"a", b"b");
        let sr_last = bounded_sst_view(3, b"z", b"z");
        let core = &mut fixture
            .handler
            .state_writer
            .state
            .manifest_mut_for_test()
            .value
            .core;
        Arc::make_mut(&mut core.tree).l0 = VecDeque::from([l0.clone()]);
        Arc::make_mut(&mut core.tree).compacted =
            vec![SortedRun::new(1, [sr_first.clone(), sr_last.clone()])];

        let compaction_id = Ulid::new();
        fixture
            .handler
            .state_mut()
            .add_compaction(Compaction::new(
                compaction_id,
                CompactionSpec::new(vec![SourceId::SstView(l0.id), SourceId::SortedRun(1)], 2),
            ))
            .expect("failed to add compaction");

        fixture
            .handler
            .maybe_validate_submitted_compactions()
            .await
            .unwrap();

        let state = fixture.handler.state();
        assert_eq!(
            state
                .compactions()
                .value
                .get(&compaction_id)
                .expect("missing compaction")
                .status(),
            CompactionStatus::Completed
        );
        assert!(state.db_state().tree.l0.is_empty());
        assert_eq!(state.db_state().tree.compacted.len(), 1);
        let output = &state.db_state().tree.compacted[0];
        assert_eq!(output.id, 2);
        assert_eq!(
            output
                .sst_views()
                .iter()
                .map(|view| view.id)
                .collect::<Vec<_>>(),
            vec![sr_first.id, l0.id, sr_last.id]
        );

        let expected_output = output.clone();
        let stored_manifest = fixture.latest_db_state().await;
        assert!(stored_manifest.tree.l0.is_empty());
        assert_eq!(stored_manifest.tree.compacted[0], expected_output);
    }

    #[tokio::test]
    async fn test_maybe_validate_submitted_compactions_schedules_when_trivial_move_disabled() {
        let options = Arc::new(CompactorOptions {
            enable_trivial_move: false,
            ..compactor_options()
        });
        let mut fixture = CompactorEventHandlerTestFixture::new_with_clock(
            Arc::new(DefaultSystemClock::new()),
            options,
        )
        .await;
        let l0 = bounded_sst_view(2, b"m", b"n");
        let sr_first = bounded_sst_view(1, b"a", b"b");
        let sr_last = bounded_sst_view(3, b"z", b"z");
        let core = &mut fixture
            .handler
            .state_writer
            .state
            .manifest_mut_for_test()
            .value
            .core;
        Arc::make_mut(&mut core.tree).l0 = VecDeque::from([l0.clone()]);
        Arc::make_mut(&mut core.tree).compacted = vec![SortedRun::new(1, [sr_first, sr_last])];
        let compaction_id = Ulid::new();
        fixture
            .handler
            .state_mut()
            .add_compaction(Compaction::new(
                compaction_id,
                CompactionSpec::new(vec![SourceId::SstView(l0.id), SourceId::SortedRun(1)], 2),
            ))
            .unwrap();

        fixture
            .handler
            .maybe_validate_submitted_compactions()
            .await
            .unwrap();

        let state = fixture.handler.state();
        assert_eq!(
            state
                .compactions()
                .value
                .get(&compaction_id)
                .unwrap()
                .status(),
            CompactionStatus::Scheduled
        );
        assert_eq!(state.db_state().tree.l0.len(), 1);
        assert_eq!(state.db_state().tree.compacted[0].id, 1);
    }

    #[tokio::test]
    async fn test_maybe_validate_submitted_compactions_marks_invalid_failed() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        let compaction_id = Ulid::new();
        fixture
            .handler
            .state_mut()
            .add_compaction(Compaction::new(
                compaction_id,
                CompactionSpec::new(Vec::new(), 0), // invalid: no sources
            ))
            .expect("failed to add compaction");

        fixture
            .handler
            .maybe_validate_submitted_compactions()
            .await
            .unwrap();

        let compactions = &fixture.handler.state_writer.state.compactions().value;
        assert_eq!(
            compactions
                .get(&compaction_id)
                .expect("missing compaction")
                .status(),
            CompactionStatus::Failed
        );
        let stored_compactions = fixture
            .compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions;
        assert_eq!(
            stored_compactions
                .get(&compaction_id)
                .expect("missing stored compaction")
                .status(),
            CompactionStatus::Failed
        );
    }

    #[tokio::test]
    async fn test_commit_compacted_ticker_skips_remote_refresh_when_idle() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        assert!(fixture
            .handler
            .state()
            .active_compactions()
            .next()
            .is_none());

        // Simulate an external submission arriving after the coordinator's last
        // regular poll. An idle fast-commit tick must not read it from storage.
        let remote_id = Ulid::new();
        let mut external = StoredCompactions::try_load(fixture.compactions_store.clone())
            .await
            .unwrap()
            .unwrap();
        let mut dirty = external.prepare_dirty().unwrap();
        dirty.value.insert(Compaction::new(
            remote_id,
            CompactionSpec::new(Vec::new(), 0),
        ));
        external.update(dirty).await.unwrap();

        fixture
            .handler
            .handle(CompactorMessage::CommitCompacted)
            .await
            .unwrap();
        assert!(
            !fixture
                .handler
                .state()
                .compactions()
                .value
                .contains(&remote_id),
            "idle fast-commit tick should not refresh .compactions"
        );

        // Once the coordinator has active work, the same fast tick must resume
        // refreshing so it can observe worker transitions promptly.
        let local_id = Ulid::new();
        fixture
            .handler
            .state_mut()
            .insert_compaction_for_test(Compaction::new(
                local_id,
                CompactionSpec::new(Vec::new(), 0),
            ));
        fixture
            .handler
            .handle(CompactorMessage::CommitCompacted)
            .await
            .unwrap();
        assert!(
            fixture
                .handler
                .state()
                .compactions()
                .value
                .contains(&remote_id),
            "active fast-commit tick should refresh .compactions"
        );
    }

    #[tokio::test]
    async fn test_handle_ticker_starts_preexisting_submitted_compaction() {
        let compactor_options = Arc::new(compactor_options());
        let options = db_options(None);

        let os = Arc::new(InMemory::new());
        let (manifest_store, compactions_store, _table_store) = build_test_stores(os.clone());
        let db = Db::builder(PATH, os.clone())
            .with_settings(options.clone())
            .build()
            .await
            .unwrap();

        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());
        let mut stored_manifest =
            StoredManifest::load(manifest_store.clone(), system_clock.clone())
                .await
                .unwrap();

        // Create an L0 so the submitted compaction has valid sources.
        let mut rng = rng::new_test_rng(None);
        let manifest = stored_manifest.refresh().await.unwrap();
        let l0s = manifest.core.tree.l0.len();
        let mut k = vec![0u8; options.l0_sst_size_bytes];
        rng.fill_bytes(&mut k);
        db.put(&k, &[b'x'; 10]).await.unwrap();
        db.flush().await.unwrap();
        loop {
            let manifest = stored_manifest.refresh().await.unwrap().clone();
            if manifest.core.tree.l0.len() > l0s {
                break;
            }
        }

        // Seed the compactions store with a Submitted compaction before handler startup.
        let db_state = stored_manifest.refresh().await.unwrap().core.clone();
        let sources = db_state
            .tree
            .l0
            .iter()
            .map(|h| SourceId::SstView(h.id))
            .collect::<Vec<_>>();
        let spec = CompactionSpec::new(sources, 0);
        let compaction_id = Ulid::new();
        let compaction = Compaction::new(compaction_id, spec);

        let mut stored_compactions = StoredCompactions::create(
            compactions_store.clone(),
            stored_manifest.manifest().compactor_epoch,
        )
        .await
        .unwrap();
        let mut dirty = stored_compactions.prepare_dirty().unwrap();
        dirty.value.insert(compaction);
        stored_compactions.update(dirty).await.unwrap();

        // Build the handler and trigger a ticker to pick up the pre-existing Submitted entry.
        let scheduler = Arc::new(MockScheduler::new());
        let rand = Arc::new(DbRand::default());
        let recorder = MetricsRecorderHelper::noop();
        let compactor_stats = Arc::new(CompactionStats::new(&recorder));
        let mut handler = CompactorEventHandler::new(
            manifest_store,
            compactions_store.clone(),
            compactor_options,
            scheduler,
            rand,
            compactor_stats,
            system_clock,
            recorder,
        )
        .await
        .unwrap();

        handler.handle_ticker().await.unwrap();

        // The pre-existing Submitted compaction should be promoted to Scheduled.
        let stored = compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions;
        assert_eq!(
            stored
                .get(&compaction_id)
                .expect("missing stored compaction")
                .status(),
            CompactionStatus::Scheduled
        );
    }

    #[tokio::test]
    async fn test_should_fail_when_compactions_store_fences_compactor() {
        // given:
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.write_l0().await;
        let compaction = fixture.build_l0_compaction().await;
        fixture.scheduler.inject_compaction(compaction.clone());

        // fence the handler by bumping the compactions epoch elsewhere
        let stored_compactions = StoredCompactions::load(fixture.compactions_store.clone())
            .await
            .unwrap();
        FenceableCompactions::init(
            stored_compactions,
            fixture.handler.options.manifest_update_timeout,
            fixture.handler.system_clock.clone(),
        )
        .await
        .unwrap();

        // when:
        let result = fixture.handler.handle_ticker().await;

        // then:
        assert!(matches!(result, Err(SlateDBError::Fenced)));
    }

    #[tokio::test]
    async fn test_should_error_when_finishing_if_compactions_fenced() {
        // given: coordinator schedules a compaction and a worker completes it,
        // leaving a `Compacted` entry for the coordinator to commit.
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.write_l0().await;
        let compaction = fixture.build_l0_compaction().await;
        fixture.scheduler.inject_compaction(compaction.clone());
        fixture.handler.handle_ticker().await.unwrap();
        fixture.simulate_worker_completes().await;

        // fence compactions before the coordinator commits the Compacted entry
        let stored_compactions = StoredCompactions::load(fixture.compactions_store.clone())
            .await
            .unwrap();
        FenceableCompactions::init(
            stored_compactions,
            fixture.handler.options.manifest_update_timeout,
            fixture.handler.system_clock.clone(),
        )
        .await
        .unwrap();

        // when: the coordinator attempts to commit while fenced
        let result = fixture.handler.handle_ticker().await;

        // then:
        assert!(matches!(result, Err(SlateDBError::Fenced)));
    }

    #[tokio::test]
    async fn test_should_not_schedule_conflicting_compaction() {
        // given: first ticker schedules a compaction (Scheduled in local state)
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.write_l0().await;
        let compaction = fixture.build_l0_compaction().await;
        fixture.scheduler.inject_compaction(compaction.clone());
        fixture.handler.handle_ticker().await.unwrap();
        assert_eq!(fixture.get_scheduled_compactions().await.len(), 1);
        fixture.write_l0().await;
        fixture.scheduler.inject_compaction(compaction.clone());

        // when: second ticker with same spec — add_compaction rejects duplicate destination
        fixture.handler.handle_ticker().await.unwrap();

        // then: still only one active compaction (no duplicate added)
        assert_eq!(1, fixture.handler.state().active_compactions().count());
    }

    #[tokio::test]
    async fn test_should_leave_checkpoint_when_removing_ssts_after_compaction() {
        // given:
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.write_l0().await;
        let compaction = fixture.build_l0_compaction().await;
        fixture.scheduler.inject_compaction(compaction.clone());
        fixture.handler.handle_ticker().await.unwrap();
        fixture.simulate_worker_completes().await;
        // when: coordinator commits
        fixture.handler.handle_ticker().await.unwrap();

        // then:
        let current_dbstate = fixture.latest_db_state().await;
        let checkpoint = current_dbstate.checkpoints.last().unwrap();
        let old_manifest = fixture
            .manifest_store
            .read_manifest(checkpoint.manifest_id)
            .await
            .unwrap();
        let l0_ids: Vec<SourceId> = old_manifest
            .core
            .tree
            .l0
            .iter()
            .map(|view| SourceId::SstView(view.id))
            .collect();
        assert_eq!(&l0_ids, compaction.sources());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[cfg(feature = "zstd")]
    async fn test_compactor_compressed_block_size() {
        use crate::compactor::stats::{BYTES_COMPACTED, SSTS_WRITTEN};
        use crate::config::{CompressionCodec, SstBlockSize};
        use slatedb_common::metrics::{lookup_metric, DefaultMetricsRecorder};

        // given:
        let os = Arc::new(InMemory::new());
        let system_clock = Arc::new(MockSystemClock::new());
        let scheduler_options = SizeTieredCompactionSchedulerOptions {
            min_compaction_sources: 1,
            max_compaction_sources: 999,
            include_size_threshold: 4.0,
            sorted_run_consolidation_threshold: 0,
        }
        .into();
        let mut options = db_options(Some(compactor_options()));
        options.l0_sst_size_bytes = 128;
        options.compression_codec = Some(CompressionCodec::Zstd);
        options
            .compactor_options
            .as_mut()
            .expect("compactor options missing")
            .scheduler_options = scheduler_options;

        let metrics_recorder = Arc::new(DefaultMetricsRecorder::new());
        let db = Db::builder(PATH, os.clone())
            .with_settings(options)
            .with_system_clock(system_clock.clone())
            .with_sst_block_size(SstBlockSize::Other(128))
            .with_metrics_recorder(metrics_recorder.clone())
            .build()
            .await
            .unwrap();

        for i in 0..4 {
            let k = vec![b'a' + i as u8; 16];
            let v = vec![b'b' + i as u8; 48];
            db.put(&k, &v).await.unwrap();
            let k = vec![b'j' + i as u8; 16];
            let v = vec![b'k' + i as u8; 48];
            db.put(&k, &v).await.unwrap();
        }

        db.flush().await.unwrap();

        // when:
        await_compaction(&db, os.clone(), Some(system_clock.clone()))
            .await
            .expect("db was not compacted");

        // then: the embedded worker recorded per-worker throughput.
        let bytes_compacted = lookup_metric(&metrics_recorder, BYTES_COMPACTED).unwrap();
        assert!(bytes_compacted > 0, "bytes_compacted: {}", bytes_compacted);
        let ssts_written = lookup_metric(&metrics_recorder, SSTS_WRITTEN).unwrap();
        assert!(ssts_written > 0, "ssts_written: {}", ssts_written);
    }

    #[tokio::test]
    async fn test_validate_compaction_empty_sources_rejected() {
        let fixture = CompactorEventHandlerTestFixture::new().await;
        let c = CompactionSpec::new(Vec::new(), 0);
        let err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), c))
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    #[tokio::test]
    async fn test_validate_compaction_rejects_missing_l0_source() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.handler.handle_ticker().await.unwrap();
        let c = CompactionSpec::new(vec![SourceId::SstView(Ulid::new())], 0);
        let err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), c))
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    #[tokio::test]
    async fn test_validate_compaction_rejects_missing_sr_source() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.handler.handle_ticker().await.unwrap();
        let c = CompactionSpec::new(vec![SourceId::SortedRun(42)], 42);
        let err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), c))
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    #[tokio::test]
    async fn test_validate_compaction_l0_only_ok_when_no_sr() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        // ensure at least one L0 exists
        fixture.write_l0().await;
        fixture.handler.handle_ticker().await.unwrap();
        let c = fixture.build_l0_compaction().await;
        fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), c))
            .unwrap();
    }

    #[tokio::test]
    async fn test_validate_compaction_l0_only_rejects_when_dest_below_highest_sr() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        // write L0 and compact to create SR id 0
        fixture.write_l0().await;
        let c1 = fixture.build_l0_compaction().await;
        fixture.scheduler.inject_compaction(c1.clone());
        fixture.handler.handle_ticker().await.unwrap();
        fixture.simulate_worker_completes().await;
        fixture.handler.handle_ticker().await.unwrap();

        // now highest_id should be 1; build L0-only compaction with dest 0 (below highest)
        fixture.write_l0().await;
        fixture.handler.handle_ticker().await.unwrap();
        let c2 = fixture.build_l0_compaction().await; // destination 0
        let err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), c2))
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    /// The L0-only monotonicity check is global across all segment trees, not
    /// just the target tree. A spec whose destination is above the target
    /// tree's local max but below the global max (an SR in some other
    /// segment) must be rejected.
    #[tokio::test]
    async fn test_validate_compaction_l0_only_rejects_when_dest_below_global_highest_sr() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        let prefix = Bytes::from_static(b"seg/");
        let l0_view = Ulid::from_parts(1, 0);
        let make_view = |id: Ulid| {
            SsTableView::identity(SsTableHandle::new(
                SsTableId::from(id),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo::default(),
            ))
        };
        let core = &mut fixture
            .handler
            .state_writer
            .state
            .manifest_mut_for_test()
            .value
            .core;
        // Root tree holds SR(7) — the global max. The segment-targeted spec
        // below proposes dst=3, which is above the segment's local max (0)
        // but below the global max.
        Arc::make_mut(&mut core.tree).compacted = vec![SortedRun::new(7, [])];
        core.segments = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![make_view(l0_view)]),
                compacted: Vec::new(),
            }),
        }];

        let spec = CompactionSpec::for_segment(prefix, vec![SourceId::SstView(l0_view)], 3);
        let err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), spec))
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    /// Completion validation must not reuse the Submitted-only fresh-SR check.
    /// Cross-segment L0 compactions can commit out of order: a job submitted
    /// earlier with destination 3 is still valid even if another segment has
    /// since committed destination 7.
    #[tokio::test]
    async fn test_validate_completed_l0_allows_destination_below_global_highest_sr() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        let prefix = Bytes::from_static(b"seg/");
        let l0_view = Ulid::from_parts(1, 0);
        let make_view = |id: Ulid| {
            SsTableView::identity(SsTableHandle::new(
                SsTableId::from(id),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo::default(),
            ))
        };
        let core = &mut fixture
            .handler
            .state_writer
            .state
            .manifest_mut_for_test()
            .value
            .core;
        Arc::make_mut(&mut core.tree).compacted = vec![SortedRun::new(7, [])];
        core.segments = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![make_view(l0_view)]),
                compacted: Vec::new(),
            }),
        }];

        let spec = CompactionSpec::for_segment(prefix, vec![SourceId::SstView(l0_view)], 3);

        let submitted_err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), spec.clone()))
            .unwrap_err();
        assert!(matches!(submitted_err, SlateDBError::InvalidCompaction));
        let completed = Compaction::new(Ulid::new(), spec).with_status(CompactionStatus::Compacted);
        fixture
            .handler
            .validate_compaction(&completed)
            .expect("completed L0 compaction should not require a still-fresh destination");
    }

    #[tokio::test]
    async fn test_validate_completed_l0_rejects_destination_below_segment_highest_sr() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        let prefix = Bytes::from_static(b"seg/");
        let l0_view = Ulid::from_parts(1, 0);
        let make_view = |id: Ulid| {
            SsTableView::identity(SsTableHandle::new(
                SsTableId::from(id),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo::default(),
            ))
        };
        fixture
            .handler
            .state_writer
            .state
            .manifest_mut_for_test()
            .value
            .core
            .segments = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![make_view(l0_view)]),
                compacted: vec![SortedRun::new(7, [])],
            }),
        }];

        let spec = CompactionSpec::for_segment(prefix, vec![SourceId::SstView(l0_view)], 3);
        let completed = Compaction::new(Ulid::new(), spec).with_status(CompactionStatus::Compacted);

        let err = fixture.handler.validate_compaction(&completed).unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    #[tokio::test]
    async fn test_validate_compaction_mixed_l0_and_sr_deferred_to_scheduler() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        // create one SR so we can reference its id
        fixture.write_l0().await;
        let c1 = fixture.build_l0_compaction().await;
        fixture.scheduler.inject_compaction(c1.clone());
        fixture.handler.handle_ticker().await.unwrap();
        fixture.simulate_worker_completes().await;
        fixture.handler.handle_ticker().await.unwrap();

        // prepare a mixed compaction: one SR source and one L0 source
        fixture.write_l0().await;
        fixture.handler.handle_ticker().await.unwrap();
        let state = fixture.latest_db_state().await;
        let sr_id = state.tree.compacted.first().unwrap().id;
        let l0_view_id = state.tree.l0.front().unwrap().id;
        let mixed = CompactionSpec::new(
            vec![SourceId::SortedRun(sr_id), SourceId::SstView(l0_view_id)],
            sr_id,
        );
        // Compactor-level validation should not reject (scheduler default validate returns Ok(()))
        fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), mixed))
            .unwrap();
    }

    #[tokio::test]
    async fn test_validate_compaction_rejects_parallel_l0() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        // write two L0s so we can build two disjoint L0 compactions
        fixture.write_l0().await;
        fixture.write_l0().await;
        fixture.handler.handle_ticker().await.unwrap();

        let state = fixture.latest_db_state().await;
        assert!(state.tree.l0.len() >= 2);

        // Build first L0 compaction from the oldest L0
        let first_l0 =
            CompactionSpec::new(vec![SourceId::SstView(state.tree.l0.back().unwrap().id)], 0);
        // Inject and schedule it so it becomes active
        fixture.scheduler.inject_compaction(first_l0.clone());
        fixture.handler.handle_ticker().await.unwrap();

        // Build second L0 compaction from the newest L0 (disjoint sources)
        let second_l0 = CompactionSpec::new(
            vec![SourceId::SstView(state.tree.l0.front().unwrap().id)],
            1,
        );
        let err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), second_l0))
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    /// L0 compactions in disjoint segments may run concurrently — each segment
    /// owns its own watermark, so cross-segment parallelism is safe (RFC-0024).
    /// The parallel-L0 rejection in `validate_compaction` must scope itself to
    /// the spec's target segment.
    ///
    /// This test inspects `validate_compaction` directly by seeding the
    /// compactor's local state with two segments and a Running spec — going
    /// through the full schedule/start round-trip would require a writer
    /// configured with a segment extractor, which the fixture doesn't model.
    #[tokio::test]
    async fn test_validate_compaction_allows_parallel_l0_in_disjoint_segments() {
        use crate::manifest::{LsmTreeState, Segment};
        use bytes::Bytes;

        let mut fixture = CompactorEventHandlerTestFixture::new().await;

        let prefix_a = Bytes::from_static(b"seg-a/");
        let prefix_b = Bytes::from_static(b"seg-b/");
        let l0_a = Ulid::from_parts(1, 0);
        let l0_b = Ulid::from_parts(2, 0);
        let make_segment = |prefix: Bytes, l0_view_id: Ulid| Segment {
            prefix,
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![SsTableView::identity(SsTableHandle::new(
                    SsTableId::from(l0_view_id),
                    SST_FORMAT_VERSION_LATEST,
                    SsTableInfo::default(),
                ))]),
                compacted: Vec::new(),
            }),
        };

        // Seed the compactor's local state with the two segments. We never
        // call handle_ticker again, so the writer/compactor merge protocol
        // (which would reject compactor-only segments with data) doesn't
        // run.
        fixture
            .handler
            .state_writer
            .state
            .manifest_mut_for_test()
            .value
            .core
            .segments = vec![
            make_segment(prefix_a.clone(), l0_a),
            make_segment(prefix_b.clone(), l0_b),
        ];

        // Seed a Running L0 compaction in segment A.
        let running_id = Ulid::from_parts(100, 0);
        let running_spec =
            CompactionSpec::for_segment(prefix_a.clone(), vec![SourceId::SstView(l0_a)], 200);
        fixture
            .handler
            .state_writer
            .state
            .insert_compaction_for_test(
                Compaction::new(running_id, running_spec).with_status(CompactionStatus::Running),
            );

        // L0 in a different segment must validate successfully — its
        // watermark is independent of segment A's.
        let spec_b =
            CompactionSpec::for_segment(prefix_b.clone(), vec![SourceId::SstView(l0_b)], 201);
        fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), spec_b))
            .expect("L0 in disjoint segment must be allowed");

        // Sanity check: a second L0 spec in the SAME segment is still rejected.
        let spec_a_dup =
            CompactionSpec::for_segment(prefix_a.clone(), vec![SourceId::SstView(l0_a)], 202);
        let err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), spec_a_dup))
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    /// `validate_compaction` rejects a spec whose target segment does not exist in
    /// the manifest. The empty-prefix segment (root tree) always exists, so only
    /// non-empty prefixes can fail this lookup.
    #[tokio::test]
    async fn test_validate_compaction_rejects_unknown_segment() {
        let fixture = CompactorEventHandlerTestFixture::new().await;
        let spec = CompactionSpec::for_segment(
            Bytes::from_static(b"missing/"),
            vec![SourceId::SortedRun(0)],
            0,
        );
        let err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), spec))
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    /// RFC-0024: a compaction must operate within a single segment. A spec whose
    /// sources reference a sorted run that lives in a different tree is rejected
    /// because the source is absent from the target segment's tree.
    #[tokio::test]
    async fn test_validate_compaction_rejects_sources_from_other_segment() {
        use crate::manifest::{LsmTreeState, Segment};

        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        // SR(99) lives in the root tree; segment "seg/" is empty.
        let core = &mut fixture
            .handler
            .state_writer
            .state
            .manifest_mut_for_test()
            .value
            .core;
        Arc::make_mut(&mut core.tree).compacted = vec![SortedRun::new(99, [])];
        let prefix = Bytes::from_static(b"seg/");
        core.segments = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::new(),
                compacted: Vec::new(),
            }),
        }];

        // Segment-targeted spec lists SR(99) as a source — but SR(99) lives in
        // the root tree, not in "seg/". Must be rejected.
        let spec = CompactionSpec::for_segment(prefix, vec![SourceId::SortedRun(99)], 0);
        let err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), spec))
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    /// SR ids are globally unique across all trees (RFC-0024), so a segment-targeted
    /// spec must be rejected if its destination already exists as an SR anywhere in
    /// the manifest — including the root tree — and the SR is not listed among its
    /// sources.
    #[tokio::test]
    async fn test_validate_compaction_destination_overwrite_check_is_global() {
        use crate::manifest::{LsmTreeState, Segment};

        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        let core = &mut fixture
            .handler
            .state_writer
            .state
            .manifest_mut_for_test()
            .value
            .core;
        // Place SR(7) in the root tree. The segment-targeted spec below uses 7 as
        // its destination but does not list it among its sources.
        Arc::make_mut(&mut core.tree).compacted = vec![SortedRun::new(7, [])];
        // Seed SR(99) into the segment so the source-existence check passes and
        // destination-overwrite is the rejection reason.
        let prefix = Bytes::from_static(b"seg/");
        core.segments = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::new(),
                compacted: vec![SortedRun::new(99, [])],
            }),
        }];

        let spec = CompactionSpec::for_segment(prefix, vec![SourceId::SortedRun(99)], 7);
        let err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), spec))
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    /// Drain specs cannot target the empty-prefix segment (RFC-0024): the
    /// root-tree compatibility encoding is not retired by drain.
    #[tokio::test]
    async fn test_validate_compaction_rejects_drain_for_empty_prefix() {
        let fixture = CompactorEventHandlerTestFixture::new().await;
        let spec = CompactionSpec::drain_segment(Bytes::new(), vec![SourceId::SortedRun(0)]);
        let err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), spec))
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    /// A drain spec must list every L0 at or below the newest drained L0 in
    /// the target tree — otherwise the watermark advances over an un-listed
    /// L0 and leaves it orphaned in `tree.l0` below the watermark.
    #[tokio::test]
    async fn test_validate_compaction_rejects_drain_with_non_contiguous_l0_sources() {
        use crate::manifest::{LsmTreeState, Segment};

        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        let prefix = Bytes::from_static(b"seg/");
        // tree.l0 is newest-first: [L0_3, L0_2, L0_1]. Drain spec names
        // L0_3 and L0_1 but skips L0_2 — the watermark would advance to L0_3
        // and L0_2 would be left below it.
        let l0_3 = Ulid::from_parts(3, 0);
        let l0_2 = Ulid::from_parts(2, 0);
        let l0_1 = Ulid::from_parts(1, 0);
        let make_view = |id: Ulid| {
            SsTableView::identity(SsTableHandle::new(
                SsTableId::from(id),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo::default(),
            ))
        };
        fixture
            .handler
            .state_writer
            .state
            .manifest_mut_for_test()
            .value
            .core
            .segments = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![make_view(l0_3), make_view(l0_2), make_view(l0_1)]),
                compacted: Vec::new(),
            }),
        }];

        let spec = CompactionSpec::drain_segment(
            prefix,
            vec![SourceId::SstView(l0_3), SourceId::SstView(l0_1)],
        );
        let err = fixture
            .handler
            .validate_compaction(&Compaction::new(Ulid::new(), spec))
            .unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    async fn put_and_flush_memtable(db: &Db, key: &[u8], value: &[u8]) {
        db.put_with_options(
            key,
            value,
            &PutOptions::default(),
            &WriteOptions {
                ..Default::default()
            },
        )
        .await
        .unwrap();
        db.flush_with_options(FlushOptions {
            flush_type: FlushType::MemTable,
        })
        .await
        .unwrap();
    }

    fn read_db_state_core(db: &Db) -> ManifestCore {
        let db_state = db.inner.state.read();
        db_state.state().core().clone()
    }

    async fn run_for<T, F>(duration: Duration, f: impl Fn() -> F) -> Option<T>
    where
        F: Future<Output = Option<T>>,
    {
        #[allow(clippy::disallowed_methods)]
        let now = SystemTime::now();
        while now.elapsed().unwrap() < duration {
            let maybe_result = f().await;
            if maybe_result.is_some() {
                return maybe_result;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        None
    }

    fn build_test_stores(
        os: Arc<dyn ObjectStore>,
    ) -> (Arc<ManifestStore>, Arc<CompactionsStore>, Arc<TableStore>) {
        let sst_format = SsTableFormat {
            block_size: 32,
            min_filter_keys: 10,
            ..SsTableFormat::default()
        };
        let manifest_store = Arc::new(ManifestStore::new(&Path::from(PATH), os.clone()));
        let compactions_store = Arc::new(CompactionsStore::new(&Path::from(PATH), os.clone()));
        let table_store = Arc::new(TableStore::new(
            os.clone(),
            sst_format,
            Path::from(PATH),
            None,
            TableStoreKind::Compactor,
            BlockCachePolicy::default(),
        ));
        (manifest_store, compactions_store, table_store)
    }

    /// Waits until all writes have made their way to L1 or below. No data is allowed in
    /// in-memory WALs, in-memory memtables, or L0 SSTs on object storage.
    ///
    /// If a clock is provided, it will be advanced the clock by 60 seconds on each iteration to
    /// trigger time-based flushes and compactions.
    async fn await_compaction(
        db: &Db,
        os: Arc<dyn ObjectStore>,
        clock: Option<Arc<dyn SystemClock>>,
    ) -> Option<ManifestCore> {
        await_compaction_matching(db, os, clock, |_| true).await
    }

    async fn await_compaction_matching(
        db: &Db,
        os: Arc<dyn ObjectStore>,
        clock: Option<Arc<dyn SystemClock>>,
        predicate: impl Fn(&ManifestCore) -> bool,
    ) -> Option<ManifestCore> {
        let manifest_store = Arc::new(ManifestStore::new(&Path::from(PATH), os.clone()));
        run_for(Duration::from_secs(10), || async {
            if let Some(clock) = &clock {
                clock.as_ref().advance(Duration::from_millis(60000)).await;
            }
            let (empty_wal, empty_memtable, core_db_state) = {
                let db_state = db.inner.state.read();
                let cow_db_state = db_state.state();
                (
                    db.inner
                        .wal_observer
                        .status()
                        .unwrap()
                        .buffered_wal_entries_count
                        == 0,
                    db_state.memtable().is_empty() && cow_db_state.imm_memtable.is_empty(),
                    db_state.state().core().clone(),
                )
            };

            let empty_l0 = core_db_state.tree.l0.is_empty();
            let compaction_ran = !core_db_state.tree.compacted.is_empty();

            if empty_wal
                && empty_memtable
                && empty_l0
                && compaction_ran
                && predicate(&core_db_state)
            {
                return Some(get_db_state(manifest_store.clone()).await);
            }
            None
        })
        .await
    }

    fn has_single_output_sst(db_state: &ManifestCore) -> bool {
        db_state.tree.compacted.len() == 1
            && db_state
                .tree
                .compacted
                .first()
                .is_some_and(|sr| sr.sst_views().len() == 1)
    }

    /// If a clock is provided, it will be advanced the clock by 60 seconds on each iteration to
    /// trigger time-based flushes and compactions.
    #[allow(unused)] // only used with feature(wal_disable)
    async fn await_compacted_compaction(
        manifest_store: Arc<ManifestStore>,
        old_compacted: Vec<SortedRun>,
        clock: Option<Arc<dyn SystemClock>>,
    ) -> Option<ManifestCore> {
        run_for(Duration::from_secs(10), || async {
            if let Some(clock) = &clock {
                clock.as_ref().advance(Duration::from_millis(60000)).await;
            }
            let db_state = get_db_state(manifest_store.clone()).await;
            if !db_state.tree.compacted.eq(&old_compacted) {
                return Some(db_state);
            }
            None
        })
        .await
    }

    async fn get_db_state(manifest_store: Arc<ManifestStore>) -> ManifestCore {
        let stored_manifest =
            StoredManifest::load(manifest_store.clone(), Arc::new(DefaultSystemClock::new()))
                .await
                .unwrap();
        stored_manifest.db_state().clone()
    }

    fn db_options(compactor_options: Option<CompactorOptions>) -> Settings {
        Settings {
            flush_interval: Some(Duration::from_millis(100)),
            #[cfg(feature = "wal_disable")]
            wal_enabled: true,
            manifest_poll_interval: Duration::from_millis(100),
            manifest_update_timeout: Duration::from_secs(300),
            l0_sst_size_bytes: 256,
            l0_max_ssts: 8,
            compactor_options,
            ..Settings::default()
        }
    }

    fn compactor_options() -> CompactorOptions {
        CompactorOptions {
            poll_interval: Duration::from_millis(100),
            max_concurrent_compactions: 1,
            scheduler_options: Default::default(),
            worker: Some(CompactionWorkerOptions {
                compactions_poll_interval: Duration::from_millis(100),
                ..CompactionWorkerOptions::default()
            }),
            ..CompactorOptions::default()
        }
    }

    fn compactor_builder_with_scheduler(
        os: Arc<dyn ObjectStore>,
        scheduler_supplier: Arc<dyn CompactionSchedulerSupplier>,
        system_clock: Arc<dyn SystemClock>,
    ) -> CompactorBuilder<&'static str> {
        CompactorBuilder::new(PATH, os)
            .with_system_clock(system_clock)
            .with_options(compactor_options())
            .with_scheduler_supplier(scheduler_supplier)
    }

    struct MockSchedulerInner {
        compaction: Vec<CompactionSpec>,
    }

    #[derive(Clone)]
    struct MockScheduler {
        inner: Arc<Mutex<MockSchedulerInner>>,
    }

    impl MockScheduler {
        fn new() -> Self {
            Self {
                inner: Arc::new(Mutex::new(MockSchedulerInner { compaction: vec![] })),
            }
        }

        fn inject_compaction(&self, compaction: CompactionSpec) {
            let mut inner = self.inner.lock();
            inner.compaction.push(compaction);
        }
    }

    impl CompactionScheduler for MockScheduler {
        fn propose(&self, _state: &CompactorStateView) -> Vec<CompactionSpec> {
            let mut inner = self.inner.lock();
            std::mem::take(&mut inner.compaction)
        }
    }

    // Builds a fake output SsTableHandle with first_entry set so that
    // SsTableView::identity can derive a non-empty effective range.
    fn fake_output_sst() -> SsTableHandle {
        SsTableHandle::new(
            SsTableId::from(Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo {
                first_entry: Some(Bytes::from_static(b"a")),
                ..SsTableInfo::default()
            },
        )
    }

    // Builds a Compacted compaction whose output is recorded on a single
    // unbounded-range subcompaction, mirroring what a worker persists on finish.
    fn compacted_with_output(
        id: Ulid,
        spec: CompactionSpec,
        output: Vec<SsTableHandle>,
    ) -> Compaction {
        Compaction::new(id, spec)
            .with_status(CompactionStatus::Compacted)
            .with_output_ssts(output)
    }

    #[tokio::test]
    async fn test_commit_compacted_entries_writes_manifest() {
        // given: a handler with one L0 in the manifest
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.write_l0().await;
        fixture.handler.state_writer.refresh().await.unwrap();

        let db_state = fixture.handler.state().db_state().clone();
        let sources: Vec<SourceId> = db_state
            .tree
            .l0
            .iter()
            .map(|view| SourceId::SstView(view.id))
            .collect();
        let destination = 0u32;
        let spec = CompactionSpec::new(sources, destination);
        let compaction_id = Ulid::from_parts(1, 0);
        let output_sst = fake_output_sst();
        let compaction = compacted_with_output(compaction_id, spec, vec![output_sst.clone()]);

        // inject the Compacted compaction into state (bypassing the executor)
        fixture
            .handler
            .state_mut()
            .insert_compaction_for_test(compaction);

        fixture.handler.state_writer.refresh().await.unwrap();

        // when: the coordinator observes and commits the Compacted entry
        fixture
            .handler
            .commit_compacted_entries()
            .await
            .expect("commit_compacted_entries failed");

        // then: compaction is Completed, L0 sources removed, output SR added
        let stored = fixture
            .compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions;
        assert_eq!(
            stored
                .get(&compaction_id)
                .expect("missing compaction")
                .status(),
            CompactionStatus::Completed,
        );
        let manifest = fixture.manifest_store.read_latest_manifest().await.unwrap();
        let core = manifest.core();
        assert!(core.tree.l0.is_empty(), "L0 sources should be removed");
        let sr = core.tree.compacted.iter().find(|sr| sr.id == destination);
        assert!(
            sr.is_some(),
            "output SR {destination} not found in manifest"
        );
        assert_eq!(
            sr.unwrap().sst_views().first().unwrap().sst.id,
            output_sst.id
        );

        // given: a Compacted SR0→SR1 compaction to validate the SR source path removed when not in L0
        let sr1_output_sst = fake_output_sst();
        let sr_compaction_id = Ulid::from_parts(2, 0);
        let sr_compaction = compacted_with_output(
            sr_compaction_id,
            CompactionSpec::new(vec![SourceId::SortedRun(0)], 1),
            vec![sr1_output_sst.clone()],
        );
        fixture
            .handler
            .state_mut()
            .insert_compaction_for_test(sr_compaction);

        // when:
        fixture
            .handler
            .commit_compacted_entries()
            .await
            .expect("SR commit failed");

        // then: SR 0 removed, SR 1 added with the correct SST
        let manifest2 = fixture.manifest_store.read_latest_manifest().await.unwrap();
        let core2 = manifest2.core();
        assert!(
            core2.tree.compacted.iter().all(|sr| sr.id != 0),
            "SR 0 should be removed after SR compaction"
        );
        let sr1 = core2.tree.compacted.iter().find(|sr| sr.id == 1);
        assert!(sr1.is_some(), "SR 1 should exist");
        assert_eq!(
            sr1.unwrap().sst_views().first().unwrap().sst.id,
            sr1_output_sst.id
        );
        let stored2 = fixture
            .compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions;
        assert_eq!(
            stored2
                .get(&sr_compaction_id)
                .expect("missing SR compaction")
                .status(),
            CompactionStatus::Completed,
        );
    }

    #[tokio::test]
    async fn test_commit_compacted_entries_marks_failed_when_sources_absent() {
        // given: a handler where the Compacted compaction references a source
        // that no longer exists in the manifest (simulates a post-crash recovery
        // where the manifest was already written before the coordinator crashed)
        let mut fixture = CompactorEventHandlerTestFixture::new().await;
        fixture.handler.state_writer.refresh().await.unwrap();

        // Use a fake view ID that is not present in any L0
        let ghost_view_id = Ulid::from_parts(u64::MAX, 0);
        let spec = CompactionSpec::new(vec![SourceId::SstView(ghost_view_id)], 0);
        let compaction_id = Ulid::new();
        let compaction = compacted_with_output(compaction_id, spec, vec![fake_output_sst()]);

        fixture
            .handler
            .state_mut()
            .insert_compaction_for_test(compaction);
        let manifest_id_before = fixture
            .manifest_store
            .read_latest_manifest()
            .await
            .unwrap()
            .id;

        // when:
        fixture
            .handler
            .commit_compacted_entries()
            .await
            .expect("commit_compacted_entries failed");

        // then: the compaction is marked Failed (not retried)
        let stored = fixture
            .compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions;
        assert_eq!(
            stored
                .get(&compaction_id)
                .expect("missing compaction")
                .status(),
            CompactionStatus::Failed,
        );
        let manifest_id_after = fixture
            .manifest_store
            .read_latest_manifest()
            .await
            .unwrap()
            .id;
        assert_eq!(
            manifest_id_after, manifest_id_before,
            "validation-only failures must not checkpoint or rewrite the manifest"
        );
    }

    /// A Running compaction whose heartbeat is older than the timeout must be
    /// reset to Scheduled (worker cleared) by `reclaim_stale_workers`.
    #[tokio::test]
    async fn test_reclaim_stale_running_compaction() {
        // given: clock starts at 0 ms
        let system_clock = Arc::new(MockSystemClock::new());
        let timeout = Duration::from_secs(30);
        let options = Arc::new(CompactorOptions {
            worker_heartbeat_timeout: timeout,
            ..compactor_options()
        });
        let mut fixture = CompactorEventHandlerTestFixture::new_with_clock(
            system_clock.clone() as Arc<dyn SystemClock>,
            options,
        )
        .await;

        fixture.handler.state_writer.refresh().await.unwrap();

        // Seed a Running compaction in-memory with last_heartbeat_ms = 0 (epoch).
        // Then persist it so that the compactions_store reflects the seeded state.
        let compaction_id = Ulid::new();
        let worker = WorkerSpec::new("worker-1".to_string(), 0);
        let compaction = Compaction::new(compaction_id, CompactionSpec::new(vec![], 0))
            .with_status(CompactionStatus::Running)
            .with_worker(Some(worker));
        fixture
            .handler
            .state_mut()
            .insert_compaction_for_test(compaction);
        fixture
            .handler
            .state_writer
            .write_compactions_safely()
            .await
            .expect("failed to persist seeded compaction");

        // Advance clock well past the timeout (2× the timeout = 60 s).
        system_clock.advance(Duration::from_secs(60)).await;

        // when: call reclaim_stale_workers directly to avoid the scheduling
        // side-effects of handle_ticker (which would try to claim the reclaimed
        // Scheduled compaction with an empty spec and mark it Failed).
        fixture
            .handler
            .reclaim_stale_workers()
            .await
            .expect("reclaim_stale_workers failed");

        // then: the compaction is now Scheduled with no worker.
        let stored = fixture
            .compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions;
        let c = stored.get(&compaction_id).expect("compaction missing");
        assert_eq!(
            c.status(),
            CompactionStatus::Scheduled,
            "should be reclaimed"
        );
        assert!(c.worker().is_none(), "worker should be cleared");

        // and: the reclamation is counted.
        let reclaimed =
            slatedb_common::metrics::lookup_metric(&fixture.test_recorder, stats::JOBS_RECLAIMED)
                .expect("metric not found");
        assert_eq!(reclaimed, 1, "one job should be counted as reclaimed");
    }

    /// `jobs_claimed` counts every job inherited by the coordinator on its first
    /// metrics update, then counts each newly claimed job exactly once across the
    /// `Running` and `Compacted` states.
    #[tokio::test]
    async fn test_jobs_claimed_metric_counts_inherited_then_new_claims() {
        let mut fixture = CompactorEventHandlerTestFixture::new().await;

        let claimed_count = || {
            slatedb_common::metrics::lookup_metric(&fixture.test_recorder, stats::JOBS_CLAIMED)
                .expect("metric not found")
        };

        // given: a job already claimed (Running) before the coordinator's first tick.
        let id1 = Ulid::new();
        fixture.handler.state_mut().insert_compaction_for_test(
            Compaction::new(id1, CompactionSpec::new(vec![], 0))
                .with_status(CompactionStatus::Running)
                .with_worker(Some(WorkerSpec::new("worker-1".to_string(), 0))),
        );

        // when: the first snapshot inherits the existing claim.
        fixture.handler.update_distributed_compaction_metrics();
        // then: the inherited job is counted once.
        assert_eq!(claimed_count(), 1);

        // when: a new job is claimed.
        let id2 = Ulid::new();
        fixture.handler.state_mut().insert_compaction_for_test(
            Compaction::new(id2, CompactionSpec::new(vec![], 0))
                .with_status(CompactionStatus::Running)
                .with_worker(Some(WorkerSpec::new("worker-1".to_string(), 0))),
        );
        fixture.handler.update_distributed_compaction_metrics();
        // then: it is counted once.
        assert_eq!(claimed_count(), 2);

        // when: that job finishes execution (Compacted, worker retained) and we
        // tick again.
        fixture.handler.state_mut().update_compaction(&id2, |c| {
            c.set_status(CompactionStatus::Compacted);
        });
        fixture.handler.update_distributed_compaction_metrics();
        // then: a still-claimed job is not recounted.
        assert_eq!(claimed_count(), 2);
    }

    /// A Running compaction whose heartbeat is within the timeout must NOT be
    /// reclaimed by `reclaim_stale_workers`.
    #[tokio::test]
    async fn test_does_not_reclaim_fresh_running_compaction() {
        // given: clock starts at 0 ms; heartbeat is also at 0 ms.
        let system_clock = Arc::new(MockSystemClock::new());
        let timeout = Duration::from_secs(30);
        let options = Arc::new(CompactorOptions {
            worker_heartbeat_timeout: timeout,
            ..compactor_options()
        });
        let mut fixture = CompactorEventHandlerTestFixture::new_with_clock(
            system_clock.clone() as Arc<dyn SystemClock>,
            options,
        )
        .await;

        fixture.handler.state_writer.refresh().await.unwrap();

        // Seed a Running compaction with heartbeat = "now" (0 ms).
        let compaction_id = Ulid::new();
        let now_ms = system_clock.now().timestamp_millis() as u64;
        let worker = WorkerSpec::new("worker-2".to_string(), now_ms);
        let compaction = Compaction::new(compaction_id, CompactionSpec::new(vec![], 0))
            .with_status(CompactionStatus::Running)
            .with_worker(Some(worker));
        fixture
            .handler
            .state_mut()
            .insert_compaction_for_test(compaction);
        fixture
            .handler
            .state_writer
            .write_compactions_safely()
            .await
            .expect("failed to persist seeded compaction");

        // Advance clock by only 5 s — well within the 30 s timeout.
        system_clock.advance(Duration::from_secs(5)).await;

        // when:
        fixture
            .handler
            .reclaim_stale_workers()
            .await
            .expect("reclaim_stale_workers failed");

        // then: the compaction is still Running (no reclaim write happened).
        // Since nothing was written, re-read from the store and verify.
        let stored = fixture
            .compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions;
        let c = stored.get(&compaction_id).expect("compaction missing");
        assert_eq!(
            c.status(),
            CompactionStatus::Running,
            "should NOT be reclaimed"
        );
        assert!(c.worker().is_some(), "worker should still be set");
    }

    /// A Running compaction without a worker violates the claim protocol. It
    /// panics in debug builds and is reclaimed in release builds so it cannot
    /// remain stuck in Running forever.
    #[tokio::test]
    #[cfg_attr(
        debug_assertions,
        should_panic(expected = "reclaiming Running compaction that has no worker")
    )]
    async fn test_reclaim_worker_less_running_compaction() {
        let system_clock = Arc::new(MockSystemClock::new());
        let options = Arc::new(CompactorOptions {
            worker_heartbeat_timeout: Duration::from_secs(30),
            ..compactor_options()
        });
        let mut fixture = CompactorEventHandlerTestFixture::new_with_clock(
            system_clock.clone() as Arc<dyn SystemClock>,
            options,
        )
        .await;

        fixture.handler.state_writer.refresh().await.unwrap();

        // Seed a Running compaction with no worker set.
        let compaction_id = Ulid::new();
        let compaction = Compaction::new(compaction_id, CompactionSpec::new(vec![], 0))
            .with_status(CompactionStatus::Running);
        fixture
            .handler
            .state_mut()
            .insert_compaction_for_test(compaction);
        fixture
            .handler
            .state_writer
            .write_compactions_safely()
            .await
            .expect("failed to persist seeded compaction");

        // when: reclaim should treat the worker-less entry as stale regardless
        // of how much time has (not) passed.
        fixture
            .handler
            .reclaim_stale_workers()
            .await
            .expect("reclaim_stale_workers failed");

        // then: the compaction is now Scheduled with no worker.
        let stored = fixture
            .compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions;
        let c = stored.get(&compaction_id).expect("compaction missing");
        assert_eq!(
            c.status(),
            CompactionStatus::Scheduled,
            "should be reclaimed"
        );
        assert!(c.worker().is_none(), "worker should remain cleared");
    }
}
