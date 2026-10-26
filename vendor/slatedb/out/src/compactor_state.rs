use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use bytes::Bytes;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::db_state::{SortedRun, SsTableHandle, SsTableView};
use crate::error::SlateDBError;
use crate::manifest::{Manifest, ManifestCore};
use crate::subcompaction::Subcompaction;
use slatedb_txn_obj::DirtyObject;

/// Identifier for a compaction input source.
///
/// A `SourceId` distinguishes between two kinds of inputs a compaction can read:
/// an existing compacted sorted run (identified by its run id), or an L0 SSTable
/// view (identified by its view ID).
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceId {
    SortedRun(u32),
    SstView(Ulid),
}

impl Display for SourceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SourceId::SortedRun(id) => {
                    format!("{}", *id)
                }
                SourceId::SstView(_) => String::from("l0"),
            }
        )
    }
}

impl SourceId {
    /// Unwraps the source as a Sorted Run id, panicking if it is an L0 SST.
    ///
    /// ## Returns
    /// - The sorted run id.
    ///
    /// ## Panics
    /// - If called on `SourceId::SstView`.
    pub(crate) fn unwrap_sorted_run(&self) -> u32 {
        self.maybe_unwrap_sorted_run()
            .expect("tried to unwrap SstView as Sorted Run")
    }

    /// Returns the sorted run id if this source is a `SortedRun`, otherwise `None`.
    pub(crate) fn maybe_unwrap_sorted_run(&self) -> Option<u32> {
        match self {
            SourceId::SortedRun(id) => Some(*id),
            SourceId::SstView(_) => None,
        }
    }

    /// Returns the view ID if this source is an `SstView`, otherwise `None`.
    pub(crate) fn maybe_unwrap_sst_view(&self) -> Option<Ulid> {
        match self {
            SourceId::SortedRun(_) => None,
            SourceId::SstView(id) => Some(*id),
        }
    }
}

/// Immutable spec that describes a compaction. Two variants are supported
/// per RFC-0024:
///
/// - [`CompactionSpec::Tiered`] is the standard merge: read input sources,
///   write a single output sorted run with a destination id.
/// - [`CompactionSpec::DrainSegment`] retires a named segment without
///   merging — the compactor names the L0s and SRs it has observed, and the
///   commit advances the segment's watermark and clears its `compacted` list.
///
/// Every spec names exactly one segment (see RFC 24). For tiered specs an
/// empty `segment` targets the compatibility-encoded `prefix=""` segment
/// (root tree); a non-empty `segment` targets the named segment. Drain
/// specs require a non-empty `segment` — the empty-prefix segment cannot
/// be drained.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CompactionSpec {
    Tiered(TieredCompactionSpec),
    DrainSegment(DrainSegmentSpec),
}

/// Tiered compaction spec: read inputs, merge into one output sorted run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TieredCompactionSpec {
    /// Target segment prefix. Empty `Bytes` targets the `prefix=""` segment.
    segment: Bytes,
    /// Input sources for the compaction.
    sources: Vec<SourceId>,
    /// Destination sorted run id for the compaction output.
    destination: u32,
}

/// Drain-segment spec: retire a named segment as part of segment retention.
/// No merge is performed; on commit the segment's watermark advances to
/// cover the listed L0s and the listed sorted runs are removed from
/// `compacted`. The result is a "drain marker" that the writer prunes once
/// observed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrainSegmentSpec {
    /// Target segment prefix. Must be non-empty — the `prefix=""` segment
    /// (root tree) cannot be drained.
    segment: Bytes,
    /// L0 SSTs and sorted runs the compactor has observed in the segment
    /// and is draining.
    sources: Vec<SourceId>,
}

impl CompactionSpec {
    /// Creates a tiered compaction spec targeting the compatibility-encoded
    /// `prefix=""` segment (the root tree). For specs that target a named
    /// segment, use [`CompactionSpec::for_segment`]. For drain operations,
    /// use [`CompactionSpec::drain_segment`].
    ///
    /// ## Arguments
    /// - `sources`: Ordered list of sources (L0 SST ULIDs and/or existing SR ids).
    /// - `destination`: Sorted Run id for the compaction output.
    pub fn new(sources: Vec<SourceId>, destination: u32) -> Self {
        Self::for_segment(Bytes::new(), sources, destination)
    }

    /// Creates a tiered compaction spec targeting the named segment with the
    /// given prefix. An empty `segment` is equivalent to [`CompactionSpec::new`]
    /// and targets the compatibility-encoded `prefix=""` segment.
    ///
    /// ## Arguments
    /// - `segment`: Target segment prefix.
    /// - `sources`: Ordered list of sources (L0 SST ULIDs and/or existing SR ids).
    /// - `destination`: Sorted Run id for the compaction output.
    pub fn for_segment(segment: Bytes, sources: Vec<SourceId>, destination: u32) -> Self {
        CompactionSpec::Tiered(TieredCompactionSpec {
            segment,
            sources,
            destination,
        })
    }

    /// Creates a drain-segment spec targeting the named (non-empty-prefix)
    /// segment. `sources` lists the L0s and SRs the compactor has observed in
    /// the segment and is draining.
    pub fn drain_segment(segment: Bytes, sources: Vec<SourceId>) -> Self {
        CompactionSpec::DrainSegment(DrainSegmentSpec { segment, sources })
    }

    /// The target segment prefix. Empty bytes mean the compatibility-encoded
    /// `prefix=""` segment (only valid for tiered specs).
    pub fn segment(&self) -> &Bytes {
        match self {
            CompactionSpec::Tiered(s) => &s.segment,
            CompactionSpec::DrainSegment(s) => &s.segment,
        }
    }

    /// The sources (input SSTs and sorted runs) for this compaction.
    pub fn sources(&self) -> &[SourceId] {
        match self {
            CompactionSpec::Tiered(s) => &s.sources,
            CompactionSpec::DrainSegment(s) => &s.sources,
        }
    }

    /// The destination sorted run id this compaction will produce, or `None`
    /// for drain specs (which produce no new SR).
    pub fn destination(&self) -> Option<u32> {
        match self {
            CompactionSpec::Tiered(s) => Some(s.destination),
            CompactionSpec::DrainSegment(_) => None,
        }
    }

    /// Returns true if this is a [`CompactionSpec::DrainSegment`] spec.
    pub fn is_drain(&self) -> bool {
        matches!(self, CompactionSpec::DrainSegment(_))
    }

    /// Returns true if any of the compaction sources are L0 SST views.
    pub fn has_l0_sources(&self) -> bool {
        self.sources()
            .iter()
            .any(|s| matches!(s, SourceId::SstView(_)))
    }

    /// Returns true if any of the compaction sources are sorted runs.
    pub fn has_sr_sources(&self) -> bool {
        self.sources()
            .iter()
            .any(|s| matches!(s, SourceId::SortedRun(_)))
    }
}

impl Display for CompactionSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let displayed_sources: Vec<String> =
            self.sources().iter().map(|s| format!("{}", s)).collect();
        let segment = self.segment();
        match self {
            CompactionSpec::Tiered(spec) => {
                if segment.is_empty() {
                    write!(f, "{:?} -> SR({})", displayed_sources, spec.destination)
                } else {
                    write!(
                        f,
                        "[seg={:?}] {:?} -> SR({})",
                        segment, displayed_sources, spec.destination,
                    )
                }
            }
            CompactionSpec::DrainSegment(_) => {
                write!(f, "[seg={:?}] drain {:?}", segment, displayed_sources)
            }
        }
    }
}

/// Lifecycle status for a compaction.
///
/// State transitions:
/// ```text
/// Submitted --> Scheduled <-> Running --> Compacted --> Completed
///     |                          |           |
///     |                          v           |
///     +-----------------------> Failed <-----+
/// ```
///
/// `Completed` and `Failed` are terminal states. `Compacted` is the
/// distributed-compaction intermediate state where the worker has written its
/// final output SSTs but the coordinator has not yet committed the result to
/// the manifest. `Scheduled` is the coordinator's "ready for a worker to claim"
/// state: only the coordinator promotes `Submitted → Scheduled`, and only after
/// the spec has been validated against the current manifest. Workers exclusively
/// claim `Scheduled` entries — they never act on `Submitted` — which keeps the
/// coordinator the single gatekeeper for validation and ensures the coordinator
/// has the entry in local state before any worker can transition it onward.
/// See RFC-0025.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CompactionStatus {
    /// The compaction has been submitted but the coordinator has not yet
    /// validated it. May originate from the internal scheduler, an admin
    /// submission, or a reload of `.compactions`.
    Submitted,
    /// The coordinator has validated the spec against the current manifest
    /// and promoted it; the job is ready to be claimed by a worker. Only the
    /// coordinator writes this state.
    Scheduled,
    /// The compaction is currently running.
    Running,
    /// The worker finished execution and wrote its output SSTs; the
    /// coordinator has not yet committed the result to the manifest.
    Compacted,
    /// The compaction finished successfully and was committed to the manifest.
    Completed,
    /// The compaction failed. It might or might not have started before failure.
    Failed,
}

impl CompactionStatus {
    fn active(self) -> bool {
        matches!(
            self,
            CompactionStatus::Submitted
                | CompactionStatus::Scheduled
                | CompactionStatus::Running
                | CompactionStatus::Compacted
        )
    }

    /// Returns whether this compaction still consumes scheduler capacity.
    pub(crate) fn counts_against_max_concurrent(self) -> bool {
        matches!(
            self,
            CompactionStatus::Submitted | CompactionStatus::Scheduled | CompactionStatus::Running
        )
    }

    fn finished(self) -> bool {
        matches!(self, CompactionStatus::Completed | CompactionStatus::Failed)
    }

    /// State transitions made by CompactionWorker that should be accepted into local state
    /// during [`CompactorState::merge_remote_compactions`]
    ///
    /// For known active compactions (Occupied), accept these remote transitions:
    ///   - `Compacted`: the worker's signal that execution finished; the
    ///     coordinator must commit the output SSTs to the manifest.
    ///   - `Scheduled → Running`: a worker has claimed the job; adopt the
    ///     Running entry so the coordinator's local copy carries the worker
    ///     claim. Without this, write_compactions_safely() would overwrite
    ///     the claim with a stale Scheduled/no-worker copy, causing the
    ///     worker to discard its result and potentially re-run the job.
    ///   - `Running → Scheduled`: a worker released its claim (execution
    ///     failed, post-claim validation rejected the job, or graceful
    ///     shutdown). Adopt the release so the job can be re-claimed;
    ///     otherwise the coordinator's stale Running/claimed copy would be
    ///     written back and no worker would ever pick the job up again.
    ///   - `Running -> Running`: should accept heartbeat updates when heartbeats
    ///     land for a job that is already in local state
    ///
    /// Terminal local states are not overwritten by a remote `Compacted` entry.
    /// The coordinator writes the manifest before `.compactions`, so local
    /// `Completed`/`Failed` can be ahead of a stale persisted `Compacted` result.
    ///
    /// All other remote updates are ignored.
    fn should_adopt_state_transition(&self, updated_status: CompactionStatus) -> bool {
        match self {
            Self::Submitted => matches!(updated_status, Self::Compacted),
            Self::Scheduled => matches!(updated_status, Self::Running | Self::Compacted),
            Self::Running => matches!(
                updated_status,
                Self::Scheduled | Self::Running | Self::Compacted
            ),
            Self::Compacted => matches!(updated_status, Self::Compacted),
            Self::Completed | Self::Failed => false,
        }
    }
}

/// Identity and liveness for the worker currently executing a compaction.
///
/// Persisted in `.compactions` alongside the `Compaction` once a worker claims
/// it. A null `WorkerSpec` on a `Submitted` compaction means the job is
/// unclaimed and available to be picked up by the next polling worker.
///
/// See RFC-0025 for the claim and heartbeat protocols.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorkerSpec {
    /// The id of the worker that owns this compaction.
    pub worker_id: String,
    /// Wall-clock ms of the last progress write performed by the owning worker.
    /// Used by the coordinator to reclaim jobs whose owners have gone silent.
    pub last_heartbeat_ms: u64,
}

impl WorkerSpec {
    pub fn new(worker_id: String, last_heartbeat_ms: u64) -> Self {
        Self {
            worker_id,
            last_heartbeat_ms,
        }
    }
}

static EMPTY_SUBCOMPACTIONS: Vec<Subcompaction> = Vec::new();

#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct CompactionContext {
    /// Subcompactions partitioning this compaction into non-overlapping key
    /// ranges (RFC-0028). The produced output SSTs are recorded per range
    /// here; On completion the aggregate is recorded in [`Compaction::output_ssts`]. A
    /// compaction that runs as a single merge has one range spanning the full
    /// keyspace. Empty only before any progress has been recorded.
    subcompactions: Vec<Subcompaction>,

    retention_min_seq: Option<u64>,
}

impl CompactionContext {
    pub(crate) fn new(subcompactions: Vec<Subcompaction>, retention_min_seq: Option<u64>) -> Self {
        Self {
            subcompactions,
            retention_min_seq,
        }
    }

    pub(crate) fn set_output_ssts(
        &mut self,
        subcompaction: usize,
        output_ssts: Vec<SsTableHandle>,
    ) {
        self.subcompactions
            .get_mut(subcompaction)
            .expect("subcompaction index out of bounds")
            .set_output_ssts(output_ssts);
    }

    /// Returns the subcompactions of this compaction, if any.
    pub fn subcompactions(&self) -> &Vec<Subcompaction> {
        &self.subcompactions
    }

    pub(crate) fn retention_min_seq(&self) -> Option<u64> {
        self.retention_min_seq
    }

    pub(crate) fn validate_update(&self, updated: &Self) {
        assert_eq!(
            self.retention_min_seq, updated.retention_min_seq,
            "compaction retention_min_seq must not change once set"
        );
        assert_eq!(
            self.subcompactions.len(),
            updated.subcompactions.len(),
            "subcompaction plan must not change once set"
        );
        for (prev, next) in self
            .subcompactions
            .iter()
            .zip(updated.subcompactions.iter())
        {
            assert_eq!(
                prev.range(),
                next.range(),
                "subcompaction ranges are immutable once set"
            );
            assert!(
                next.output_ssts()
                    .starts_with(prev.output_ssts().as_slice()),
                "new subcompaction output SSTs must always extend previous output SSTs"
            );
        }
    }
}

/// Canonical, internal record of a compaction.
///
/// A compaction is the unit tracked by the compactor: it has a stable `id` (ULID) and a `spec`
/// (what to compact and where).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Compaction {
    /// Stable id (ULID) used to track this compaction across messages and attempts.
    id: Ulid,
    /// What to compact (sources) and where to write (destination).
    spec: CompactionSpec,
    /// Total number of bytes processed so far for this compaction.
    bytes_processed: u64,
    /// Current status for this compaction.
    ///
    /// This is tracked only in memory at the moment.
    status: CompactionStatus,
    /// The output of compaction. Only valid in Compacted state
    output_ssts: Vec<SsTableHandle>,
    /// The worker that has claimed this compaction. `None` means the
    /// compaction is unclaimed (only valid when `status == Submitted`).
    worker: Option<WorkerSpec>,
    ctx: Option<CompactionContext>,
}

impl Compaction {
    pub(crate) fn new(id: Ulid, spec: CompactionSpec) -> Self {
        Self {
            id,
            spec,
            bytes_processed: 0,
            status: CompactionStatus::Submitted,
            worker: None,
            output_ssts: vec![],
            ctx: None,
        }
    }

    pub(crate) fn with_status(mut self, status: CompactionStatus) -> Self {
        self.set_status(status);
        self
    }

    pub(crate) fn with_worker(mut self, worker: Option<WorkerSpec>) -> Self {
        self.worker = worker;
        self
    }

    pub(crate) fn with_ctx(mut self, ctx: Option<CompactionContext>) -> Self {
        self.ctx = ctx;
        self
    }

    pub(crate) fn with_output_ssts(mut self, output_ssts: Vec<SsTableHandle>) -> Self {
        self.output_ssts = output_ssts;
        self
    }

    /// Returns all sorted run sources for this compaction. Sources are looked
    /// up from the spec's target segment tree (root tree for an empty
    /// segment).
    ///
    /// ## Arguments
    /// - `db_state`: The current core DB state from the manifest.
    pub(crate) fn get_sorted_runs(&self, db_state: &ManifestCore) -> Vec<SortedRun> {
        let Some(tree) = db_state.tree_for_segment(self.spec.segment()) else {
            return Vec::new();
        };
        let srs_by_id: HashMap<u32, &SortedRun> =
            tree.compacted.iter().map(|sr| (sr.id, sr)).collect();

        self.spec
            .sources()
            .iter()
            .filter_map(|s| s.maybe_unwrap_sorted_run())
            .filter_map(|id| srs_by_id.get(&id).map(|t| (*t).clone()))
            .collect()
    }

    /// Returns all L0 SSTable sources for this compaction. Sources are looked
    /// up from the spec's target segment tree (root tree for an empty
    /// segment).
    ///
    /// ## Arguments
    /// - `db_state`: The current core DB state from the manifest.
    pub(crate) fn get_l0_sst_views(&self, db_state: &ManifestCore) -> Vec<SsTableView> {
        let Some(tree) = db_state.tree_for_segment(self.spec.segment()) else {
            return Vec::new();
        };
        let sst_views_by_id: HashMap<Ulid, &SsTableView> =
            tree.l0.iter().map(|view| (view.id, view)).collect();

        self.spec
            .sources()
            .iter()
            .filter_map(|s| s.maybe_unwrap_sst_view())
            .filter_map(|ulid| sst_views_by_id.get(&ulid).map(|t| (*t).clone()))
            .collect()
    }

    /// Builds the output run when all input SST views have disjoint effective
    /// key ranges. Reusing the views avoids reading or rewriting SST data.
    pub(crate) fn trivial_move_output(&self, db_state: &ManifestCore) -> Option<SortedRun> {
        let destination = self.spec.destination()?;
        let mut sst_views = self.get_l0_sst_views(db_state);
        sst_views.extend(
            self.get_sorted_runs(db_state)
                .iter()
                .flat_map(|sr| sr.sst_views().iter().cloned()),
        );
        sst_views.sort_by(|left, right| {
            left.compacted_effective_range()
                .comparable_start_bound()
                .cmp(&right.compacted_effective_range().comparable_start_bound())
        });

        (!sst_views.is_empty()
            && sst_views.windows(2).all(|pair| {
                pair[0]
                    .compacted_effective_range()
                    .intersect(pair[1].compacted_effective_range())
                    .is_none()
            }))
        .then_some(SortedRun::new(destination, sst_views))
    }

    /// The stable id (ULID) used to track this compaction across messages and attempts.
    pub fn id(&self) -> Ulid {
        self.id
    }

    /// Returns the immutable compaction spec describing inputs and destination.
    pub fn spec(&self) -> &CompactionSpec {
        &self.spec
    }

    /// Sets bytes processed so far for this compaction.
    // Consumed once the worker wires up progress/heartbeat emission in the
    // failure-detection follow-up.
    #[allow(dead_code)]
    pub(crate) fn set_bytes_processed(&mut self, bytes: u64) {
        self.bytes_processed = bytes;
    }

    /// Gets the bytes processed so far.
    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed
    }

    /// Returns the current status of this compaction.
    pub fn status(&self) -> CompactionStatus {
        self.status
    }

    /// Returns all output SSTs produced by this compaction.
    pub fn output_ssts(&self) -> Vec<SsTableHandle> {
        self.output_ssts.clone()
    }

    /// Returns the job context required to resume a compaction job
    pub fn ctx(&self) -> Option<&CompactionContext> {
        self.ctx.as_ref()
    }

    pub(crate) fn set_ctx(&mut self, updated: Option<CompactionContext>) {
        match (&self.ctx, updated) {
            (_, None) => self.ctx = None,
            (Some(existing), Some(updated)) => {
                existing.validate_update(&updated);
                self.ctx = Some(updated);
            }
            (None, Some(updated)) => self.ctx = Some(updated),
        }
    }

    pub(crate) fn clear_ctx(&mut self) {
        self.set_ctx(None);
    }

    /// Returns the subcompactions of this compaction, if any.
    pub fn subcompactions(&self) -> &Vec<Subcompaction> {
        self.ctx
            .as_ref()
            .map(CompactionContext::subcompactions)
            .unwrap_or(&EMPTY_SUBCOMPACTIONS)
    }

    /// Sets the current status of this compaction.
    pub(crate) fn set_status(&mut self, status: CompactionStatus) {
        self.status = status;
        if status == CompactionStatus::Failed {
            self.clear_ctx();
        }
    }

    /// Sets the current worker of this compaction.
    pub(crate) fn set_worker(&mut self, worker: Option<WorkerSpec>) {
        self.worker = worker;
    }

    /// Returns the worker that has claimed this compaction, if any.
    pub fn worker(&self) -> Option<&WorkerSpec> {
        self.worker.as_ref()
    }

    /// Returns true if the compaction is active (submitted or running).
    pub fn active(&self) -> bool {
        self.status.active()
    }
}

impl Display for Compaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.spec)?;
        if self.bytes_processed > 0 {
            let human_bytes_processed = crate::utils::format_bytes_si(self.bytes_processed);
            write!(f, " ({} processed)", human_bytes_processed)?;
        }
        // TODO: fix me by implementing Display for CompactionJob
        if !self.subcompactions().is_empty() {
            // Subcompactions carry no status, so report how many ranges have
            // produced output so far rather than a completion count.
            let with_output = self
                .subcompactions()
                .iter()
                .filter(|s| !s.output_ssts().is_empty())
                .count();
            write!(
                f,
                " [{}/{} subcompactions with output]",
                with_output,
                self.subcompactions().len()
            )?;
        }
        Ok(())
    }
}

/// Internal immutable in-memory view of a `.compactions` file.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct CompactionsCore {
    /// The set of recent compactions tracked by this compactor. These may
    /// be pending, in progress, or recently completed (either with success
    /// or failure).
    recent_compactions: BTreeMap<Ulid, Compaction>,
}

impl CompactionsCore {
    pub(crate) fn new() -> Self {
        Self {
            recent_compactions: BTreeMap::new(),
        }
    }

    pub(crate) fn with_compactions(mut self, compactions: BTreeMap<Ulid, Compaction>) -> Self {
        self.recent_compactions = compactions;
        self
    }

    /// Returns all compactions retained in this state. Persisted state contains all active
    /// compactions and the most recently finished one. Process-local state may also retain
    /// older terminal entries until its next successful write.
    pub(crate) fn recent_compactions(&self) -> impl Iterator<Item = &Compaction> {
        self.recent_compactions.values()
    }
}

/// A compactions snapshot paired with its version ID for monotonic ordering.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VersionedCompactions {
    /// The version ID of the compactions file.
    pub(crate) id: u64,
    /// The flattened compactions state at this version.
    #[serde(flatten)]
    pub(crate) compactions: Compactions,
}

impl VersionedCompactions {
    pub(crate) fn from_compactions(id: u64, compactions: Compactions) -> Self {
        Self { id, compactions }
    }

    /// Returns the compactions file version ID.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the compactor epoch recorded in this compactions snapshot.
    pub fn compactor_epoch(&self) -> u64 {
        self.compactions.compactor_epoch
    }

    /// Returns an iterator over the recent compactions tracked in this snapshot.
    pub fn recent_compactions(&self) -> impl Iterator<Item = &Compaction> {
        self.compactions.core.recent_compactions()
    }

    pub(crate) fn core(&self) -> &CompactionsCore {
        &self.compactions.core
    }
}

/// Container for compactions tracked by the compactor alongside its epoch.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Compactions {
    // The current compactor's epoch.
    pub(crate) compactor_epoch: u64,
    #[serde(flatten)]
    pub(crate) core: CompactionsCore,
}

impl Compactions {
    pub(crate) fn new(compactor_epoch: u64) -> Self {
        Self {
            compactor_epoch,
            core: CompactionsCore::new(),
        }
    }

    pub(crate) fn with_compactions(mut self, compactions: Vec<Compaction>) -> Self {
        let recent_compactions = compactions
            .into_iter()
            .map(|c| (c.id(), c))
            .collect::<BTreeMap<Ulid, Compaction>>();
        self.core.recent_compactions = recent_compactions;
        self
    }

    /// Inserts a new compaction to be tracked.
    pub(crate) fn insert(&mut self, compaction: Compaction) {
        self.core
            .recent_compactions
            .insert(compaction.id, compaction);
    }

    /// Returns the tracked compaction for the specified id, if any.
    pub(crate) fn get(&self, compaction_id: &Ulid) -> Option<&Compaction> {
        self.core.recent_compactions.get(compaction_id)
    }

    pub(crate) fn get_mut(&mut self, compaction_id: &Ulid) -> Option<&mut Compaction> {
        self.core.recent_compactions.get_mut(compaction_id)
    }

    /// Returns true if the specified compaction id is being tracked.
    #[cfg(test)]
    pub(crate) fn contains(&self, compaction_id: &Ulid) -> bool {
        self.core.recent_compactions.contains_key(compaction_id)
    }

    /// Returns an iterator over all tracked compactions.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Compaction> {
        self.core.recent_compactions.values()
    }

    /// Returns an iterator over mutable compactions.
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut Compaction> {
        self.core.recent_compactions.values_mut()
    }

    /// Returns an iterator over all active (submitted or running) compactions.
    pub(crate) fn iter_active(&self) -> impl Iterator<Item = &Compaction> {
        self.core.recent_compactions.values().filter(|c| c.active())
    }

    /// Returns an iterator over all compactions whose status is one of `statuses`.
    pub(crate) fn iter_with_status(
        &self,
        statuses: &[CompactionStatus],
    ) -> impl Iterator<Item = &Compaction> {
        let statuses = statuses.to_vec();
        self.core
            .recent_compactions
            .values()
            .filter(move |c| statuses.contains(&c.status()))
    }

    /// Keeps the most recently finished compaction and any active compactions, and removes others.
    pub(crate) fn retain_active_and_last_finished(&mut self) {
        let latest_finished = self
            .core
            .recent_compactions
            .iter()
            .filter_map(|(_, c)| {
                if c.status().finished() {
                    Some(c.id())
                } else {
                    None
                }
            })
            .max();

        self.core
            .recent_compactions
            .retain(|id, compaction| compaction.active() || Some(id) == latest_finished.as_ref());
    }
}

/// Process-local runtime state owned by the compactor.
///
/// This is the in-memory view that a single compactor task uses to:
/// - keep a fresh `DirtyManifest` (view of `CoreDbState`),
/// - track in-flight compactions by id (ULID), and
/// - retain terminal tombstones until their `.compactions` write succeeds.
pub struct CompactorState {
    manifest: DirtyObject<Manifest>,
    compactions: DirtyObject<Compactions>,
}

impl CompactorState {
    /// Creates a new compactor state seeded with the provided dirty manifest and compactions.
    pub(crate) fn new(
        manifest: DirtyObject<Manifest>,
        compactions: DirtyObject<Compactions>,
    ) -> Self {
        assert_eq!(
            manifest.value.compactor_epoch,
            compactions.value.compactor_epoch
        );
        Self {
            manifest,
            compactions,
        }
    }

    /// Returns the current in-memory core DB state derived from the manifest.
    pub(crate) fn db_state(&self) -> &ManifestCore {
        &self.manifest.value.core
    }

    /// Returns the local dirty manifest that will be written back after compactions.
    pub(crate) fn manifest(&self) -> &DirtyObject<Manifest> {
        &self.manifest
    }

    /// Test-only mutable access to the local dirty manifest. Production
    /// callers should use [`Self::merge_remote_manifest`] or one of the
    /// `finish_*` methods so the writer/compactor merge protocol stays
    /// consistent.
    #[cfg(test)]
    pub(crate) fn manifest_mut_for_test(&mut self) -> &mut DirtyObject<Manifest> {
        &mut self.manifest
    }

    /// Test-only injection of a Submitted compaction into the in-memory
    /// state. Production callers route through [`Self::add_compaction`]
    /// (validation + dedup); this helper bypasses both for tests that want
    /// to set up specific scheduling scenarios.
    #[cfg(test)]
    pub(crate) fn insert_compaction_for_test(&mut self, compaction: Compaction) {
        self.compactions
            .value
            .core
            .recent_compactions
            .insert(compaction.id(), compaction);
    }

    /// Returns an iterator over all in-flight compactions.
    pub(crate) fn active_compactions(&self) -> impl Iterator<Item = &Compaction> {
        self.compactions.value.iter_active()
    }

    pub(crate) fn compactions_with_status(
        &self,
        statuses: &[CompactionStatus],
    ) -> impl Iterator<Item = &Compaction> {
        self.compactions.value.iter_with_status(statuses)
    }

    /// Returns the dirty compactions tracked by this state.
    pub(crate) fn compactions(&self) -> &DirtyObject<Compactions> {
        &self.compactions
    }

    /// Replaces the tracked dirty compactions with the provided value.
    pub(crate) fn set_compactions(&mut self, compactions: DirtyObject<Compactions>) {
        assert_eq!(
            self.manifest.value.compactor_epoch,
            compactions.value.compactor_epoch
        );
        self.compactions = compactions;
    }

    /// Merges the remote compactions view into the compactor's local state.
    ///
    /// This preserves local in-flight compactions while pulling in newly submitted remote
    /// compactions (e.g. external scheduling requests).
    pub(crate) fn merge_remote_compactions(
        &mut self,
        mut remote_compactions: DirtyObject<Compactions>,
    ) {
        let mut merged = BTreeMap::new();

        for compaction in self.compactions.value.iter() {
            merged.insert(compaction.id(), compaction.clone());
        }

        // For compactions not in local state (Vacant), accept new submissions,
        // worker-completed results, and retained terminal entries. On a write
        // conflict, the retry path reloads and merges the persisted `.compactions`
        // object before writing again. Process-local terminal entries are deliberately
        // retained until that write succeeds, so an older remote state for the same id
        // is treated as a stale transition rather than resurrected as active work.
        // Insert genuinely absent entries and let the commit path resolve stale
        // Compacted entries via `validate_compaction`.
        //
        // Scheduled/Running are different: they carry no finished output and require prior
        // coordinator ownership, so seeing them absent from local state remains anomalous.
        for compaction in remote_compactions.value.iter() {
            match merged.entry(compaction.id()) {
                Entry::Vacant(v) => match compaction.status() {
                    CompactionStatus::Submitted
                    | CompactionStatus::Compacted
                    | CompactionStatus::Completed
                    | CompactionStatus::Failed => {
                        v.insert(compaction.clone());
                    }
                    CompactionStatus::Scheduled | CompactionStatus::Running => {
                        error!(
                            "skipping remote active compaction absent from local state [compaction={:?}]",
                            compaction,
                        );
                    }
                },
                Entry::Occupied(mut o) => {
                    if o.get()
                        .status
                        .should_adopt_state_transition(compaction.status)
                    {
                        o.insert(compaction.clone());
                    } else {
                        debug!(
                            "ignoring remote compaction update [id={}, local_status={:?}, remote_status={:?}]",
                            compaction.id(),
                            o.get().status(),
                            compaction.status(),
                        );
                    }
                }
            }
        }

        let merged_compactions = Compactions {
            compactor_epoch: self.compactions.value.compactor_epoch,
            core: CompactionsCore::new().with_compactions(merged),
        };
        remote_compactions.value = merged_compactions;
        self.set_compactions(remote_compactions);
    }

    /// Merges the remote (writer) manifest view into the compactor's local state.
    ///
    /// This preserves local knowledge about compactions already applied (e.g., L0 last
    /// compacted marker, existing compacted runs) while pulling in newly created L0 SSTs
    /// and other writer-updated fields.
    pub(crate) fn merge_remote_manifest(&mut self, mut remote_manifest: DirtyObject<Manifest>) {
        let my_db_state = self.db_state();
        let remote = &remote_manifest.value.core;
        let tree = my_db_state.tree.merge_from_writer(&remote.tree);
        let segments =
            crate::manifest::merge_segments_from_writer(&my_db_state.segments, &remote.segments);
        // Segment configuration is stable; the writer is the source of truth.
        let merged = ManifestCore {
            initialized: remote_manifest.value.core.initialized,
            tree: Arc::new(tree),
            segments,
            segment_extractor_name: remote_manifest.value.core.segment_extractor_name.clone(),
            next_wal_sst_id: remote_manifest.value.core.next_wal_sst_id,
            replay_after_wal_id: remote_manifest.value.core.replay_after_wal_id,
            last_l0_clock_tick: remote_manifest.value.core.last_l0_clock_tick,
            last_l0_seq: remote_manifest.value.core.last_l0_seq,
            checkpoints: remote_manifest.value.core.checkpoints.clone(),
            wal_object_store_uri: my_db_state.wal_object_store_uri.clone(),
            recent_snapshot_min_seq: remote_manifest.value.core.recent_snapshot_min_seq,
            sequence_tracker: remote_manifest.value.core.sequence_tracker,
        };
        remote_manifest.value.core = merged;
        remote_manifest.value.prune_external_sst_ids();
        self.manifest = remote_manifest;
    }

    /// Registers a newly proposed compaction in the compactor's local state.
    ///
    /// Only enforces invariants about *conflicts across in-flight compactions* — checks
    /// that depend on the set of other active compactions, not on the spec in isolation.
    /// Spec-against-current-state validity (sources exist in the target tree, drain doesn't
    /// target the root segment, SR id overwrites, etc.) is enforced later by
    /// [`CompactorEventHandler::validate_compaction`], which runs on every Submitted spec
    /// regardless of origin (internal scheduler, admin submission, reloaded `.compactions`).
    ///
    /// ## Returns
    /// - `Ok(())` if accepted, or [`SlateDBError::InvalidCompaction`] if the compaction
    ///   conflicts with another active compaction.
    pub(crate) fn add_compaction(&mut self, compaction: Compaction) -> Result<(), SlateDBError> {
        let spec = compaction.spec();
        // At most one active drain per segment. Drain specs have no destination, so the
        // destination-collision check below cannot catch duplicates; an explicit
        // per-segment guard does.
        if spec.is_drain()
            && self
                .compactions
                .value
                .iter_active()
                .any(|c| c.spec().is_drain() && c.spec().segment() == spec.segment())
        {
            return Err(SlateDBError::InvalidCompaction);
        }
        // SR ids are globally unique across all segment trees (RFC-0024), so destination
        // collisions are checked across every active compaction regardless of target
        // segment. Drain specs produce no output SR, so this only applies to tiered specs.
        if let Some(dst) = spec.destination() {
            if self
                .compactions
                .value
                .iter_active()
                .filter_map(|c| c.spec().destination())
                .any(|d| d == dst)
            {
                return Err(SlateDBError::InvalidCompaction);
            }
        }
        info!("accepted submitted compaction [compaction={}]", compaction);

        self.compactions
            .value
            .core
            .recent_compactions
            .insert(compaction.id(), compaction);
        Ok(())
    }

    /// Mutates a compaction in place if it exists.
    ///
    /// Terminal entries remain in process-local state until the next successful
    /// `.compactions` write. They act as tombstones during conflict retries so
    /// an older persisted `Submitted` or `Compacted` record cannot be resurrected.
    /// The persisted value is still trimmed by
    /// [`crate::compactor_state_protocols::CompactorStateWriter`].
    pub(crate) fn update_compaction<F>(&mut self, compaction_id: &Ulid, f: F)
    where
        F: FnOnce(&mut Compaction),
    {
        if let Some(compaction) = self
            .compactions
            .value
            .core
            .recent_compactions
            .get_mut(compaction_id)
        {
            f(compaction);
        }
    }

    /// Applies the effects of a finished compaction to the in-memory manifest.
    ///
    /// This removes compacted L0 SSTs and source SRs from the spec's target segment tree
    /// (root tree for an empty segment), inserts the output SR in id-descending order,
    /// updates `last_compacted_l0_*`, and marks the compaction finished (retaining the most
    /// recent finished compaction for GC; see #1044).
    pub(crate) fn finish_compaction(&mut self, compaction_id: Ulid, output_sr: SortedRun) {
        let mut db_state = self.db_state().clone();
        if let Some(compaction) = self.compactions.value.get_mut(&compaction_id) {
            let spec = compaction.spec();
            info!("finished compaction [spec={}]", spec);
            // Tiered finish path; drain specs are completed via
            // `finish_drain_compaction` and never reach here.
            let dst = spec
                .destination()
                .expect("finish_compaction with output SR called on a non-tiered spec");
            let segment = spec.segment().clone();
            // reconstruct l0
            let compaction_l0s: HashSet<Ulid> = spec
                .sources()
                .iter()
                .filter_map(|id| id.maybe_unwrap_sst_view())
                .collect();
            let compaction_srs: HashSet<u32> = spec
                .sources()
                .iter()
                .chain(std::iter::once(&SourceId::SortedRun(dst)))
                .filter_map(|id| id.maybe_unwrap_sorted_run())
                .collect();
            let first_source = *spec
                .sources()
                .first()
                .expect("illegal: empty compaction spec");

            let Some(tree) = db_state.tree_for_segment_mut(&segment) else {
                error!(
                    "finish_compaction: target segment missing [segment={:?}, compaction_id={}]",
                    segment, compaction_id
                );
                // Segment was dropped between submission and finish; the
                // output is discarded. Mark the compaction terminal so it
                // doesn't linger as Active.
                self.update_compaction(&compaction_id, |c| {
                    c.set_status(CompactionStatus::Failed);
                });
                return;
            };

            let new_l0: VecDeque<SsTableView> = tree
                .l0
                .iter()
                .filter(|l0| !compaction_l0s.contains(&l0.id))
                .cloned()
                .collect();
            let mut new_compacted = Vec::new();
            let mut inserted = false;
            for compacted in tree.compacted.iter() {
                if !inserted && output_sr.id >= compacted.id {
                    new_compacted.push(output_sr.clone());
                    inserted = true;
                }
                if !compaction_srs.contains(&compacted.id) {
                    new_compacted.push(compacted.clone());
                }
            }
            if !inserted {
                new_compacted.push(output_sr);
            }
            Self::assert_compacted_srs_in_id_order(&new_compacted);
            if let Some(view_id) = first_source.maybe_unwrap_sst_view() {
                // if there are l0s, the newest must be the first entry in sources.
                // TODO: validate that this is the case
                tree.last_compacted_l0_sst_view_id = Some(view_id);
                // Resolve the SST ID from the view before it's removed from l0.
                tree.last_compacted_l0_sst_id = tree
                    .l0
                    .iter()
                    .find(|v| v.id == view_id)
                    .map(|v| v.sst.id.value());
            }
            tree.l0 = new_l0;
            tree.compacted = new_compacted;
            self.manifest.value.core = db_state;
            self.manifest.value.prune_external_sst_ids();
            self.update_compaction(&compaction_id, |c| {
                c.set_status(CompactionStatus::Completed);
            });
        } else {
            error!("compaction not found [compaction_id={}]", compaction_id);
        }
    }

    /// Applies the effects of a finished drain compaction to the in-memory
    /// manifest (RFC-0024). Unlike [`Self::finish_compaction`], drain produces
    /// no output sorted run — it only:
    ///
    /// - advances the target segment's `last_compacted_l0_*` watermark to the
    ///   newest L0 named in the spec's sources, and
    /// - removes the named sorted runs from `compacted`.
    ///
    /// If every observed L0 and SR is named in `sources`, the result is a
    /// "drain marker" (`l0=[], compacted=[], watermark=set`) that the writer
    /// will prune through the merge protocol on its next commit. If the
    /// segment received late writes (L0s with ids above the watermark) the
    /// segment stays Live with those L0s remaining, and the next propose()
    /// pass decides whether to drain them.
    pub(crate) fn finish_drain_compaction(&mut self, compaction_id: Ulid) {
        let mut db_state = self.db_state().clone();
        let Some(compaction) = self.compactions.value.get(&compaction_id) else {
            error!("compaction not found [compaction_id={}]", compaction_id);
            return;
        };
        let spec = compaction.spec();
        info!("finished drain compaction [spec={}]", spec);
        let drain = match spec {
            CompactionSpec::DrainSegment(d) => d.clone(),
            CompactionSpec::Tiered(_) => {
                #[allow(clippy::panic)]
                {
                    panic!(
                        "finish_drain_compaction called on a tiered spec [compaction_id={}]",
                        compaction_id
                    );
                }
            }
        };
        let segment = drain.segment.clone();
        let drained_l0_ids: HashSet<Ulid> = drain
            .sources
            .iter()
            .filter_map(|id| id.maybe_unwrap_sst_view())
            .collect();
        let drained_sr_ids: HashSet<u32> = drain
            .sources
            .iter()
            .filter_map(|id| id.maybe_unwrap_sorted_run())
            .collect();

        let Some(tree) = db_state.tree_for_segment_mut(&segment) else {
            error!(
                "finish_drain_compaction: target segment missing [segment={:?}, compaction_id={}]",
                segment, compaction_id
            );
            // Segment was dropped between submission and finish; mark the
            // compaction terminal so it doesn't linger as Active.
            self.update_compaction(&compaction_id, |c| {
                c.set_status(CompactionStatus::Failed);
            });
            return;
        };

        // tree.l0 is sorted newest-first, so the first match is the newest L0
        // in `sources`. Per RFC-0024 the watermark advances to that L0 only;
        // late L0s with ids above it survive the writer's later merge.
        if let Some(view) = tree
            .l0
            .iter()
            .find(|view| drained_l0_ids.contains(&view.id))
            .cloned()
        {
            tree.last_compacted_l0_sst_view_id = Some(view.id);
            tree.last_compacted_l0_sst_id = Some(view.sst.id.value());
        }

        tree.l0.retain(|view| !drained_l0_ids.contains(&view.id));
        tree.compacted.retain(|sr| !drained_sr_ids.contains(&sr.id));

        self.manifest.value.core = db_state;
        self.manifest.value.prune_external_sst_ids();
        self.update_compaction(&compaction_id, |c| {
            c.set_status(CompactionStatus::Completed);
        });
    }

    /// Debug assertion that compacted sorted runs are kept in strictly descending id order.
    fn assert_compacted_srs_in_id_order(compacted: &[SortedRun]) {
        let mut last_sr_id = u32::MAX;
        for sr in compacted.iter() {
            assert!(sr.id < last_sr_id);
            last_sr_id = sr.id;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread::sleep;
    use std::time::Duration;

    use super::*;
    use crate::bytes_range::BytesRange;
    use crate::checkpoint::Checkpoint;
    use crate::compactor_state::SourceId::SstView;
    use crate::config::{FlushOptions, FlushType, Settings};
    use crate::db::Db;
    use crate::db_state::{SsTableId, SsTableInfo};
    use crate::format::sst::SST_FORMAT_VERSION_LATEST;
    use crate::manifest::store::test_utils::new_dirty_manifest;
    use crate::manifest::store::{ManifestStore, StoredManifest};
    use crate::manifest::{LsmTreeState, Segment};
    use crate::test_utils::bounded_sst_view;
    use crate::utils::IdGenerator;
    use bytes::Bytes;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use slatedb_common::clock::{DefaultSystemClock, SystemClock};
    use slatedb_common::DbRand;
    use slatedb_txn_obj::test_utils::new_dirty_object;
    use tokio::runtime::{Handle, Runtime};

    const PATH: &str = "/test/db";

    fn test_subcompaction_sst(first_key: &[u8]) -> SsTableHandle {
        SsTableHandle::new(
            SsTableId::from(Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo {
                first_entry: Some(Bytes::copy_from_slice(first_key)),
                ..SsTableInfo::default()
            },
        )
    }

    fn test_compaction_with_subcompactions(subcompactions: Vec<Subcompaction>) -> Compaction {
        Compaction::new(
            Ulid::new(),
            CompactionSpec::new(vec![SourceId::SortedRun(1)], 1),
        )
        .with_ctx(Some(CompactionContext::new(subcompactions, Some(0))))
    }

    #[test]
    fn test_trivial_move_output_builds_sorted_run_from_disjoint_inputs() {
        let l0 = bounded_sst_view(2, b"m", b"n");
        let sr_first = bounded_sst_view(1, b"a", b"b");
        let sr_last = bounded_sst_view(3, b"z", b"z");
        let mut db_state = ManifestCore::new();
        Arc::make_mut(&mut db_state.tree).l0 = VecDeque::from([l0.clone()]);
        Arc::make_mut(&mut db_state.tree).compacted =
            vec![SortedRun::new(1, [sr_first.clone(), sr_last.clone()])];
        let compaction = Compaction::new(
            Ulid::new(),
            CompactionSpec::new(vec![SstView(l0.id), SourceId::SortedRun(1)], 2),
        );

        let output = compaction
            .trivial_move_output(&db_state)
            .expect("disjoint inputs should be a trivial move");

        assert_eq!(output.id, 2);
        assert_eq!(
            output
                .sst_views()
                .iter()
                .map(|view| view.id)
                .collect::<Vec<_>>(),
            vec![sr_first.id, l0.id, sr_last.id]
        );
    }

    #[test]
    fn test_trivial_move_output_rejects_overlapping_inputs() {
        let l0 = bounded_sst_view(1, b"a", b"m");
        let sr_view = bounded_sst_view(2, b"m", b"z");
        let mut db_state = ManifestCore::new();
        Arc::make_mut(&mut db_state.tree).l0 = VecDeque::from([l0.clone()]);
        Arc::make_mut(&mut db_state.tree).compacted = vec![SortedRun::new(1, [sr_view])];
        let compaction = Compaction::new(
            Ulid::new(),
            CompactionSpec::new(vec![SstView(l0.id), SourceId::SortedRun(1)], 2),
        );

        assert!(compaction.trivial_move_output(&db_state).is_none());
    }

    #[test]
    fn test_trivial_move_output_rejects_drain() {
        let db_state = ManifestCore::new();
        let compaction = Compaction::new(
            Ulid::new(),
            CompactionSpec::drain_segment(
                Bytes::from_static(b"segment/"),
                vec![SourceId::SortedRun(1)],
            ),
        );

        assert!(compaction.trivial_move_output(&db_state).is_none());
    }

    fn set_test_subcompactions(compaction: &mut Compaction, subcompactions: Vec<Subcompaction>) {
        compaction.set_ctx(Some(CompactionContext::new(subcompactions, Some(0))));
    }

    #[test]
    fn test_should_extend_output_ssts_when_subcompaction_plan_unchanged() {
        // given: a compaction with one subcompaction that recorded one output SST
        let range = BytesRange::unbounded();
        let sst = test_subcompaction_sst(b"a");
        let mut compaction =
            test_compaction_with_subcompactions(vec![
                Subcompaction::new(range.clone()).with_output_ssts(vec![sst.clone()])
            ]);

        // when: the plan is set again with the output SST list extended
        set_test_subcompactions(
            &mut compaction,
            vec![
                Subcompaction::new(range).with_output_ssts(vec![sst, test_subcompaction_sst(b"m")])
            ],
        );

        // then: the extended output SSTs are accepted
        assert_eq!(compaction.subcompactions()[0].output_ssts().len(), 2);
    }

    #[test]
    #[should_panic(expected = "subcompaction plan must not change once set")]
    fn test_should_panic_when_subcompaction_plan_size_changes() {
        // given: a compaction with a single-range subcompaction plan
        let mut compaction =
            test_compaction_with_subcompactions(vec![Subcompaction::new(BytesRange::unbounded())]);

        // when/then: setting a plan with a different number of ranges panics
        set_test_subcompactions(
            &mut compaction,
            vec![
                Subcompaction::new(BytesRange::from_slice(..b"m".as_slice())),
                Subcompaction::new(BytesRange::from_slice(b"m".as_slice()..)),
            ],
        );
    }

    #[test]
    #[should_panic(expected = "subcompaction ranges are immutable once set")]
    fn test_should_panic_when_subcompaction_range_changes() {
        // given: a compaction with a subcompaction covering (-inf, "m")
        let mut compaction = test_compaction_with_subcompactions(vec![Subcompaction::new(
            BytesRange::from_slice(..b"m".as_slice()),
        )]);

        // when/then: setting a plan whose range differs panics
        set_test_subcompactions(
            &mut compaction,
            vec![Subcompaction::new(BytesRange::from_slice(
                ..b"z".as_slice(),
            ))],
        );
    }

    #[test]
    #[should_panic(expected = "must always extend previous output SSTs")]
    fn test_should_panic_when_subcompaction_output_ssts_shrink() {
        // given: a compaction whose subcompaction already recorded output SST "a"
        let range = BytesRange::unbounded();
        let mut compaction =
            test_compaction_with_subcompactions(vec![Subcompaction::new(range.clone())
                .with_output_ssts(vec![test_subcompaction_sst(b"a")])]);

        // when/then: replacing (rather than extending) the output SSTs panics
        set_test_subcompactions(
            &mut compaction,
            vec![Subcompaction::new(range).with_output_ssts(vec![test_subcompaction_sst(b"b")])],
        );
    }

    #[test]
    fn test_trim_keeps_latest_finished_and_active_compactions() {
        let mut compactions = Compactions::new(0);
        let oldest_finished = Ulid::from_parts(1, 0);
        let latest_finished = Ulid::from_parts(2, 0);
        let active = Ulid::from_parts(3, 0);

        compactions.insert(compaction_with_status(
            oldest_finished,
            CompactionStatus::Completed,
        ));
        compactions.insert(compaction_with_status(
            latest_finished,
            CompactionStatus::Completed,
        ));
        compactions.insert(compaction_with_status(active, CompactionStatus::Submitted));

        compactions.retain_active_and_last_finished();

        assert!(compactions.contains(&active));
        assert!(compactions.contains(&latest_finished));
        assert!(!compactions.contains(&oldest_finished));
    }

    #[test]
    fn test_compaction_status_classifications() {
        assert!(CompactionStatus::Submitted.active());
        assert!(CompactionStatus::Scheduled.active());
        assert!(CompactionStatus::Running.active());
        assert!(CompactionStatus::Compacted.active());
        assert!(!CompactionStatus::Completed.active());
        assert!(!CompactionStatus::Failed.active());

        assert!(CompactionStatus::Submitted.counts_against_max_concurrent());
        assert!(CompactionStatus::Scheduled.counts_against_max_concurrent());
        assert!(CompactionStatus::Running.counts_against_max_concurrent());
        assert!(!CompactionStatus::Compacted.counts_against_max_concurrent());
        assert!(!CompactionStatus::Completed.counts_against_max_concurrent());
        assert!(!CompactionStatus::Failed.counts_against_max_concurrent());

        assert!(!CompactionStatus::Submitted.finished());
        assert!(!CompactionStatus::Scheduled.finished());
        assert!(!CompactionStatus::Running.finished());
        assert!(!CompactionStatus::Compacted.finished());
        assert!(CompactionStatus::Completed.finished());
        assert!(CompactionStatus::Failed.finished());
    }

    #[test]
    fn test_trim_keeps_only_most_recent_finished_when_no_active() {
        let mut compactions = Compactions::new(0);
        let older = Ulid::from_parts(10, 0);
        let middle = Ulid::from_parts(20, 0);
        let newest = Ulid::from_parts(30, 0);

        compactions.insert(compaction_with_status(older, CompactionStatus::Completed));
        compactions.insert(compaction_with_status(middle, CompactionStatus::Failed));
        compactions.insert(compaction_with_status(newest, CompactionStatus::Completed));

        compactions.retain_active_and_last_finished();

        assert!(!compactions.contains(&older));
        assert!(!compactions.contains(&middle));
        assert!(compactions.contains(&newest));
    }

    #[test]
    fn test_trim_preserves_all_active_when_no_finished() {
        let mut compactions = Compactions::new(0);
        let submitted = Ulid::from_parts(1, 0);
        let running = Ulid::from_parts(2, 0);

        compactions.insert(compaction_with_status(
            submitted,
            CompactionStatus::Submitted,
        ));
        compactions.insert(compaction_with_status(running, CompactionStatus::Running));

        compactions.retain_active_and_last_finished();

        assert!(compactions.contains(&submitted));
        assert!(compactions.contains(&running));
    }

    #[test]
    fn test_merge_remote_compactions_preserves_local_and_adds_remote() {
        let manifest = new_dirty_manifest();
        let compactor_epoch = manifest.value.compactor_epoch;
        let local_running = Ulid::from_parts(1, 0);
        let local_finished = Ulid::from_parts(2, 0);

        let mut local_compactions = new_dirty_compactions(compactor_epoch);
        local_compactions.value.insert(compaction_with_status(
            local_running,
            CompactionStatus::Running,
        ));
        local_compactions.value.insert(compaction_with_status(
            local_finished,
            CompactionStatus::Completed,
        ));

        let mut state = CompactorState::new(manifest, local_compactions);

        let remote_submitted = Ulid::from_parts(3, 0);
        let mut remote_compactions = new_dirty_compactions(compactor_epoch);
        remote_compactions.value.insert(compaction_with_status(
            remote_submitted,
            CompactionStatus::Submitted,
        ));

        state.merge_remote_compactions(remote_compactions);

        let merged = &state.compactions.value;
        assert!(merged.contains(&local_running));
        assert_eq!(
            merged.get(&local_running).expect("not found").status(),
            CompactionStatus::Running
        );
        assert!(merged.contains(&local_finished));
        assert_eq!(
            merged.get(&local_finished).expect("not found").status(),
            CompactionStatus::Completed
        );
        assert!(merged.contains(&remote_submitted));
        assert_eq!(
            merged.get(&remote_submitted).expect("not found").status(),
            CompactionStatus::Submitted
        );
    }

    #[test]
    fn test_merge_remote_compactions_accepts_compacted_from_worker() {
        let manifest = new_dirty_manifest();
        let compactor_epoch = manifest.value.compactor_epoch;
        let id = Ulid::from_parts(1, 0);

        let mut local_compactions = new_dirty_compactions(compactor_epoch);
        local_compactions
            .value
            .insert(compaction_with_status(id, CompactionStatus::Running));
        let mut state = CompactorState::new(manifest, local_compactions);

        let mut remote_compactions = new_dirty_compactions(compactor_epoch);
        remote_compactions
            .value
            .insert(compaction_with_status(id, CompactionStatus::Compacted));

        state.merge_remote_compactions(remote_compactions);

        let merged = &state.compactions.value;
        assert_eq!(
            merged.get(&id).expect("not found").status(),
            CompactionStatus::Compacted
        );
    }

    #[test]
    fn test_merge_remote_compactions_drops_non_submitted_remote() {
        let manifest = new_dirty_manifest();
        let compactor_epoch = manifest.value.compactor_epoch;

        let local_running = Ulid::from_parts(1, 0);
        let mut local_compactions = new_dirty_compactions(compactor_epoch);
        local_compactions.value.insert(compaction_with_status(
            local_running,
            CompactionStatus::Running,
        ));
        let mut state = CompactorState::new(manifest, local_compactions);

        let remote_running = Ulid::from_parts(3, 0);
        let mut remote_compactions = new_dirty_compactions(compactor_epoch);
        remote_compactions.value.insert(compaction_with_status(
            remote_running,
            CompactionStatus::Running,
        ));

        state.merge_remote_compactions(remote_compactions);

        let merged = &state.compactions.value;
        assert!(merged.contains(&local_running));
        assert_eq!(
            merged.get(&local_running).expect("not found").status(),
            CompactionStatus::Running
        );
        assert!(!merged.contains(&remote_running));
    }

    #[test]
    fn test_should_register_compaction() {
        // given:
        let rt = build_runtime();
        let (_, _, mut state, system_clock, rand) = build_test_state(rt.handle());

        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let spec = build_l0_compaction(&state.db_state().tree.l0, 0);
        // when:
        let compaction = Compaction::new(compaction_id, spec.clone());
        state
            .add_compaction(compaction)
            .expect("failed to add compaction");

        // then:
        let mut compactions = state.active_compactions();
        let expected = Compaction::new(compaction_id, spec);
        assert_eq!(compactions.next().expect("compaction not found"), &expected);
        assert!(compactions.next().is_none());
    }

    #[test]
    fn test_should_update_dbstate_when_compaction_finished() {
        // given:
        let rt = build_runtime();
        let (_, _, mut state, system_clock, rand) = build_test_state(rt.handle());
        let before_compaction = state.db_state().clone();
        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let spec = build_l0_compaction(&before_compaction.tree.l0, 0);
        let compaction = Compaction::new(compaction_id, spec);
        state
            .add_compaction(compaction)
            .expect("failed to add compaction");

        // when:
        let sr = SortedRun::new(0, before_compaction.tree.l0.iter().cloned());
        state.finish_compaction(compaction_id, sr.clone());

        // then:
        assert_eq!(
            state.db_state().tree.last_compacted_l0_sst_view_id,
            Some(before_compaction.tree.l0.front().unwrap().id)
        );
        assert_eq!(state.db_state().tree.l0.len(), 0);
        assert_eq!(state.db_state().tree.compacted.len(), 1);
        assert_eq!(state.db_state().tree.compacted.first().unwrap().id, sr.id);
        let expected_ids: Vec<SsTableId> = sr.sst_views().iter().map(|h| h.sst.id).collect();
        let found_ids: Vec<SsTableId> = state
            .db_state()
            .tree
            .compacted
            .first()
            .unwrap()
            .sst_views()
            .iter()
            .map(|h| h.sst.id)
            .collect();
        assert_eq!(expected_ids, found_ids);
    }

    #[test]
    fn test_finish_compaction_prunes_external_sst_ids() {
        use crate::manifest::ExternalDb;
        use uuid::Uuid;

        let rt = build_runtime();
        let (_, _, mut state, system_clock, rand) = build_test_state(rt.handle());
        let before_compaction = state.db_state().clone();

        // Seed external_dbs with a mix of IDs that are currently in L0 (will move to the
        // compacted sorted run and remain live) and a stale ID that was never in the
        // manifest (simulating a parent SST already compacted away on a prior cycle).
        let live_id = before_compaction.tree.l0.front().unwrap().sst.id;
        let stale_id = SsTableId::from(Ulid::new());
        state.manifest.value.external_dbs = vec![
            ExternalDb {
                path: "/parent/db".to_string(),
                source_checkpoint_id: Uuid::new_v4(),
                final_checkpoint_id: Some(Uuid::new_v4()),
                sst_ids: vec![live_id, stale_id],
            },
            ExternalDb {
                path: "/other/parent".to_string(),
                source_checkpoint_id: Uuid::new_v4(),
                final_checkpoint_id: Some(Uuid::new_v4()),
                sst_ids: vec![stale_id],
            },
        ];

        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let spec = build_l0_compaction(&before_compaction.tree.l0, 0);
        state
            .add_compaction(Compaction::new(compaction_id, spec))
            .expect("failed to add compaction");

        let sr = SortedRun::new(0, before_compaction.tree.l0.iter().cloned());
        state.finish_compaction(compaction_id, sr);

        let external_dbs = &state.manifest().value.external_dbs;
        assert_eq!(external_dbs.len(), 2, "entries must be retained");
        assert_eq!(
            external_dbs[0].sst_ids,
            vec![live_id],
            "stale id must be pruned, live id kept"
        );
        assert!(
            external_dbs[1].sst_ids.is_empty(),
            "fully-stale entry must be retained with empty sst_ids (detach is GC's job)"
        );
        assert!(external_dbs[1].final_checkpoint_id.is_some());
    }

    #[test]
    fn test_should_remove_compaction_when_compaction_finished() {
        // given:
        let rt = build_runtime();
        let (_, _, mut state, system_clock, rand) = build_test_state(rt.handle());
        let before_compaction = state.db_state().clone();
        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let spec = build_l0_compaction(&before_compaction.tree.l0, 0);
        let compaction = Compaction::new(compaction_id, spec);
        state
            .add_compaction(compaction)
            .expect("failed to add compaction");

        // when:
        let sr = SortedRun::new(0, before_compaction.tree.l0.iter().cloned());
        state.finish_compaction(compaction_id, sr);

        // then:
        assert_eq!(state.active_compactions().count(), 0)
    }

    #[test]
    fn test_should_merge_db_state_correctly_when_never_compacted() {
        // given:
        let rt = build_runtime();
        let (os, mut sm, mut state, _, _) = build_test_state(rt.handle());
        // open a new db and write another l0
        let db = build_db(os.clone(), rt.handle());
        rt.block_on(db.put(&[b'a'; 16], &[b'b'; 48])).unwrap();
        rt.block_on(db.put(&[b'j'; 16], &[b'k'; 48])).unwrap();
        wait_for_manifest_with_l0_len(&mut sm, rt.handle(), state.db_state().tree.l0.len() + 1);

        // when:
        state.merge_remote_manifest(sm.prepare_dirty().unwrap());

        // then:
        assert!(state
            .db_state()
            .tree
            .last_compacted_l0_sst_view_id
            .is_none());
        let expected_merged_l0s: Vec<Ulid> = sm
            .manifest()
            .core
            .tree
            .l0
            .iter()
            .map(|t| t.sst.id.value())
            .collect();
        let merged_l0s: Vec<Ulid> = state
            .db_state()
            .tree
            .l0
            .iter()
            .map(|h| h.sst.id.value())
            .collect();
        assert_eq!(expected_merged_l0s, merged_l0s);
    }

    #[test]
    fn test_merge_remote_manifest_reestablishes_external_sst_invariant() {
        let manifest = new_dirty_manifest();
        let compactions = new_dirty_compactions(manifest.value.compactor_epoch);
        let mut state = CompactorState::new(manifest, compactions);
        let stale_id = SsTableId::from(Ulid::new());
        let mut remote = new_dirty_manifest();
        remote.value.external_dbs = vec![crate::manifest::ExternalDb {
            path: "/parent/db".to_string(),
            source_checkpoint_id: uuid::Uuid::new_v4(),
            final_checkpoint_id: Some(uuid::Uuid::new_v4()),
            sst_ids: vec![stale_id],
        }];

        state.merge_remote_manifest(remote);

        let external = &state.manifest().value.external_dbs;
        assert_eq!(external.len(), 1, "detach metadata must be retained");
        assert!(
            external[0].sst_ids.is_empty(),
            "IDs absent from the merged tree must not be resurrected"
        );
        assert!(external[0].final_checkpoint_id.is_some());
    }

    #[test]
    fn test_should_merge_db_state_correctly() {
        // given:
        let rt = build_runtime();
        let (os, mut sm, mut state, system_clock, rand) = build_test_state(rt.handle());
        // compact the last sst
        let original_l0s = &state.db_state().clone().tree.l0;
        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let spec = CompactionSpec::new(vec![SstView(original_l0s.back().unwrap().id)], 0);
        let compaction = Compaction::new(compaction_id, spec);
        state
            .add_compaction(compaction)
            .expect("failed to add compaction");
        state.finish_compaction(
            compaction_id,
            SortedRun::new(0, [original_l0s.back().unwrap().clone()]),
        );
        // open a new db and write another l0
        let db = build_db(os.clone(), rt.handle());
        rt.block_on(db.put(&[b'a'; 16], &[b'b'; 48])).unwrap();
        rt.block_on(db.put(&[b'j'; 16], &[b'k'; 48])).unwrap();
        wait_for_manifest_with_l0_len(&mut sm, rt.handle(), original_l0s.len() + 1);
        let db_state_before_merge = state.db_state().clone();

        // when:
        state.merge_remote_manifest(sm.prepare_dirty().unwrap());

        // then:
        let db_state = state.db_state();
        let mut expected_merged_l0s: VecDeque<Ulid> =
            original_l0s.iter().map(|h| h.sst.id.value()).collect();
        expected_merged_l0s.pop_back();
        let new_l0 = sm.manifest().core.tree.l0.front().unwrap().sst.id.value();
        expected_merged_l0s.push_front(new_l0);
        let merged_l0: VecDeque<Ulid> = db_state.tree.l0.iter().map(|h| h.sst.id.value()).collect();
        assert_eq!(merged_l0, expected_merged_l0s);
        assert_eq!(
            compacted_to_description(&db_state.tree.compacted),
            compacted_to_description(&db_state_before_merge.tree.compacted)
        );
        assert_eq!(
            db_state.replay_after_wal_id,
            sm.manifest().core.replay_after_wal_id
        );
        assert_eq!(db_state.next_wal_sst_id, sm.manifest().core.next_wal_sst_id);
    }

    #[test]
    fn test_should_merge_db_state_correctly_when_all_l0_compacted() {
        // given:
        let rt = build_runtime();
        let (os, mut sm, mut state, system_clock, rand) = build_test_state(rt.handle());
        // compact the last sst
        let original_l0s = &state.db_state().clone().tree.l0;
        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());

        let spec = CompactionSpec::new(original_l0s.iter().map(|h| SstView(h.id)).collect(), 0);
        let compaction = Compaction::new(compaction_id, spec);
        state
            .add_compaction(compaction)
            .expect("failed to add compaction");
        state.finish_compaction(
            compaction_id,
            SortedRun::new(0, original_l0s.iter().cloned()),
        );
        assert_eq!(state.db_state().tree.l0.len(), 0);
        // open a new db and write another l0
        let db = build_db(os.clone(), rt.handle());
        rt.block_on(db.put(&[b'a'; 16], &[b'b'; 48])).unwrap();
        rt.block_on(db.put(&[b'j'; 16], &[b'k'; 48])).unwrap();
        wait_for_manifest_with_l0_len(&mut sm, rt.handle(), original_l0s.len() + 1);

        // when:
        state.merge_remote_manifest(sm.prepare_dirty().unwrap());

        // then:
        let db_state = state.db_state();
        let mut expected_merged_l0s = VecDeque::new();
        let new_l0 = sm.manifest().core.tree.l0.front().unwrap().sst.id.value();
        expected_merged_l0s.push_front(new_l0);
        let merged_l0: VecDeque<Ulid> = db_state.tree.l0.iter().map(|h| h.sst.id.value()).collect();
        assert_eq!(merged_l0, expected_merged_l0s);
    }

    #[test]
    fn test_should_merge_db_state_with_new_checkpoints() {
        // given:
        let manifest = new_dirty_manifest();
        let compactions = new_dirty_compactions(manifest.value.compactor_epoch);
        let mut state = CompactorState::new(manifest, compactions);
        // mimic an externally added checkpoint
        let mut dirty = new_dirty_manifest();
        let checkpoint = Checkpoint {
            id: uuid::Uuid::new_v4(),
            manifest_id: 1,
            expire_time: None,
            create_time: DefaultSystemClock::default().now(),
            name: None,
        };
        dirty.value.core.checkpoints.push(checkpoint.clone());

        // when:
        state.merge_remote_manifest(dirty);

        // then:
        assert_eq!(vec![checkpoint], state.db_state().checkpoints);
    }

    #[test]
    fn test_should_adopt_remote_segments_on_merge() {
        fn view(seq: u64) -> SsTableView {
            let ulid = Ulid::from_parts(seq, 0);
            SsTableView::identity(SsTableHandle::new(
                SsTableId::from(ulid),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo::default(),
            ))
        }
        let v_a = view(1);
        let v_b = view(2);

        // Local compactor state starts with no segment configuration.
        let manifest = new_dirty_manifest();
        let compactions = new_dirty_compactions(manifest.value.compactor_epoch);
        let mut state = CompactorState::new(manifest, compactions);
        assert!(state.db_state().segments.is_empty());
        assert!(state.db_state().segment_extractor_name.is_none());

        // Remote (writer-authored) manifest configures an extractor with two
        // populated segments. Constructed sorted by prefix to satisfy the
        // `ManifestCore::segments` invariant.
        let mut dirty = new_dirty_manifest();
        dirty.value.core.segment_extractor_name = Some("hour-bucket".to_string());
        dirty.value.core.segments = vec![
            Segment {
                prefix: Bytes::from_static(b"hour=11/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::from(vec![v_b.clone()]),
                    compacted: vec![],
                }),
            },
            Segment {
                prefix: Bytes::from_static(b"hour=12/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::from(vec![v_a.clone()]),
                    compacted: vec![],
                }),
            },
        ];

        // when:
        state.merge_remote_manifest(dirty);

        // then: the compactor adopts the writer's segment configuration with
        // its L0s. A regression that drops segments on merge would surface here.
        let merged = state.db_state();
        assert_eq!(
            merged.segment_extractor_name.as_deref(),
            Some("hour-bucket")
        );
        assert_eq!(merged.segments.len(), 2);
        // The merged list is sorted by prefix.
        assert_eq!(merged.segments[0].prefix, Bytes::from_static(b"hour=11/"));
        assert_eq!(merged.segments[1].prefix, Bytes::from_static(b"hour=12/"));
    }

    #[test]
    fn test_segment_drain_lifecycle() {
        // Walk the full Live → Marker → Pruned cycle across three commits:
        //   V1: compactor commits drain; segment is a marker on compactor.
        //   V2: writer's commit-time merge prunes the marker.
        //   V3: compactor reads writer's V2, follows the prune, drops the
        //       segment from its own manifest.
        use crate::manifest::merge_segments_from_compactor;

        fn view(seq: u64) -> SsTableView {
            let ulid = Ulid::from_parts(seq, 0);
            SsTableView::identity(SsTableHandle::new(
                SsTableId::from(ulid),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo::default(),
            ))
        }
        let v1 = view(1);
        let prefix = Bytes::from_static(b"hour=12/");

        // V0: writer has X with l0=[v1]. Compactor matches.
        let writer_v0 = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![v1.clone()]),
                compacted: vec![],
            }),
        }];

        // V1: compactor's drain post-spec state. Watermark advanced to v1,
        // l0 cleared, no compacted runs. Compactor commits — its compactor-
        // side merge with writer.V0 keeps the marker.
        let compactor_post_drain = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: Some(v1.id),
                last_compacted_l0_sst_id: None,
                l0: VecDeque::new(),
                compacted: vec![],
            }),
        }];
        let v1_segments =
            crate::manifest::merge_segments_from_writer(&compactor_post_drain, &writer_v0);
        assert_eq!(v1_segments.len(), 1);

        // V2: writer reads V1 (compactor's manifest). Writer's local view
        // is still V0 (l0=[v1]). The writer-side merge kernel produces a
        // marker; the writer prunes it.
        let v2_segments = merge_segments_from_compactor(&writer_v0, &v1_segments);
        assert!(
            v2_segments.is_empty(),
            "writer should prune the drain marker"
        );

        // V3: compactor reads writer's V2 (no segments). Compactor's local
        // still holds the marker from V1. The compactor-side merge follows
        // the writer's prune and drops the segment.
        let v3_segments = crate::manifest::merge_segments_from_writer(&v1_segments, &v2_segments);
        assert!(
            v3_segments.is_empty(),
            "compactor should follow the writer's prune"
        );
    }

    #[test]
    fn test_should_submit_correct_compaction() {
        // given:
        let rt = build_runtime();
        let (_os, mut _sm, mut state, system_clock, rand) = build_test_state(rt.handle());
        // compact the last sst
        let original_l0s = &state.db_state().clone().tree.l0;
        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let spec = CompactionSpec::new(
            original_l0s
                .iter()
                .enumerate()
                .filter(|(i, _e)| i > &2usize)
                .map(|(_i, x)| SstView(x.id))
                .collect::<Vec<SourceId>>(),
            0,
        );
        let compaction = Compaction::new(compaction_id, spec);
        let result = state.add_compaction(compaction);

        // then:
        assert!(result.is_ok());
    }

    #[test]
    fn test_source_boundary_compaction() {
        // given:
        let rt = build_runtime();
        let (_os, mut _sm, mut state, system_clock, rand) = build_test_state(rt.handle());
        let original_l0s = &state.db_state().clone().tree.l0;
        let original_srs = &state.db_state().clone().tree.compacted;
        // L0: from 4th onward (index > 2)
        let l0_sources = original_l0s.iter().skip(3).map(|h| SstView(h.id));

        // SRs: first 3 (index < 3)
        let sr_sources = original_srs
            .iter()
            .take(3)
            .map(|sr| SourceId::SortedRun(sr.id));

        // If you need both:
        let sources: Vec<SourceId> = l0_sources.chain(sr_sources).collect();

        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let spec = CompactionSpec::new(sources, 0);
        let compaction = Compaction::new(compaction_id, spec);
        let result = state.add_compaction(compaction);

        // or simply:
        assert!(result.is_ok());
    }

    // test helpers

    fn new_dirty_compactions(compactor_epoch: u64) -> DirtyObject<Compactions> {
        new_dirty_object(1u64, Compactions::new(compactor_epoch))
    }

    fn compaction_with_status(id: Ulid, status: CompactionStatus) -> Compaction {
        Compaction::new(id, CompactionSpec::new(vec![], 0)).with_status(status)
    }

    fn run_for<T, F>(duration: Duration, mut f: F) -> Option<T>
    where
        F: FnMut() -> Option<T>,
    {
        let clock = DefaultSystemClock::default();
        let start = clock.now();
        while clock
            .now()
            .signed_duration_since(start)
            .to_std()
            .expect("duration < 0 not allowed")
            < duration
        {
            let maybe_result = f();
            if maybe_result.is_some() {
                return maybe_result;
            }
            sleep(Duration::from_millis(100));
        }
        None
    }

    #[derive(PartialEq, Debug)]
    struct SortedRunDescription {
        id: u32,
        ssts: Vec<SsTableId>,
    }

    fn build_runtime() -> Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    fn compacted_to_description(compacted: &[SortedRun]) -> Vec<SortedRunDescription> {
        compacted.iter().map(sorted_run_to_description).collect()
    }

    fn sorted_run_to_description(sr: &SortedRun) -> SortedRunDescription {
        SortedRunDescription {
            id: sr.id,
            ssts: sr.sst_views().iter().map(|h| h.sst.id).collect(),
        }
    }

    fn wait_for_manifest_with_l0_len(
        stored_manifest: &mut StoredManifest,
        tokio_handle: &Handle,
        len: usize,
    ) {
        run_for(Duration::from_secs(30), || {
            let manifest = tokio_handle.block_on(stored_manifest.refresh()).unwrap();
            if manifest.core.tree.l0.len() == len {
                return Some(manifest.core.clone());
            }
            None
        })
        .expect("no manifest found with l0 len");
    }

    fn build_l0_compaction(ssts: &VecDeque<SsTableView>, dst: u32) -> CompactionSpec {
        let sources = ssts.iter().map(|h| SstView(h.id)).collect();
        CompactionSpec::new(sources, dst)
    }

    /// Verifies that `finish_compaction` applies its effects to the spec's
    /// target segment (RFC-0024) and not the root tree. Constructs a manifest
    /// with a single named segment whose `compacted` list holds two SRs,
    /// commits a compaction that merges them into a new SR within that
    /// segment, and asserts the segment's tree is updated while the root
    /// tree is untouched.
    #[test]
    fn test_finish_compaction_routes_to_target_segment() {
        let rt = build_runtime();
        let (_os, _sm, mut state, system_clock, rand) = build_test_state(rt.handle());

        // Add a named segment with two compacted SRs (ids 5 and 3, list-position
        // ordered newest-first as per ManifestCore conventions).
        let prefix = Bytes::from_static(b"hour=12/");
        let sr5 = SortedRun::new(5, []);
        let sr3 = SortedRun::new(3, []);
        let segment = Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::new(),
                compacted: vec![sr5.clone(), sr3.clone()],
            }),
        };
        state.manifest.value.core.segments = vec![segment];
        let root_compacted_before = state.db_state().tree.compacted.clone();

        // Submit a compaction that targets the segment, merging SR(5) and SR(3)
        // into SR(7).
        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let spec = CompactionSpec::for_segment(
            prefix.clone(),
            vec![SourceId::SortedRun(5), SourceId::SortedRun(3)],
            7,
        );
        state
            .add_compaction(Compaction::new(compaction_id, spec))
            .expect("failed to add compaction");

        // Finish the compaction with a fresh output SR.
        let output = SortedRun::new(7, []);
        state.finish_compaction(compaction_id, output);

        // The segment's compacted list now holds only the new SR(7).
        let seg = state
            .db_state()
            .tree_for_segment(&prefix)
            .expect("segment missing");
        assert_eq!(seg.compacted.len(), 1);
        assert_eq!(seg.compacted[0].id, 7);

        // The root tree is unchanged — no leakage from the segment compaction.
        assert_eq!(state.db_state().tree.compacted, root_compacted_before);
    }

    /// If the target segment is dropped between submission and finish, the
    /// compaction must be marked terminal so it doesn't linger as Active.
    #[test]
    fn test_finish_compaction_marks_failed_when_segment_missing() {
        let rt = build_runtime();
        let (_os, _sm, mut state, system_clock, rand) = build_test_state(rt.handle());

        // Seed SR(7) in the segment so the spec's source SR(7) passes
        // source-isolation; the spec rewrites that SR (destination=7).
        let prefix = Bytes::from_static(b"seg/");
        state.manifest.value.core.segments = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::new(),
                compacted: vec![SortedRun::new(7, [])],
            }),
        }];

        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let spec = CompactionSpec::for_segment(prefix.clone(), vec![SourceId::SortedRun(7)], 7);
        state
            .add_compaction(Compaction::new(compaction_id, spec))
            .expect("submission must register");

        // Segment dropped after submission, before finish.
        state.manifest.value.core.segments = Vec::new();

        let output = SortedRun::new(7, []);
        state.finish_compaction(compaction_id, output);

        let compaction = state
            .compactions
            .value
            .get(&compaction_id)
            .expect("compaction should still be tracked");
        assert_eq!(compaction.status(), CompactionStatus::Failed);
        assert_eq!(state.compactions.value.iter_active().count(), 0);
    }

    /// SR ids are globally unique across all trees (RFC-0024), so the
    /// active-conflict check rejects a duplicate destination regardless of
    /// which segment each spec targets.
    #[test]
    fn test_add_compaction_active_conflict_check_is_global() {
        let rt = build_runtime();
        let (_os, _sm, mut state, system_clock, rand) = build_test_state(rt.handle());

        // Seed real sources so the source-isolation check passes for both
        // submissions. Root tree gets SR(99); segment "seg/" gets SR(100).
        Arc::make_mut(&mut state.manifest.value.core.tree).compacted = vec![SortedRun::new(99, [])];
        let prefix = Bytes::from_static(b"seg/");
        state.manifest.value.core.segments = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::new(),
                compacted: vec![SortedRun::new(100, [])],
            }),
        }];

        let root_id = rand.rng().gen_ulid(system_clock.as_ref());
        let root_spec = CompactionSpec::new(vec![SourceId::SortedRun(99)], 7);
        state
            .add_compaction(Compaction::new(root_id, root_spec))
            .expect("root-tree compaction must register");

        let seg_id = rand.rng().gen_ulid(system_clock.as_ref());
        let seg_spec = CompactionSpec::for_segment(prefix, vec![SourceId::SortedRun(100)], 7);
        let err = state
            .add_compaction(Compaction::new(seg_id, seg_spec))
            .expect_err(
                "segment spec must be rejected when an active root-tree compaction shares the destination",
            );
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    /// Two in-flight compactions targeting the same destination must be
    /// rejected — covers the trivial intra-segment case.
    #[test]
    fn test_add_compaction_active_conflict_rejects_same_segment_same_destination() {
        let rt = build_runtime();
        let (_os, _sm, mut state, system_clock, rand) = build_test_state(rt.handle());

        // Seed SR(7) in the root tree so the source-isolation check passes
        // for the first submission; the spec rewrites that SR (destination=7).
        Arc::make_mut(&mut state.manifest.value.core.tree).compacted = vec![SortedRun::new(7, [])];

        let first_id = rand.rng().gen_ulid(system_clock.as_ref());
        let first = CompactionSpec::new(vec![SourceId::SortedRun(7)], 7);
        state
            .add_compaction(Compaction::new(first_id, first))
            .expect("first compaction must register");

        let second_id = rand.rng().gen_ulid(system_clock.as_ref());
        let second = CompactionSpec::new(vec![SourceId::SortedRun(7)], 7);
        let err = state
            .add_compaction(Compaction::new(second_id, second))
            .expect_err("same-segment same-destination must be rejected");
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    /// Drain specs have no destination, so the destination-collision check
    /// doesn't catch duplicates; the explicit per-segment guard must reject
    /// a second drain targeting the same segment while the first is active.
    #[test]
    fn test_add_compaction_rejects_concurrent_drain_on_same_segment() {
        let rt = build_runtime();
        let (_os, _sm, mut state, system_clock, rand) = build_test_state(rt.handle());

        let prefix = Bytes::from_static(b"seg/");
        let l0_a = drain_test_view(1);
        let l0_b = drain_test_view(2);
        state.manifest.value.core.segments = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![l0_b.clone(), l0_a.clone()]),
                compacted: Vec::new(),
            }),
        }];

        let first_id = rand.rng().gen_ulid(system_clock.as_ref());
        let first = CompactionSpec::drain_segment(prefix.clone(), vec![SstView(l0_a.id)]);
        state
            .add_compaction(Compaction::new(first_id, first))
            .expect("first drain must register");

        let second_id = rand.rng().gen_ulid(system_clock.as_ref());
        let second = CompactionSpec::drain_segment(prefix, vec![SstView(l0_b.id)]);
        let err = state
            .add_compaction(Compaction::new(second_id, second))
            .expect_err("second drain on same segment must be rejected");
        assert!(matches!(err, SlateDBError::InvalidCompaction));
    }

    /// Drain specs on different segments may run concurrently — the
    /// per-segment guard only blocks duplicates on the same target.
    #[test]
    fn test_add_compaction_allows_concurrent_drains_on_different_segments() {
        let rt = build_runtime();
        let (_os, _sm, mut state, system_clock, rand) = build_test_state(rt.handle());

        let prefix_a = Bytes::from_static(b"seg-a/");
        let prefix_b = Bytes::from_static(b"seg-b/");
        let l0_a = drain_test_view(1);
        let l0_b = drain_test_view(2);
        state.manifest.value.core.segments = vec![
            Segment {
                prefix: prefix_a.clone(),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::from(vec![l0_a.clone()]),
                    compacted: Vec::new(),
                }),
            },
            Segment {
                prefix: prefix_b.clone(),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::from(vec![l0_b.clone()]),
                    compacted: Vec::new(),
                }),
            },
        ];

        let first_id = rand.rng().gen_ulid(system_clock.as_ref());
        let first = CompactionSpec::drain_segment(prefix_a, vec![SstView(l0_a.id)]);
        state
            .add_compaction(Compaction::new(first_id, first))
            .expect("first drain must register");

        let second_id = rand.rng().gen_ulid(system_clock.as_ref());
        let second = CompactionSpec::drain_segment(prefix_b, vec![SstView(l0_b.id)]);
        state
            .add_compaction(Compaction::new(second_id, second))
            .expect("drain on a different segment must register");
    }

    /// Same as the tiered case: a drain compaction whose segment disappears
    /// between submission and finish must be marked terminal.
    #[test]
    fn test_finish_drain_compaction_marks_failed_when_segment_missing() {
        let rt = build_runtime();
        let (_os, _sm, mut state, system_clock, rand) = build_test_state(rt.handle());

        let prefix = Bytes::from_static(b"seg/");
        let l0 = drain_test_view(1);
        state.manifest.value.core.segments = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![l0.clone()]),
                compacted: Vec::new(),
            }),
        }];

        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let spec = CompactionSpec::drain_segment(prefix, vec![SstView(l0.id)]);
        state
            .add_compaction(Compaction::new(compaction_id, spec))
            .expect("drain submission must register");

        // Segment dropped after submission, before finish.
        state.manifest.value.core.segments = Vec::new();

        state.finish_drain_compaction(compaction_id);

        let compaction = state
            .compactions
            .value
            .get(&compaction_id)
            .expect("compaction should still be tracked");
        assert_eq!(compaction.status(), CompactionStatus::Failed);
        assert_eq!(state.compactions.value.iter_active().count(), 0);
    }

    fn drain_test_view(seed: u64) -> SsTableView {
        let ulid = Ulid::from_parts(seed, 0);
        SsTableView::identity(SsTableHandle::new(
            SsTableId::from(ulid),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo::default(),
        ))
    }

    /// Drain on a segment that contains exactly the observed L0s and SRs
    /// produces a "drain marker" (RFC-0024): empty `l0`, empty `compacted`,
    /// watermark advanced to the newest drained L0.
    #[test]
    fn test_finish_drain_compaction_produces_marker() {
        let rt = build_runtime();
        let (_os, _sm, mut state, system_clock, rand) = build_test_state(rt.handle());

        // Two L0s (newest first) and one SR in the segment.
        let l0_newer = drain_test_view(2);
        let l0_older = drain_test_view(1);
        let sr = SortedRun::new(5, []);
        let prefix = Bytes::from_static(b"hour=10/");
        state.manifest.value.core.segments = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![l0_newer.clone(), l0_older.clone()]),
                compacted: vec![sr.clone()],
            }),
        }];

        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let spec = CompactionSpec::drain_segment(
            prefix.clone(),
            vec![
                SstView(l0_newer.id),
                SstView(l0_older.id),
                SourceId::SortedRun(sr.id),
            ],
        );
        state
            .add_compaction(Compaction::new(compaction_id, spec))
            .expect("drain compaction must register");

        state.finish_drain_compaction(compaction_id);

        let seg = state
            .db_state()
            .tree_for_segment(&prefix)
            .expect("segment present");
        assert!(seg.l0.is_empty(), "drained L0s must be removed");
        assert!(seg.compacted.is_empty(), "drained SRs must be removed");
        assert_eq!(
            seg.last_compacted_l0_sst_view_id,
            Some(l0_newer.id),
            "watermark must advance to the newest drained L0"
        );
    }

    /// A drain spec that lists only the L0s the compactor observed must not
    /// remove L0s above the watermark (concurrent late writes survive).
    /// The segment stays Live.
    #[test]
    fn test_finish_drain_compaction_with_late_l0_stays_live() {
        let rt = build_runtime();
        let (_os, _sm, mut state, system_clock, rand) = build_test_state(rt.handle());

        // Compactor observed only `l0_observed`. The newer `l0_late` arrived
        // afterward (seed > observed, so its ULID sorts higher in tree.l0).
        let l0_late = drain_test_view(2);
        let l0_observed = drain_test_view(1);
        let prefix = Bytes::from_static(b"hour=10/");
        state.manifest.value.core.segments = vec![Segment {
            prefix: prefix.clone(),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![l0_late.clone(), l0_observed.clone()]),
                compacted: Vec::new(),
            }),
        }];

        let compaction_id = rand.rng().gen_ulid(system_clock.as_ref());
        let spec = CompactionSpec::drain_segment(prefix.clone(), vec![SstView(l0_observed.id)]);
        state
            .add_compaction(Compaction::new(compaction_id, spec))
            .expect("drain compaction must register");

        state.finish_drain_compaction(compaction_id);

        let seg = state
            .db_state()
            .tree_for_segment(&prefix)
            .expect("segment present");
        // Late L0 survives.
        assert_eq!(seg.l0.len(), 1);
        assert_eq!(seg.l0.front().expect("late L0").id, l0_late.id);
        // Watermark advances only to the observed L0.
        assert_eq!(seg.last_compacted_l0_sst_view_id, Some(l0_observed.id));
    }

    fn build_db(os: Arc<dyn ObjectStore>, tokio_handle: &Handle) -> Db {
        let opts = Settings {
            l0_sst_size_bytes: 256,
            // make sure to run with the compactor disabled. The tests will explicitly
            // manage compaction execution and assert the associated state mutations.
            compactor_options: None,
            ..Default::default()
        };
        tokio_handle
            .block_on(Db::builder(PATH, os.clone()).with_settings(opts).build())
            .unwrap()
    }

    #[allow(clippy::type_complexity)]
    fn build_test_state(
        tokio_handle: &Handle,
    ) -> (
        Arc<dyn ObjectStore>,
        StoredManifest,
        CompactorState,
        Arc<dyn SystemClock>,
        Arc<DbRand>,
    ) {
        let os: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let db = build_db(os.clone(), tokio_handle);
        let l0_count: u64 = 5;
        for i in 0..l0_count {
            tokio_handle
                .block_on(db.put(&[b'a' + i as u8; 16], &[b'b' + i as u8; 48]))
                .unwrap();
            tokio_handle
                .block_on(db.put(&[b'j' + i as u8; 16], &[b'k' + i as u8; 48]))
                .unwrap();
        }
        tokio_handle.block_on(async {
            // Persist the seeded L0s into the manifest before close so reopen
            // doesn't reconstruct them from WAL replay on a slow shutdown.
            db.flush_with_options(FlushOptions {
                flush_type: FlushType::MemTable,
            })
            .await
            .unwrap();
            db.close().await.unwrap();
        });
        let system_clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());
        let rand: Arc<DbRand> = Arc::new(DbRand::default());

        let manifest_store = Arc::new(ManifestStore::new(&Path::from(PATH), os.clone()));
        let stored_manifest = tokio_handle
            .block_on(StoredManifest::load(
                manifest_store,
                Arc::new(DefaultSystemClock::new()),
            ))
            .unwrap();
        let compactions = new_dirty_compactions(stored_manifest.manifest().compactor_epoch);
        let state = CompactorState::new(stored_manifest.prepare_dirty().unwrap(), compactions);
        (os, stored_manifest, state, system_clock, rand)
    }
}
