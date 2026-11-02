//! Distributed-compaction worker (RFC-0025).
//!
//! A [`CompactionWorker`] polls `.compactions` for `Scheduled` entries, claims
//! them via the optimistic CAS protocol described in RFC-0025, executes the
//! compaction with the same code path the in-process executor uses, and writes
//! `Compacted` (with the produced output recorded on the job's subcompaction)
//! back to `.compactions`. The coordinator separately observes those
//! `Compacted` entries and commits the manifest update (see
//! [`crate::compactor::CompactorEventHandler::commit_compacted_entries`]).
//!
//! # Deployment patterns
//!
//! Workers run in one of two modes:
//!
//! 1. **Embedded with Hybrid Optionality** a single worker is spawned inside the compaction coordinator's process. (
//!    The coordinator must have `worker: Some(CompactionWorkerOptions))` in its [`crate::config::CompactorOptions`].
//!    This is the default. Additional (non-embedded) workers may be started in addition to the embedded worker to
//!    satisfy scaling needs. This doesn't cause fencing and is an intended usage pattern.
//!
//! 2. **Standalone**: The compaction coordinator runs without an embedded worker and one or
//!    more separate worker processes each run a [`CompactionWorker`]. The coordinator must
//!    have `worker: None` in its [`crate::config::CompactorOptions`].
//!
//! # Heartbeat and failure detection
//!
//! Workers emit heartbeats to prove liveness. A heartbeat is a CAS write that
//! bumps `last_heartbeat_ms` in the worker's `.compactions` entry. Every
//! `heartbeat_interval`, the worker refreshes liveness for every active
//! job it still owns and publishes the latest compaction context reported by
//! the executor (the plan and each range's output SSTs), so a reclaiming
//! worker can resume completed ranges. The heartbeat ticker is the only path
//! that writes worker progress; executor progress reports are buffered in
//! memory until the next tick.
//!
//! The coordinator reclaims stale Running compactions whose
//! `last_heartbeat_ms` is older than
//! [`crate::config::CompactorOptions::worker_heartbeat_timeout`].
//! Reclaimed jobs resume from their last persisted state (`output_ssts`) when the
//! next worker picks them up.
//!
//! # Metrics
//!
//! Workers emit the following per-worker metrics labeled `{worker_id=<id>}`:
//!
//! | Metric | Description |
//! |---|---|
//! | `slatedb.compactor.bytes_compacted` | Bytes merged by this worker |
//! | `slatedb.compactor.running_compactions` | Jobs currently in-flight |
//! | `slatedb.compactor.ssts_written` | Output SSTs produced |
//!
//! Supply a recorder via [`CompactionWorkerBuilder::with_metrics_recorder`].
//! The coordinator emits complementary metrics (`jobs_claimed`, `jobs_reclaimed`,
//! `worker_last_heartbeat_ms`) on its own recorder.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use fail_parallel::{fail_point, FailPointRegistry};
use futures::stream::BoxStream;
use log::{debug, error, info, warn};
use tokio::runtime::Handle;
use ulid::Ulid;

use crate::compactions_store::{CompactionsStore, StoredCompactions};
use crate::compactor::stats::{CompactionStats, WorkerStats};
use crate::compactor_executor::{
    CompactionExecutor, StartCompactionJobArgs, TokioCompactionExecutor,
    TokioCompactionExecutorOptions,
};
use crate::compactor_state::{Compaction, CompactionContext, CompactionStatus, WorkerSpec};
use crate::config::CompactionWorkerOptions;
use crate::db_state::SortedRun;
use crate::dispatcher::{MessageHandler, MessageHandlerExecutor, MessageTickerDef, Notifier};
use crate::error::SlateDBError;
use crate::manifest::store::ManifestStore;
use crate::manifest::ManifestCore;
use crate::merge_operator::MergeOperatorType;
use crate::sequenced_store::{WriteNotifier, MIN_WRITE_NOTICE_INTERVAL};
use crate::subcompaction::Subcompaction;
use crate::tablestore::TableStore;
use crate::utils::{format_bytes_si, IdGenerator};
#[cfg(feature = "compaction_filters")]
use crate::CompactionFilterSupplier;
use slatedb_common::clock::SystemClock;
use slatedb_common::metrics::MetricsRecorderHelper;
use slatedb_common::DbRand;

pub(crate) const COMPACTION_WORKER_TASK_NAME: &str = "compaction_worker";

#[derive(Debug)]
pub(crate) enum WorkerMessage {
    /// Signals that a compaction job has finished execution.
    CompactionJobFinished {
        /// Job id (distinct from the canonical compaction id).
        id: Ulid,
        /// Output SR on success, or the compaction error.
        result: Result<SortedRun, SlateDBError>,
    },
    /// Progress update from the [`CompactionExecutor`].
    CompactionJobProgress {
        /// The job id associated with this progress report.
        id: Ulid,
        /// The total number of bytes processed so far (estimate).
        bytes_processed: u64,
        /// The current compaction context, which may carry new output SSTs.
        ctx: CompactionContext,
    },
    /// Ticker-triggered message to poll `.compactions` for claimable jobs.
    PollCompactions,
    /// A component sharing this process's `.compactions` store, such as the embedded
    /// coordinator after scheduling a job, wrote version `id`.
    CompactionsWritten { id: u64 },
    /// Ticker-triggered message to refresh liveness for all jobs this worker
    /// currently owns.
    HeartbeatOwnedJobs,
}

/// Stateless executor of compaction jobs claimed from `.compactions`.
///
/// Build one with [`CompactionWorkerBuilder`] and drive its event loop with
/// [`CompactionWorker::run`]. Call [`CompactionWorker::stop`] to gracefully
/// release any in-flight claims.
pub struct CompactionWorker {
    task_executor: Arc<MessageHandlerExecutor>,
}

impl CompactionWorker {
    pub(crate) fn new(task_executor: Arc<MessageHandlerExecutor>) -> Self {
        Self { task_executor }
    }

    /// Runs the worker until cancellation or fatal error. The worker polls
    /// `.compactions` every [`CompactionWorkerOptions::compactions_poll_interval`],
    /// claims up to [`CompactionWorkerOptions::max_concurrent_compactions`] jobs,
    /// executes them, and writes `Compacted` back to `.compactions`.
    pub async fn run(&self) -> Result<(), crate::Error> {
        self.start()?;
        self.join().await
    }

    /// Starts the worker's event loop monitor on the current runtime.
    ///
    /// Callers that interleave shutdown with a cancellation signal should call
    /// this before racing [`CompactionWorker::join`] against that signal, so the
    /// task is registered before [`CompactionWorker::stop`] can run. Otherwise a
    /// cancellation that wins the race would invoke `stop` on a worker that was
    /// never started, silently dropping the unstarted event loop. See
    /// [`crate::admin::Admin::run_compaction_worker`].
    pub(crate) fn start(&self) -> Result<(), crate::Error> {
        self.task_executor.monitor_on(&Handle::current())?;
        Ok(())
    }

    /// Waits for the worker's event loop to finish.
    pub(crate) async fn join(&self) -> Result<(), crate::Error> {
        self.task_executor
            .join_task(COMPACTION_WORKER_TASK_NAME)
            .await
            .map_err(|e| e.into())
    }

    /// Gracefully stops the worker, resetting any compactions it claimed back
    /// to `Scheduled` so other workers can pick them up immediately.
    pub async fn stop(&self) -> Result<(), crate::Error> {
        self.task_executor
            .shutdown_task(COMPACTION_WORKER_TASK_NAME)
            .await
            .map_err(|e| e.into())
    }
}

/// Total output SSTs recorded across a subcompaction progress (RFC-0028).
fn total_output_ssts(subcompactions: &[Subcompaction]) -> usize {
    subcompactions.iter().map(|s| s.output_ssts().len()).sum()
}

/// Internal `MessageHandler` for the worker's event loop.
///
/// Reuses [`CompactorMessage`] so the embedded [`TokioCompactionExecutor`] can
/// report `CompactionJobFinished` on the same channel the dispatcher polls.
pub(crate) struct CompactionWorkerHandler {
    worker_id: String,
    options: Arc<CompactionWorkerOptions>,
    compactions_store: Arc<CompactionsStore>,
    manifest_store: Arc<ManifestStore>,
    executor: Arc<dyn CompactionExecutor + Send + Sync>,
    clock: Arc<dyn SystemClock>,
    /// Lazily-initialized handle for CAS reads/writes on `.compactions`. The
    /// coordinator creates the file on first run; the worker tolerates its
    /// absence on early ticks.
    stored: Option<StoredCompactions>,
    rand: Arc<DbRand>,
    fp_registry: Arc<FailPointRegistry>,
    /// Latest compaction context reported by the executor for each active
    /// job. Entry present iff the job is active. Buffered here and published
    /// to `.compactions` with the next heartbeat tick (see
    /// [`Self::heartbeat_owned_jobs`]); progress reports themselves never
    /// write.
    job_progress: BTreeMap<Ulid, Option<CompactionContext>>,
}

impl CompactionWorkerHandler {
    pub(crate) fn new(
        worker_id: String,
        options: Arc<CompactionWorkerOptions>,
        compactions_store: Arc<CompactionsStore>,
        manifest_store: Arc<ManifestStore>,
        executor: Arc<dyn CompactionExecutor + Send + Sync>,
        clock: Arc<dyn SystemClock>,
        rand: Arc<DbRand>,
        fp_registry: Arc<FailPointRegistry>,
    ) -> Self {
        Self {
            worker_id,
            options,
            compactions_store,
            manifest_store,
            executor,
            clock,
            stored: None,
            rand,
            fp_registry,
            job_progress: BTreeMap::new(),
        }
    }

    /// Builds the worker's [`CompactionWorkerHandler`] and the receiver that
    /// the handler reads completion messages from. Shared between the
    /// standalone `run()` path and the embedded-worker path in `Compactor::run`.
    pub(crate) fn build_worker_handler(
        manifest_store: Arc<ManifestStore>,
        compactions_store: Arc<CompactionsStore>,
        table_store: Arc<TableStore>,
        options: Arc<CompactionWorkerOptions>,
        worker_runtime: Handle,
        rand: Arc<DbRand>,
        stats: Arc<CompactionStats>,
        recorder: MetricsRecorderHelper,
        system_clock: Arc<dyn SystemClock>,
        fp_registry: Arc<FailPointRegistry>,
        merge_operator: Option<MergeOperatorType>,
        #[cfg(feature = "compaction_filters")] compaction_filter_supplier: Option<
            Arc<dyn CompactionFilterSupplier>,
        >,
    ) -> (
        CompactionWorkerHandler,
        async_channel::Receiver<WorkerMessage>,
    ) {
        let (tx, rx) = async_channel::unbounded::<WorkerMessage>();
        let worker_id = rand.rng().gen_ulid(system_clock.as_ref()).to_string();
        info!(
            "starting compaction worker [worker_id={}, max_concurrent_compactions={}, compactions_poll_interval={:?}]",
            worker_id,
            options.max_concurrent_compactions,
            options.compactions_poll_interval,
        );

        let worker_stats = WorkerStats::new(&recorder, &worker_id);
        let executor = Arc::new(TokioCompactionExecutor::new(
            TokioCompactionExecutorOptions {
                handle: worker_runtime.clone(),
                options: options.clone(),
                worker_tx: tx,
                table_store: table_store.clone(),
                rand: rand.clone(),
                stats: stats.clone(),
                worker_stats,
                clock: system_clock.clone(),
                manifest_store: manifest_store.clone(),
                merge_operator: merge_operator.clone(),
                #[cfg(feature = "compaction_filters")]
                compaction_filter_supplier: compaction_filter_supplier.clone(),
            },
        ));

        let handler = CompactionWorkerHandler::new(
            worker_id,
            options.clone(),
            compactions_store.clone(),
            manifest_store.clone(),
            executor,
            system_clock.clone(),
            rand.clone(),
            fp_registry,
        );
        (handler, rx)
    }

    const EXPECT_LOADED: &'static str = "ensure_loaded should have set stored compactions";

    /// Loads `.compactions` on first use; subsequent calls reuse the cached
    /// handle. Returns `Ok(false)` if the file does not yet exist (worker
    /// started before the coordinator).
    async fn ensure_loaded(&mut self) -> Result<bool, SlateDBError> {
        if self.stored.is_some() {
            return Ok(true);
        }
        match StoredCompactions::try_load(self.compactions_store.clone()).await? {
            Some(s) => {
                self.stored = Some(s);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn capacity(&self) -> usize {
        self.options
            .max_concurrent_compactions
            .saturating_sub(self.job_progress.len())
    }

    /// Scans `.compactions` for `Scheduled` entries without a worker, claims up
    /// to remaining capacity via CAS, then validates each claim against a
    /// manifest read *after* the claim and dispatches it to the executor.
    /// Claims that fail validation are released back to `Scheduled`.
    async fn poll_and_claim(&mut self) -> Result<(), SlateDBError> {
        let capacity = self.capacity();

        // CAS loop: read latest, identify candidates, attempt write.
        // Candidates are filtered on their spec alone here; validating the
        // spec's sources against the manifest happens after the claim
        // succeeds, since only a manifest read after the claim is guaranteed
        // to be consistent with the claimed compaction.
        let claimed = loop {
            let stored = self.stored.as_mut().expect(Self::EXPECT_LOADED);
            stored.refresh().await?;
            let mut dirty_compactions = stored.prepare_dirty()?;

            let mut to_claim: Vec<Compaction> = Vec::new();
            for c in dirty_compactions
                .value
                .iter_with_status(&[CompactionStatus::Scheduled])
                .filter(|c| c.worker().is_none())
            {
                if c.spec().is_drain() || c.spec().destination().is_none() {
                    // Drain specs are coordinator-local, and a tiered spec without
                    // a destination can never be executed; neither can become
                    // valid later, so skip them rather than claim and release.
                    warn!("skipping unrunnable compaction spec [id={}]", c.id());
                } else if self.job_progress.contains_key(&c.id()) {
                    // It's possible this worker tries to claim a job that it's already running.
                    // This can happen if coordinator hasn't seen this worker's heartbeat yet,
                    // and transitions the job back to `Scheduled`. We don't attempt to reclaim
                    // the job since its context might have diverged or this worker might be
                    // misbehaving. Instead, stop the local execution so a subsequent poll can
                    // claim only after local bookkeeping has been cleared.
                    debug!(
                        "skipping; this compaction is already running [worker_id={}, id={}]",
                        self.worker_id,
                        c.id()
                    );
                    Self::stop_compaction_job(&self.executor, &mut self.job_progress, c.id());
                } else if to_claim.len() < capacity {
                    to_claim.push(c.clone());
                }
            }
            if to_claim.is_empty() {
                debug!(
                    "No claimable compactions; skipping .compactions CAS write and executor dispatch [worker_id={}]",
                    self.worker_id
                );
                return Ok(());
            }

            let heartbeat_ms = self.clock.now().timestamp_millis() as u64;
            let worker_spec = WorkerSpec::new(self.worker_id.clone(), heartbeat_ms);

            for c in &to_claim {
                dirty_compactions.value.insert(
                    c.clone()
                        .with_status(CompactionStatus::Running)
                        .with_worker(Some(worker_spec.clone())),
                );
            }
            match stored.update(dirty_compactions).await {
                Ok(()) => break to_claim,
                Err(e) if e.is_sequenced_write_conflict() => {
                    debug!("claim conflict on .compactions; refreshing and retrying");
                    continue;
                }
                Err(e) => return Err(e),
            }
        };

        // Build job args against a manifest read *after* the claim CAS. The
        // coordinator writes the manifest before `.compactions` (see
        // `CompactorStateWriter::write_state_safely`), so this manifest is at
        // least as recent as the compactions state the claim landed on. A
        // manifest read before the claim could pair a stale manifest with a
        // newer spec whose source ids were recycled in the meantime (e.g. a
        // sorted run rebuilt with the same id), which the id-equality
        // validation in `build_job_args` cannot detect.
        let manifest = self.manifest_store.read_latest_manifest().await?;

        for compaction in claimed {
            match Self::build_job_args(&compaction, manifest.core(), &self.worker_id) {
                Ok(args) => {
                    info!(
                        "claimed compaction [worker_id={}, id={}]",
                        self.worker_id,
                        compaction.id()
                    );
                    self.job_progress
                        .insert(compaction.id(), compaction.ctx().cloned());
                    Self::dispatch_to_executor(&self.executor, args);
                }
                Err(e) => {
                    warn!(
                        "claimed compaction is invalid against the post-claim manifest; releasing claim [worker_id={}, id={}, error={:?}]",
                        self.worker_id,
                        compaction.id(),
                        e
                    );
                    self.release_claim(compaction.id()).await?;
                }
            }
        }
        Ok(())
    }

    /// Buffers the latest compaction context reported by the executor. The
    /// context is published to `.compactions` by the next heartbeat tick (see
    /// [`Self::heartbeat_owned_jobs`]); progress reports themselves never
    /// write, so the heartbeat ticker is the sole writer of worker progress.
    ///
    /// `bytes_processed` is *cumulative* per job (the running byte total), not
    /// a delta.
    fn record_progress(&mut self, id: Ulid, bytes_processed: u64, ctx: CompactionContext) {
        if let Some(latest) = self.job_progress.get_mut(&id) {
            debug!(
                "buffered progress [worker_id={}, id={}, bytes={}, output_ssts={}]",
                self.worker_id,
                id,
                format_bytes_si(bytes_processed),
                total_output_ssts(ctx.subcompactions())
            );
            *latest = Some(ctx);
        }
    }

    fn build_job_args(
        compaction: &Compaction,
        db_state: &ManifestCore,
        _worker_id: &str,
    ) -> Result<StartCompactionJobArgs, SlateDBError> {
        let destination = compaction
            .spec()
            .destination()
            .ok_or(SlateDBError::InvalidCompaction)?;
        let l0_sst_views = compaction.get_l0_sst_views(db_state);
        let sorted_runs = compaction.get_sorted_runs(db_state);

        // Reject drain specs (workers only execute tiered compactions; drain
        // is coordinator-local).
        if compaction.spec().is_drain() {
            return Err(SlateDBError::InvalidCompaction);
        }

        // Validate the spec's sources actually exist in the manifest. If they
        // don't, the spec was racing with a manifest write; release the
        // claim and let the coordinator reschedule.
        let expected_l0: Vec<Ulid> = compaction
            .spec()
            .sources()
            .iter()
            .filter_map(|s| s.maybe_unwrap_sst_view())
            .collect();
        let expected_srs: Vec<u32> = compaction
            .spec()
            .sources()
            .iter()
            .filter_map(|s| s.maybe_unwrap_sorted_run())
            .collect();
        let actual_l0: Vec<Ulid> = l0_sst_views.iter().map(|v| v.id).collect();
        let actual_srs: Vec<u32> = sorted_runs.iter().map(|sr| sr.id).collect();
        if actual_l0 != expected_l0 || actual_srs != expected_srs {
            return Err(SlateDBError::InvalidCompaction);
        }

        let is_dest_last_run = match db_state.tree_for_segment(compaction.spec().segment()) {
            Some(tree) => {
                tree.compacted.is_empty()
                    || tree.compacted.last().is_some_and(|sr| destination == sr.id)
            }
            None => false,
        };

        let complete_input_through = db_state.tree_for_segment(compaction.spec().segment())
            .filter(|tree| tree.l0.len() == l0_sst_views.len() && tree.compacted.len() == sorted_runs.len())
            .map(|_| db_state.last_l0_seq);

        Ok(StartCompactionJobArgs {
            // Use compaction_id as job id so completion messages line up with
            // the entry in `.compactions`. (One-job-per-Compaction in phase 2.)
            id: compaction.id(),
            compaction_id: compaction.id(),
            segment: compaction.spec().segment().clone(),
            destination,
            l0_sst_views,
            sorted_runs,
            compaction_clock_tick: db_state.last_l0_clock_tick,
            is_dest_last_run,
            complete_input_through,
            retention_min_seq: Some(db_state.recent_snapshot_min_seq),
            // Resume from the persisted compaction context (RFC-0028): a
            // reclaim preserves it, so feeding it back lets the executor continue
            // the same SST list and the progress it reports next still extends
            // the persisted one
            ctx: compaction.ctx().cloned(),
        })
    }

    fn dispatch_to_executor(
        executor: &Arc<dyn CompactionExecutor + Send + Sync>,
        args: StartCompactionJobArgs,
    ) {
        executor.start_compaction_job(args);
    }

    /// Refreshes liveness for every active job this worker still owns,
    /// publishing the latest compaction context buffered from executor
    /// progress reports along with the heartbeat, so a reclaiming worker can
    /// resume completed ranges. This is the only path that writes worker
    /// progress to `.compactions`.
    async fn heartbeat_owned_jobs(&mut self) -> Result<(), SlateDBError> {
        loop {
            let ids: Vec<Ulid> = self.job_progress.keys().copied().collect();
            if ids.is_empty() {
                return Ok(());
            }

            let stored = self.stored.as_mut().expect(Self::EXPECT_LOADED);
            stored.refresh().await?;
            let mut dirty = stored.prepare_dirty()?;
            let now_ms = self.clock.now().timestamp_millis() as u64;
            let worker_spec = WorkerSpec::new(self.worker_id.clone(), now_ms);
            let mut heartbeated = 0usize;
            let mut published_output_ssts = 0usize;

            for id in ids {
                let Some(existing) = dirty.value.get(&id).cloned() else {
                    debug!(
                        "heartbeat: compaction entry missing [worker_id={}, compaction_id={}]; stopping execution",
                        self.worker_id,
                        id
                    );
                    Self::stop_compaction_job(&self.executor, &mut self.job_progress, id);
                    continue;
                };
                if existing.worker().map(|w| w.worker_id.as_str()) != Some(self.worker_id.as_str())
                {
                    info!(
                        "heartbeat: lost ownership [worker_id={}, compaction_id={}]; stopping execution",
                        self.worker_id,
                        id
                    );
                    Self::stop_compaction_job(&self.executor, &mut self.job_progress, id);
                    continue;
                }

                let mut updated = existing.with_worker(Some(worker_spec.clone()));
                if let Some(ctx) = self.job_progress.get(&id).and_then(|ctx| ctx.clone()) {
                    published_output_ssts += total_output_ssts(ctx.subcompactions());
                    updated.set_ctx(Some(ctx));
                }
                dirty.value.insert(updated);
                heartbeated += 1;
            }

            if heartbeated == 0 {
                return Ok(());
            }

            match stored.update(dirty).await {
                Ok(()) => {
                    debug!(
                        "worker heartbeat refreshed owned compactions [worker_id={}, jobs={}, output_ssts={}, now_ms={}]",
                        self.worker_id, heartbeated, published_output_ssts, now_ms
                    );
                    if published_output_ssts > 0 {
                        let fp_registry = Arc::clone(&self.fp_registry);
                        let _ = &fp_registry;
                        fail_point!(fp_registry, "compactor-heartbeat-after-output-sst", |_| {
                            Err(SlateDBError::Fenced)
                        });
                    }
                    return Ok(());
                }
                Err(e) if e.is_sequenced_write_conflict() => continue,
                Err(e) => return Err(e),
            }
        }
    }

    /// Writes `Compacted` (with the produced output recorded on the job's
    /// subcompaction) for a successfully executed job. Only writes if the worker
    /// still owns the entry; otherwise it has been reclaimed and the produced
    /// SSTs become orphans (collected by GC).
    async fn write_compacted(
        &mut self,
        compaction_id: Ulid,
        sorted_run: SortedRun,
    ) -> Result<(), SlateDBError> {
        loop {
            let stored = self.stored.as_mut().expect(Self::EXPECT_LOADED);
            stored.refresh().await?;
            let mut dirty = stored.prepare_dirty()?;
            let Some(existing) = dirty.value.get(&compaction_id).cloned() else {
                info!(
                    "compaction entry missing on completion; dropping [id={}]",
                    compaction_id
                );
                return Ok(());
            };
            if existing.worker().map(|w| w.worker_id.as_str()) != Some(self.worker_id.as_str()) {
                info!(
                    "lost ownership before completion; dropping [id={}, status={:?}, owner={:?}]",
                    compaction_id,
                    existing.status(),
                    existing.worker().map(|w| &w.worker_id),
                );
                return Ok(());
            }
            let heartbeat_ms = self.clock.now().timestamp_millis() as u64;
            let updated = existing
                .with_status(CompactionStatus::Compacted)
                .with_output_ssts(
                    sorted_run
                        .sst_views()
                        .iter()
                        .map(|v| v.sst.clone())
                        .collect(),
                )
                .with_worker(Some(WorkerSpec::new(self.worker_id.clone(), heartbeat_ms)))
                .with_ctx(None);
            dirty.value.insert(updated);
            match stored.update(dirty).await {
                Ok(()) => return Ok(()),
                Err(e) if e.is_sequenced_write_conflict() => continue,
                Err(e) => return Err(e),
            }
        }
    }

    // Stops a compaction job that is currently executing on this worker, removing
    // it from the active jobs and progress bookkeeping. This is used when a
    // job is no longer owned by this worker (e.g., it was reclaimed by the
    // coordinator due to a heartbeat timeout). The function returns without waiting
    // for the executor to finish stopping the job.
    //
    // To release ownership of a job, use `release_claim` instead.
    fn stop_compaction_job(
        executor: &Arc<dyn CompactionExecutor + Send + Sync>,
        job_progress: &mut BTreeMap<Ulid, Option<CompactionContext>>,
        compaction_id: Ulid,
    ) {
        executor.stop_compaction_job(compaction_id);
        job_progress.remove(&compaction_id);
    }

    /// Returns a claim to `Scheduled` so it can be re-attempted by any worker
    /// (used when execution fails or when the worker shuts down gracefully).
    async fn release_claim(&mut self, compaction_id: Ulid) -> Result<(), SlateDBError> {
        let worker_id = self.worker_id.as_str();
        loop {
            let stored = self.stored.as_mut().expect(Self::EXPECT_LOADED);
            stored.refresh().await?;
            let mut dirty = stored.prepare_dirty()?;
            let Some(existing) = dirty.value.get(&compaction_id).cloned() else {
                info!(
                    "compaction no longer exists, no claim to release [worker_id]={} [compaction_id]={}",
                    worker_id,
                    compaction_id
                );
                return Ok(());
            };
            let compaction_owner = existing.worker().map(|w| w.worker_id.as_str());
            if compaction_owner != Some(worker_id) {
                info!(
                    "compaction is not owned by this worker, no claim to release [worker_id]={} [compaction_id]={} [owner]={:?}",
                    worker_id,
                    compaction_id,
                    compaction_owner
                );
                return Ok(());
            }
            let updated = existing
                .with_status(CompactionStatus::Scheduled)
                .with_worker(None);
            dirty.value.insert(updated);
            match stored.update(dirty).await {
                Ok(()) => return Ok(()),
                Err(e) if e.is_sequenced_write_conflict() => continue,
                Err(e) => return Err(e),
            }
        }
    }

    async fn handle_finished(
        &mut self,
        id: Ulid,
        result: Result<SortedRun, SlateDBError>,
    ) -> Result<(), SlateDBError> {
        self.job_progress.remove(&id);
        match result {
            Ok(sorted_run) => self.write_compacted(id, sorted_run).await?,
            Err(e) => {
                error!("compaction job failed [id={}, error={:?}]", id, e);
                self.release_claim(id).await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl MessageHandler<WorkerMessage> for CompactionWorkerHandler {
    fn tickers(&mut self) -> Vec<MessageTickerDef<WorkerMessage>> {
        // RFC-0025: spread `.compactions` polls across workers so they don't
        // synchronize on the same read cadence. Each poll waits a random
        // duration centered on `compactions_poll_interval` (the interval plus or
        // minus half), so the mean poll rate is unchanged.
        vec![
            MessageTickerDef::new(
                self.options.compactions_poll_interval,
                Box::new(|| WorkerMessage::PollCompactions),
            )
            .with_jitter(0.5, self.rand.clone()),
            MessageTickerDef::new(
                self.options.heartbeat_interval,
                Box::new(|| WorkerMessage::HeartbeatOwnedJobs),
            ),
        ]
    }

    fn notifiers(&mut self) -> Vec<Box<dyn Notifier<WorkerMessage>>> {
        // A job scheduled in this process is claimed without waiting for the next poll.
        vec![Box::new(WriteNotifier::new(
            self.compactions_store.subscribe_writes(),
            self.clock.clone(),
            self.options
                .compactions_poll_interval
                .min(MIN_WRITE_NOTICE_INTERVAL),
            |id| WorkerMessage::CompactionsWritten { id },
        ))]
    }

    async fn handle(&mut self, message: WorkerMessage) -> Result<(), SlateDBError> {
        if !self.ensure_loaded().await? {
            warn!(
                ".compactions does not exist yet; retrying on the next poll [worker_id={}]",
                self.worker_id
            );
            return Ok(());
        }
        match message {
            WorkerMessage::PollCompactions => {
                self.poll_and_claim().await?;
            }
            WorkerMessage::CompactionsWritten { id } => {
                // This worker's own claims and heartbeats are already in its view.
                let seen = self.stored.as_ref().expect(Self::EXPECT_LOADED).id();
                if id > seen {
                    self.poll_and_claim().await?;
                }
            }
            WorkerMessage::HeartbeatOwnedJobs => {
                self.heartbeat_owned_jobs().await?;
            }
            WorkerMessage::CompactionJobFinished { id, result } => {
                self.handle_finished(id, result).await?;
            }
            WorkerMessage::CompactionJobProgress {
                id,
                bytes_processed,
                ctx,
            } => {
                self.record_progress(id, bytes_processed, ctx);
            }
        }
        Ok(())
    }

    async fn cleanup(
        &mut self,
        _messages: BoxStream<'async_trait, WorkerMessage>,
        _result: Result<(), SlateDBError>,
    ) -> Result<(), SlateDBError> {
        // Stop accepting new work, then release any active claims so other
        // workers can pick them up immediately rather than waiting for the
        // heartbeat-timeout reclamation path. Stopping the executor also
        // decrements `running_compactions` for cancelled tasks.
        self.executor.stop();
        let claimed = std::mem::take(&mut self.job_progress);
        for id in claimed.into_keys() {
            if let Err(e) = self.release_claim(id).await {
                error!(
                    "failed to release claim on shutdown [worker_id={}, id={}, error={:?}]",
                    self.worker_id, id, e
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::block_cache_policy::BlockCachePolicy;
    use crate::bytes_range::BytesRange;
    use crate::compactor_state::{Compaction, CompactionSpec, SourceId};
    use crate::db_state::{SsTableHandle, SsTableId, SsTableInfo, SsTableView};
    use crate::format::sst::{SsTableFormat, SST_FORMAT_VERSION_LATEST};
    use crate::manifest::store::StoredManifest;
    use crate::manifest::ManifestCore;
    use crate::tablestore::{TableStore, TableStoreKind};
    use crate::test_utils::{build_sorted_runs, write_ssts, GatedObjectStore};
    use crate::types::RowEntry;
    use crate::utils::WatchableOnceCell;
    use bytes::Bytes;
    use futures::stream::StreamExt;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use parking_lot::Mutex;
    use slatedb_common::clock::DefaultSystemClock;
    use slatedb_common::MockSystemClock;

    const ROOT: &str = "/worker-test";

    /// Captures `start_compaction_job` calls without executing them, so the
    /// worker handler can be exercised without spinning up actual SST writers.
    struct NoopExecutor {
        jobs: Mutex<Vec<StartCompactionJobArgs>>,
        stopped_jobs: Mutex<Vec<Ulid>>,
    }

    impl NoopExecutor {
        fn new() -> Self {
            Self {
                jobs: Mutex::new(Vec::new()),
                stopped_jobs: Mutex::new(Vec::new()),
            }
        }

        fn jobs(&self) -> Vec<StartCompactionJobArgs> {
            self.jobs.lock().clone()
        }

        fn stopped_jobs(&self) -> Vec<Ulid> {
            self.stopped_jobs.lock().clone()
        }
    }

    impl CompactionExecutor for NoopExecutor {
        fn start_compaction_job(&self, args: StartCompactionJobArgs) {
            self.jobs.lock().push(args);
        }
        fn stop_compaction_job(&self, id: Ulid) -> bool {
            self.stopped_jobs.lock().push(id);
            true
        }
        fn stop(&self) {}
    }

    struct WorkerFixture {
        compactions_store: Arc<CompactionsStore>,
        executor: Arc<NoopExecutor>,
        handler: CompactionWorkerHandler,
        worker_id: String,
        // Holds an SsTableView so the test scope keeps it alive; reused as a
        // source for claimed compactions.
        l0_view: SsTableView,
    }

    impl WorkerFixture {
        async fn new(worker_id: &str) -> Self {
            let clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());
            Self::new_with_clock(worker_id, clock, CompactionWorkerOptions::default()).await
        }

        async fn new_with_clock(
            worker_id: &str,
            clock: Arc<dyn SystemClock>,
            options: CompactionWorkerOptions,
        ) -> Self {
            let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
            let path = Path::from(ROOT);
            let manifest_store = Arc::new(ManifestStore::new(&path, object_store.clone()));
            let compactions_store = Arc::new(CompactionsStore::new(&path, object_store.clone()));

            // Seed manifest with one L0 view so the worker can validate spec
            // sources during the claim flow. The unsegmented V1 manifest
            // wire format only persists the SST ULID for L0 entries (view
            // id == sst id on round-trip), so use `identity` here.
            let mut core = ManifestCore::new();
            let sst_ulid = Ulid::from_parts(1000, 0);
            let sst_info = SsTableInfo {
                first_entry: Some(Bytes::from_static(b"a")),
                ..SsTableInfo::default()
            };
            let l0_view = SsTableView::identity(SsTableHandle::new(
                SsTableId::from(sst_ulid),
                SST_FORMAT_VERSION_LATEST,
                sst_info,
            ));
            Arc::make_mut(&mut core.tree).l0.push_back(l0_view.clone());
            StoredManifest::create_new_db(manifest_store.clone(), core, clock.clone())
                .await
                .unwrap();

            // Coordinator normally creates `.compactions` on startup. Seed it
            // here for the worker.
            StoredCompactions::create(compactions_store.clone(), 0)
                .await
                .unwrap();

            let executor = Arc::new(NoopExecutor::new());
            let mut handler = CompactionWorkerHandler::new(
                worker_id.to_string(),
                Arc::new(options),
                compactions_store.clone(),
                manifest_store.clone(),
                executor.clone(),
                clock,
                Arc::new(DbRand::new(0)),
                Arc::new(FailPointRegistry::new()),
            );
            // `handle()` lazily loads `.compactions` on the first message; the
            // tests below drive the child fns (poll_and_claim, handle_finished,
            // cleanup) directly, so load it here to match that entry path.
            handler
                .ensure_loaded()
                .await
                .expect("compactions file seeded above");

            Self {
                compactions_store,
                executor,
                handler,
                worker_id: worker_id.to_string(),
                l0_view,
            }
        }

        /// Writes a single Scheduled compaction directly to `.compactions`,
        /// simulating one a coordinator would emit.
        async fn seed_scheduled_compaction(&self, id: Ulid, sources: Vec<SourceId>) {
            let spec = CompactionSpec::new(sources, 0);
            let compaction = Compaction::new(id, spec).with_status(CompactionStatus::Scheduled);
            let mut stored = StoredCompactions::try_load(self.compactions_store.clone())
                .await
                .unwrap()
                .expect("compactions file must exist");
            let mut dirty = stored.prepare_dirty().unwrap();
            dirty.value.insert(compaction);
            stored.update(dirty).await.unwrap();
        }

        async fn read_compaction(&self, id: Ulid) -> Option<Compaction> {
            let v = self
                .compactions_store
                .read_latest_compactions()
                .await
                .unwrap();
            v.compactions.get(&id).cloned()
        }
    }

    #[tokio::test]
    async fn test_worker_claims_scheduled_compaction() {
        let mut fx = WorkerFixture::new("worker-A").await;
        let id = Ulid::from_parts(1, 0);
        fx.seed_scheduled_compaction(id, vec![SourceId::SstView(fx.l0_view.id)])
            .await;

        fx.handler.poll_and_claim().await.unwrap();

        // The compaction should now be Running with this worker's id.
        let c = fx.read_compaction(id).await.expect("compaction missing");
        assert_eq!(c.status(), CompactionStatus::Running);
        let worker = c.worker().expect("worker spec missing");
        assert_eq!(worker.worker_id, fx.worker_id);
        assert!(worker.last_heartbeat_ms > 0);

        // The worker should have dispatched the job to its executor.
        let jobs = fx.executor.jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].compaction_id, id);

        // And the worker tracks the job locally.
        assert!(fx.handler.job_progress.contains_key(&id));
    }

    #[tokio::test]
    async fn test_worker_skips_compactions_owned_by_other_workers() {
        let mut fx = WorkerFixture::new("worker-A").await;
        // Pre-claim a compaction as "worker-B".
        let id = Ulid::from_parts(1, 0);
        let spec = CompactionSpec::new(vec![SourceId::SstView(fx.l0_view.id)], 0);
        let other = Compaction::new(id, spec)
            .with_status(CompactionStatus::Running)
            .with_worker(Some(WorkerSpec::new("worker-B".to_string(), 12345)));
        let mut stored = StoredCompactions::try_load(fx.compactions_store.clone())
            .await
            .unwrap()
            .unwrap();
        let mut dirty = stored.prepare_dirty().unwrap();
        dirty.value.insert(other);
        stored.update(dirty).await.unwrap();

        fx.handler.poll_and_claim().await.unwrap();

        // No claim should have been made; worker-B's entry is untouched.
        let c = fx.read_compaction(id).await.expect("compaction missing");
        assert_eq!(c.status(), CompactionStatus::Running);
        assert_eq!(c.worker().unwrap().worker_id, "worker-B");
        assert!(fx.executor.jobs().is_empty());
        assert!(fx.handler.job_progress.is_empty());
    }

    #[tokio::test]
    async fn test_worker_stops_rescheduled_job_already_active_locally_on_poll() {
        let mut fx = WorkerFixture::new_with_clock(
            "worker-A",
            Arc::new(DefaultSystemClock::new()),
            CompactionWorkerOptions {
                max_concurrent_compactions: 1,
                ..CompactionWorkerOptions::default()
            },
        )
        .await;
        let id = Ulid::from_parts(1, 0);
        fx.seed_scheduled_compaction(id, vec![SourceId::SstView(fx.l0_view.id)])
            .await;
        fx.handler.poll_and_claim().await.unwrap();
        assert_eq!(fx.executor.jobs().len(), 1);
        assert!(fx.handler.job_progress.contains_key(&id));

        // Simulate the coordinator reclaiming this worker's still-running job:
        // the persisted entry is Scheduled again while the local executor still
        // has it active. The worker should not re-claim it in the same poll,
        // but it should stop local execution and clear local bookkeeping.
        let mut stored = StoredCompactions::try_load(fx.compactions_store.clone())
            .await
            .unwrap()
            .unwrap();
        stored.refresh().await.unwrap();
        let mut dirty = stored.prepare_dirty().unwrap();
        let rescheduled = dirty
            .value
            .get(&id)
            .cloned()
            .expect("claimed compaction missing")
            .with_status(CompactionStatus::Scheduled)
            .with_worker(None);
        dirty.value.insert(rescheduled);
        stored.update(dirty).await.unwrap();

        fx.handler.poll_and_claim().await.unwrap();

        assert_eq!(
            fx.executor.jobs().len(),
            1,
            "worker must not dispatch a duplicate execution"
        );
        assert_eq!(fx.executor.stopped_jobs(), vec![id]);
        assert!(!fx.handler.job_progress.contains_key(&id));
        let c = fx.read_compaction(id).await.expect("compaction missing");
        assert_eq!(c.status(), CompactionStatus::Scheduled);
        assert!(c.worker().is_none());
    }

    #[tokio::test]
    async fn test_worker_stops_active_job_when_heartbeat_loses_ownership() {
        let mut fx = WorkerFixture::new_with_clock(
            "worker-A",
            Arc::new(DefaultSystemClock::new()),
            CompactionWorkerOptions {
                max_concurrent_compactions: 1,
                ..CompactionWorkerOptions::default()
            },
        )
        .await;
        let id = Ulid::from_parts(1, 0);
        fx.seed_scheduled_compaction(id, vec![SourceId::SstView(fx.l0_view.id)])
            .await;
        fx.handler.poll_and_claim().await.unwrap();
        assert!(fx.handler.job_progress.contains_key(&id));

        // Simulate another worker taking ownership before this worker's next
        // heartbeat. The heartbeat must not rewrite the entry; it should just
        // request local execution stop and clear local capacity bookkeeping.
        let mut stored = StoredCompactions::try_load(fx.compactions_store.clone())
            .await
            .unwrap()
            .unwrap();
        stored.refresh().await.unwrap();
        let mut dirty = stored.prepare_dirty().unwrap();
        let stolen = dirty
            .value
            .get(&id)
            .cloned()
            .expect("claimed compaction missing")
            .with_worker(Some(WorkerSpec::new("worker-B".to_string(), 12345)));
        dirty.value.insert(stolen);
        stored.update(dirty).await.unwrap();

        fx.handler.heartbeat_owned_jobs().await.unwrap();

        assert_eq!(fx.executor.stopped_jobs(), vec![id]);
        assert!(!fx.handler.job_progress.contains_key(&id));
        let c = fx.read_compaction(id).await.expect("compaction missing");
        assert_eq!(c.status(), CompactionStatus::Running);
        assert_eq!(c.worker().unwrap().worker_id, "worker-B");
        assert_eq!(c.worker().unwrap().last_heartbeat_ms, 12345);

        let next_id = Ulid::from_parts(2, 0);
        fx.seed_scheduled_compaction(next_id, vec![SourceId::SstView(fx.l0_view.id)])
            .await;
        fx.handler.poll_and_claim().await.unwrap();

        let jobs = fx.executor.jobs();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[1].compaction_id, next_id);
        assert!(fx.handler.job_progress.contains_key(&next_id));
    }

    #[tokio::test]
    async fn test_worker_writes_compacted_on_finish() {
        let mut fx = WorkerFixture::new("worker-A").await;
        let id = Ulid::from_parts(1, 0);
        fx.seed_scheduled_compaction(id, vec![SourceId::SstView(fx.l0_view.id)])
            .await;
        fx.handler.poll_and_claim().await.unwrap();

        // Build a synthetic sorted run the executor would have returned.
        let output_handle = SsTableHandle::new(
            SsTableId::from(Ulid::from_parts(9000, 0)),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo {
                first_entry: Some(Bytes::from_static(b"a")),
                ..SsTableInfo::default()
            },
        );
        let sorted_run = SortedRun::new(0, [SsTableView::identity(output_handle.clone())]);

        fx.handler
            .handle_finished(id, Ok(sorted_run))
            .await
            .unwrap();

        let c = fx.read_compaction(id).await.expect("compaction missing");
        assert_eq!(c.status(), CompactionStatus::Compacted);
        assert_eq!(c.output_ssts().len(), 1);
        assert_eq!(c.output_ssts()[0].id, output_handle.id);
        // worker_id is still attached (the coordinator clears it on commit).
        assert_eq!(c.worker().unwrap().worker_id, fx.worker_id);
        // Active set is drained on finish.
        assert!(!fx.handler.job_progress.contains_key(&id));
    }

    #[tokio::test]
    async fn test_worker_releases_claim_on_execution_failure() {
        let mut fx = WorkerFixture::new("worker-A").await;
        let id = Ulid::from_parts(1, 0);
        fx.seed_scheduled_compaction(id, vec![SourceId::SstView(fx.l0_view.id)])
            .await;
        fx.handler.poll_and_claim().await.unwrap();

        fx.handler
            .handle_finished(id, Err(SlateDBError::InvalidDBState))
            .await
            .unwrap();

        // On error the worker releases the claim so another worker can retry.
        let c = fx.read_compaction(id).await.expect("compaction missing");
        assert_eq!(c.status(), CompactionStatus::Scheduled);
        assert!(c.worker().is_none());
        assert!(!fx.handler.job_progress.contains_key(&id));
    }

    #[tokio::test]
    async fn test_worker_cleanup_releases_active_claims() {
        let mut fx = WorkerFixture::new("worker-A").await;
        let id = Ulid::from_parts(1, 0);
        fx.seed_scheduled_compaction(id, vec![SourceId::SstView(fx.l0_view.id)])
            .await;
        fx.handler.poll_and_claim().await.unwrap();
        assert_eq!(fx.handler.job_progress.len(), 1);

        // cleanup mirrors graceful shutdown.
        let empty: BoxStream<'_, WorkerMessage> = futures::stream::empty().boxed();
        fx.handler.cleanup(empty, Ok(())).await.unwrap();

        let c = fx.read_compaction(id).await.expect("compaction missing");
        assert_eq!(c.status(), CompactionStatus::Scheduled);
        assert!(c.worker().is_none());
        assert!(fx.handler.job_progress.is_empty());
    }

    #[test]
    fn test_worker_cleanup_releases_active_claims_in_id_order() {
        let id1 = Ulid::from_parts(1, 0);
        let id2 = Ulid::from_parts(2, 0);
        let id3 = Ulid::from_parts(3, 0);

        let job_progress: BTreeMap<Ulid, Option<CompactionContext>> =
            BTreeMap::from([(id3, None), (id1, None), (id2, None)]);
        let release_order = job_progress.into_keys().collect::<Vec<_>>();

        assert_eq!(release_order, vec![id1, id2, id3]);
    }

    #[tokio::test]
    async fn test_worker_skips_unrunnable_spec() {
        let mut fx = WorkerFixture::new("worker-A").await;
        let id = Ulid::from_parts(1, 0);
        // Source view ID that does not exist in the manifest — build_job_args
        // should reject this after the claim, releasing it back to Scheduled
        // for another worker to retry.
        let ghost = Ulid::from_parts(u64::MAX, 0);
        fx.seed_scheduled_compaction(id, vec![SourceId::SstView(ghost)])
            .await;

        fx.handler.poll_and_claim().await.unwrap();

        let c = fx.read_compaction(id).await.expect("compaction missing");
        assert_eq!(c.status(), CompactionStatus::Scheduled);
        assert!(c.worker().is_none());
        // No active job retained.
        assert!(fx.handler.job_progress.is_empty());
        // No job was dispatched to the executor either.
        assert!(fx.executor.jobs().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn test_worker_start_then_stop() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from(ROOT);
        let worker = crate::db::builder::CompactionWorkerBuilder::new(path, object_store)
            .build()
            .await
            .expect("failed to build compaction worker");

        // Mirrors the standalone-worker lifecycle: register the event loop with
        // `start`, then shut it down. `stop` must succeed against the task that
        // `start` registered, rather than silently no-op on an unstarted worker.
        worker.start().expect("failed to start compaction worker");
        worker
            .stop()
            .await
            .expect("failed to stop compaction worker");
    }

    /// Builds a throwaway output SST handle for tests.
    fn fake_output_handle(ulid: Ulid) -> SsTableHandle {
        SsTableHandle::new(
            SsTableId::from(ulid),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo {
                first_entry: Some(Bytes::from_static(b"a")),
                ..SsTableInfo::default()
            },
        )
    }

    #[tokio::test]
    async fn test_worker_heartbeat_ticker_refreshes_all_owned_jobs() {
        let mock_clock = Arc::new(MockSystemClock::new());
        mock_clock.set(1000);
        let options = CompactionWorkerOptions {
            max_concurrent_compactions: 2,
            heartbeat_interval: Duration::from_millis(1),
            ..CompactionWorkerOptions::default()
        };
        let clock: Arc<dyn SystemClock> = mock_clock.clone();
        let mut fx = WorkerFixture::new_with_clock("worker-hb-all", clock, options).await;

        let id1 = Ulid::from_parts(1, 0);
        let id2 = Ulid::from_parts(2, 0);
        fx.seed_scheduled_compaction(id1, vec![SourceId::SstView(fx.l0_view.id)])
            .await;
        fx.seed_scheduled_compaction(id2, vec![SourceId::SstView(fx.l0_view.id)])
            .await;
        fx.handler.poll_and_claim().await.unwrap();
        assert_eq!(fx.handler.job_progress.len(), 2);

        let sst = fake_output_handle(Ulid::from_parts(9000, 0));
        let ctx = CompactionContext::new(
            vec![Subcompaction::new(BytesRange::unbounded()).with_output_ssts(vec![sst])],
            Some(7),
        );
        fx.handler.record_progress(id1, 123, ctx.clone());
        let before_id1 = fx
            .read_compaction(id1)
            .await
            .expect("compaction missing")
            .worker()
            .expect("worker missing")
            .last_heartbeat_ms;
        let before_id2 = fx
            .read_compaction(id2)
            .await
            .expect("compaction missing")
            .worker()
            .expect("worker missing")
            .last_heartbeat_ms;

        mock_clock.advance(Duration::from_secs(5)).await;
        fx.handler.heartbeat_owned_jobs().await.unwrap();

        let c1 = fx.read_compaction(id1).await.expect("compaction missing");
        let c2 = fx.read_compaction(id2).await.expect("compaction missing");
        let hb1 = c1.worker().expect("worker missing").last_heartbeat_ms;
        let hb2 = c2.worker().expect("worker missing").last_heartbeat_ms;

        assert_eq!(c1.status(), CompactionStatus::Running);
        assert_eq!(c1.ctx(), Some(&ctx));
        assert!(c1.output_ssts().is_empty());
        assert!(hb1 > before_id1);
        assert!(hb2 > before_id2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_worker_heartbeat_ticker_refreshes_job_while_planning_reads_block() {
        // given: real compaction input SSTs behind a gated table store, while
        // manifest and `.compactions` reads/writes use the ungated inner store.
        let clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());
        let planning_heartbeat_sst_size = 512;
        let options = Arc::new(CompactionWorkerOptions {
            max_concurrent_compactions: 1,
            compactions_poll_interval: Duration::from_millis(5),
            heartbeat_interval: Duration::from_millis(5),
            max_sst_size: planning_heartbeat_sst_size,
            ..CompactionWorkerOptions::default()
        });
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let gated = Arc::new(GatedObjectStore::new(inner.clone()));
        let gated_store: Arc<dyn ObjectStore> = gated.clone();
        let root_path = Path::from("testdb-worker-planning-heartbeat");
        let table_store = Arc::new(TableStore::new(
            gated_store,
            SsTableFormat {
                block_size: 256,
                ..SsTableFormat::default()
            },
            root_path.clone(),
            None,
            TableStoreKind::Compactor,
            BlockCachePolicy::default(),
        ));
        let manifest_store = Arc::new(ManifestStore::new(&root_path, inner.clone()));
        let compactions_store = Arc::new(CompactionsStore::new(&root_path, inner.clone()));
        // Build a few L0 SSTs and a single sorted run to seed the manifest.
        let rows = |range: std::ops::Range<u64>,
                    step: usize,
                    value_prefix: &str,
                    seq_base: u64|
         -> Vec<RowEntry> {
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
        };
        let mut l0_sst_views = Vec::new();
        for entries in [
            rows(0..160, 2, "l0a", 10_000),
            rows(1..160, 2, "l0b", 20_000),
        ] {
            let ssts = write_ssts(&table_store, &entries, planning_heartbeat_sst_size).await;
            l0_sst_views.extend(ssts.into_iter().map(SsTableView::identity));
        }
        let sorted_runs = build_sorted_runs(
            &table_store,
            &[vec![rows(0..160, 1, "sr", 1)]],
            planning_heartbeat_sst_size,
        )
        .await;
        let mut core = ManifestCore::new();
        {
            let tree = Arc::make_mut(&mut core.tree);
            tree.l0.extend(l0_sst_views.clone());
            tree.compacted = sorted_runs.clone();
        }
        StoredManifest::create_new_db(manifest_store.clone(), core, clock.clone())
            .await
            .unwrap();
        StoredCompactions::create(compactions_store.clone(), 0)
            .await
            .unwrap();
        // Wire up the executor and handler.
        let (tx, rx) = async_channel::unbounded::<WorkerMessage>();
        let executor: Arc<dyn CompactionExecutor + Send + Sync> = Arc::new(
            TokioCompactionExecutor::new(TokioCompactionExecutorOptions {
                handle: Handle::current(),
                options: options.clone(),
                worker_tx: tx,
                table_store: table_store.clone(),
                rand: Arc::new(DbRand::new(100u64)),
                stats: {
                    let recorder = MetricsRecorderHelper::noop();
                    Arc::new(CompactionStats::new(&recorder))
                },
                worker_stats: WorkerStats::noop(),
                clock: clock.clone(),
                manifest_store: manifest_store.clone(),
                merge_operator: None,
                #[cfg(feature = "compaction_filters")]
                compaction_filter_supplier: None,
            }),
        );
        let handler = CompactionWorkerHandler::new(
            "worker-plan-hb".to_string(),
            options.clone(),
            compactions_store.clone(),
            manifest_store,
            executor.clone(),
            clock.clone(),
            Arc::new(DbRand::new(0)),
            Arc::new(FailPointRegistry::new()),
        );
        let id = Ulid::from_parts(42, 0);
        let sources = l0_sst_views
            .iter()
            .map(|sst| SourceId::SstView(sst.id))
            .chain(sorted_runs.iter().map(|sr| SourceId::SortedRun(sr.id)))
            .collect();
        let compaction = Compaction::new(id, CompactionSpec::new(sources, 0))
            .with_status(CompactionStatus::Scheduled);
        let mut stored = StoredCompactions::try_load(compactions_store.clone())
            .await
            .unwrap()
            .unwrap();
        let mut dirty = stored.prepare_dirty().unwrap();
        dirty.value.insert(compaction);
        stored.update(dirty).await.unwrap();

        let task_executor = Arc::new(MessageHandlerExecutor::new(
            Arc::new(WatchableOnceCell::new()),
            clock,
        ));
        task_executor
            .add_handler(
                COMPACTION_WORKER_TASK_NAME.to_string(),
                Box::new(handler),
                rx,
                &Handle::current(),
            )
            .unwrap();
        let worker = CompactionWorker::new(task_executor);

        // when: the worker's poll ticker claims the job and executor planning
        // blocks on the first SST index read.
        let setup_gets = gated.get_opts_gate.arrivals();
        gated.get_opts_gate.close();
        worker.start().unwrap();
        tokio::time::timeout(
            Duration::from_secs(30),
            gated.get_opts_gate.wait_for_arrivals(setup_gets + 1),
        )
        .await
        .expect("planning should reach the blocked read gate");

        let before = compactions_store
            .read_latest_compactions()
            .await
            .unwrap()
            .compactions
            .get(&id)
            .expect("compaction missing")
            .worker()
            .expect("worker missing")
            .last_heartbeat_ms;

        // then: the worker-level heartbeat ticker refreshes the persisted
        // liveness timestamp without requiring executor progress.
        let after = tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                let compaction = compactions_store
                    .read_latest_compactions()
                    .await
                    .unwrap()
                    .compactions
                    .get(&id)
                    .expect("compaction missing")
                    .clone();
                if compaction.worker().unwrap().last_heartbeat_ms > before {
                    break compaction;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("worker heartbeat ticker should refresh planning job");
        assert_eq!(after.status(), CompactionStatus::Running);
        assert_eq!(after.ctx(), None);
        assert!(after.worker().unwrap().last_heartbeat_ms > before);

        worker.stop().await.unwrap();
        gated.get_opts_gate.release();
    }

    #[tokio::test]
    async fn test_worker_heartbeat_ticker_stops_lost_jobs_and_refreshes_remaining_jobs() {
        let mock_clock = Arc::new(MockSystemClock::new());
        mock_clock.set(1000);
        let options = CompactionWorkerOptions {
            max_concurrent_compactions: 2,
            heartbeat_interval: Duration::from_millis(1),
            ..CompactionWorkerOptions::default()
        };
        let clock: Arc<dyn SystemClock> = mock_clock.clone();
        let mut fx = WorkerFixture::new_with_clock("worker-hb-lost", clock, options).await;

        let lost_id = Ulid::from_parts(1, 0);
        let kept_id = Ulid::from_parts(2, 0);
        fx.seed_scheduled_compaction(lost_id, vec![SourceId::SstView(fx.l0_view.id)])
            .await;
        fx.seed_scheduled_compaction(kept_id, vec![SourceId::SstView(fx.l0_view.id)])
            .await;
        fx.handler.poll_and_claim().await.unwrap();
        assert_eq!(fx.handler.job_progress.len(), 2);

        let mut stored = StoredCompactions::try_load(fx.compactions_store.clone())
            .await
            .unwrap()
            .unwrap();
        stored.refresh().await.unwrap();
        let mut dirty = stored.prepare_dirty().unwrap();
        let stolen = dirty
            .value
            .get(&lost_id)
            .cloned()
            .expect("claimed compaction missing")
            .with_worker(Some(WorkerSpec::new("worker-B".to_string(), 12345)));
        dirty.value.insert(stolen);
        stored.update(dirty).await.unwrap();

        mock_clock.advance(Duration::from_secs(5)).await;
        fx.handler.heartbeat_owned_jobs().await.unwrap();

        assert_eq!(fx.executor.stopped_jobs(), vec![lost_id]);
        assert!(!fx.handler.job_progress.contains_key(&lost_id));
        assert!(fx.handler.job_progress.contains_key(&kept_id));

        let lost = fx
            .read_compaction(lost_id)
            .await
            .expect("compaction missing");
        assert_eq!(lost.worker().unwrap().worker_id, "worker-B");
        assert_eq!(lost.worker().unwrap().last_heartbeat_ms, 12345);

        let kept = fx
            .read_compaction(kept_id)
            .await
            .expect("compaction missing");
        assert_eq!(kept.worker().unwrap().worker_id, fx.worker_id);
        assert!(kept.worker().unwrap().last_heartbeat_ms > 1000);
    }

    /// When the executor reports the initial planned context, the worker
    /// buffers it in memory and publishes it with the next heartbeat tick;
    /// the progress report itself must not write to `.compactions`.
    #[tokio::test]
    async fn test_worker_publishes_initial_compaction_context_on_heartbeat() {
        use tokio::time::pause;
        pause();

        // given: a claimed compaction with the clock advanced past the
        // heartbeat min-interval.
        let mock_clock = Arc::new(MockSystemClock::new());
        mock_clock.set(1000);
        let options = CompactionWorkerOptions {
            heartbeat_interval: Duration::from_millis(1),
            ..CompactionWorkerOptions::default()
        };
        let clock: Arc<dyn SystemClock> = mock_clock.clone();
        let mut fx = WorkerFixture::new_with_clock("worker-hb2", clock, options).await;
        let id = Ulid::from_parts(2, 0);
        fx.seed_scheduled_compaction(id, vec![SourceId::SstView(fx.l0_view.id)])
            .await;
        fx.handler.poll_and_claim().await.unwrap();
        mock_clock.advance(Duration::from_secs(2)).await;

        // when: progress reports the executor's first planned context, but no
        // output SSTs yet.
        let ctx =
            CompactionContext::new(vec![Subcompaction::new(BytesRange::unbounded())], Some(0));
        fx.handler.record_progress(id, 1000, ctx.clone());

        // then: the progress report alone writes nothing.
        let c = fx.read_compaction(id).await.expect("compaction missing");
        assert_eq!(c.ctx(), None);
        assert_eq!(
            c.worker().expect("worker spec missing").last_heartbeat_ms,
            1000
        );

        // when: the heartbeat ticker fires.
        fx.handler.heartbeat_owned_jobs().await.unwrap();

        // then: a heartbeat is written and the initial context is published,
        // but no per-range output progress exists yet.
        let c = fx.read_compaction(id).await.expect("compaction missing");
        assert_eq!(c.status(), CompactionStatus::Running);
        let worker = c.worker().expect("worker spec missing");
        assert!(
            worker.last_heartbeat_ms > 1000,
            "heartbeat_ms should have been updated; got {}",
            worker.last_heartbeat_ms
        );
        assert_eq!(c.ctx(), Some(&ctx));
        let ctx = c.ctx().unwrap();
        assert_eq!(
            0,
            ctx.subcompactions()
                .iter()
                .map(|s| s.output_ssts().len())
                .sum::<usize>()
        );
    }

    /// When the executor reports per-range subcompaction progress (RFC-0028),
    /// the heartbeat tick must publish it to `.compactions` so a reclaiming
    /// worker can resume. A later report that extends a range's output SSTs
    /// must also be published by the next heartbeat.
    #[tokio::test]
    async fn test_worker_publishes_subcompaction_progress_on_heartbeat() {
        use tokio::time::pause;
        pause();

        // given: a claimed compaction with buffered subcompaction progress.
        let mock_clock = Arc::new(MockSystemClock::new());
        mock_clock.set(1000);
        let options = CompactionWorkerOptions {
            ..CompactionWorkerOptions::default()
        };
        let clock: Arc<dyn SystemClock> = mock_clock.clone();
        let mut fx = WorkerFixture::new_with_clock("worker-subc", clock, options).await;
        let id = Ulid::from_parts(3, 0);
        fx.seed_scheduled_compaction(id, vec![SourceId::SstView(fx.l0_view.id)])
            .await;
        fx.handler.poll_and_claim().await.unwrap();
        mock_clock.advance(Duration::from_secs(5)).await;

        // when: progress carries subcompaction output and the heartbeat fires.
        let sst1 = fake_output_handle(Ulid::from_parts(9000, 0));
        let subcompactions =
            vec![Subcompaction::new(BytesRange::unbounded()).with_output_ssts(vec![sst1.clone()])];
        let ctx = CompactionContext::new(subcompactions.clone(), Some(0));
        fx.handler.record_progress(id, 123, ctx);
        fx.handler.heartbeat_owned_jobs().await.unwrap();

        // then: the progress is published and the worker heartbeat is refreshed.
        let c = fx.read_compaction(id).await.expect("compaction missing");
        assert_eq!(c.status(), CompactionStatus::Running);
        assert!(c.worker().expect("worker spec missing").last_heartbeat_ms > 1000);
        assert_eq!(c.subcompactions(), &subcompactions);

        // when: a later progress report extends the range's output SSTs and
        // another heartbeat fires.
        let sst2 = fake_output_handle(Ulid::from_parts(9001, 0));
        let extended =
            vec![Subcompaction::new(BytesRange::unbounded()).with_output_ssts(vec![sst1, sst2])];
        let extended_ctx = CompactionContext::new(extended.clone(), Some(0));
        fx.handler.record_progress(id, 456, extended_ctx);
        fx.handler.heartbeat_owned_jobs().await.unwrap();

        // then: the extended progress is also published (plan unchanged, so the
        // checked setter accepts it).
        let c = fx.read_compaction(id).await.expect("compaction missing");
        assert_eq!(c.subcompactions(), &extended);
    }

    /// A reclaimed compaction preserves its subcompaction progress, and a
    /// worker that re-claims it must resume the executor from the per-range
    /// output SSTs (RFC-0028) rather than restarting from scratch.
    #[tokio::test]
    async fn test_worker_resumes_from_subcompaction_progress() {
        // given: a reclaimed compaction (Scheduled, no worker) that still
        // carries per-range progress from a prior attempt.
        let mut fx = WorkerFixture::new("worker-resume").await;
        let id = Ulid::from_parts(4, 0);
        let sst1 = fake_output_handle(Ulid::from_parts(9100, 0));
        let spec = CompactionSpec::new(vec![SourceId::SstView(fx.l0_view.id)], 0);
        let compaction = Compaction::new(id, spec)
            .with_status(CompactionStatus::Scheduled)
            .with_ctx(Some(CompactionContext::new(
                vec![Subcompaction::new(BytesRange::unbounded())
                    .with_output_ssts(vec![sst1.clone()])],
                Some(0),
            )));
        let mut stored = StoredCompactions::try_load(fx.compactions_store.clone())
            .await
            .unwrap()
            .unwrap();
        let mut dirty = stored.prepare_dirty().unwrap();
        dirty.value.insert(compaction);
        stored.update(dirty).await.unwrap();

        // when: a worker claims it.
        fx.handler.poll_and_claim().await.unwrap();

        // then: the dispatched job resumes from the persisted subcompaction
        // output rather than starting from an empty list.
        let jobs = fx.executor.jobs();
        assert_eq!(jobs.len(), 1);
        let ctx = jobs[0].ctx.as_ref().expect("missing context");
        assert_eq!(ctx.subcompactions().len(), 1);
        assert_eq!(ctx.subcompactions()[0].output_ssts().len(), 1);
        assert_eq!(ctx.subcompactions()[0].output_ssts()[0].id, sst1.id);
    }
}

#[cfg(test)]
#[path = "../tests/unit/compaction_input_coverage.rs"]
mod compaction_input_coverage;
