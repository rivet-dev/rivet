use std::cmp::{max, min, Ordering};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::ops::{Bound, RangeBounds};
use std::sync::Arc;

use crate::bytes_range::BytesRange;
use crate::checkpoint::Checkpoint;
use crate::clone::{CloneSource, SegmentFilterFn, SegmentProjectionFn};
use crate::error::SlateDBError;
use crate::seq_tracker::SequenceTracker;
use crate::utils::IdGenerator;
use bytes::Bytes;
use log::{debug, warn};
use serde::Serialize;
use slatedb_common::DbRand;
use slatedb_txn_obj::DirtyObject;
use uuid::Uuid;

pub(crate) mod invariants;
pub(crate) mod store;

pub use crate::db_state::{SortedRun, SsTableHandle, SsTableId, SsTableInfo, SsTableView};

/// The per-source trees a union merges into each segment, keyed by segment
/// prefix and ordered within a segment by that segment's start key.
type SegmentContributors<'a> = BTreeMap<Bytes, Vec<&'a LsmTreeState>>;

/// Per-LSM-tree state. Shared shape between the unsegmented tree (held directly
/// on `ManifestCore`) and each named segment held in `ManifestCore::segments`.
#[derive(Clone, Default, PartialEq, Serialize, Debug)]
pub(crate) struct LsmTreeState {
    /// The last compacted l0 SstView ID.
    pub last_compacted_l0_sst_view_id: Option<ulid::Ulid>,

    /// The SST ID of the last compacted L0. In V2, view IDs differ from SST IDs,
    /// but V1 only stores SST IDs. This field preserves the SST ID so that a
    /// V1-encoded manifest can correctly reference the compacted L0.
    pub last_compacted_l0_sst_id: Option<ulid::Ulid>,

    /// A list of the L0 SST views that are valid to read in the `compacted` folder.
    pub l0: VecDeque<SsTableView>,

    /// A list of the sorted runs that are valid to read in the `compacted` folder.
    pub compacted: Vec<SortedRun>,
}

impl LsmTreeState {
    /// Compactor-side merge: combine the writer's view of this tree (`writer`)
    /// into the compactor's view (`self`). The compactor keeps its compacted
    /// runs and `last_compacted_l0_*` markers (which only change when a
    /// compaction completes) and adopts the writer's L0, trimmed to drop
    /// entries the compactor has already absorbed.
    pub(crate) fn merge_from_writer(&self, writer: &Self) -> Self {
        Self::merge_writer_and_compactor(writer, self)
    }

    /// Writer-side merge: combine the compactor's view of this tree
    /// (`compactor`) into the writer's view (`self`). The writer keeps its L0
    /// (trimmed at the compactor's `last_compacted_l0_*` markers) and adopts
    /// the compactor's compacted runs and markers.
    pub(crate) fn merge_from_compactor(&self, compactor: &Self) -> Self {
        Self::merge_writer_and_compactor(self, compactor)
    }

    /// True iff this tree has no L0 SSTs and no compacted runs. The watermark
    /// (`last_compacted_l0_sst_view_id`) may or may not be set; an empty tree
    /// with a set watermark is a drain marker.
    pub(crate) fn is_empty(&self) -> bool {
        self.l0.is_empty() && self.compacted.is_empty()
    }

    /// Iterate every SST view referenced by this tree — L0 views followed by
    /// the views of every sorted run.
    pub(crate) fn sst_views(&self) -> impl Iterator<Item = &SsTableView> {
        self.l0
            .iter()
            .chain(self.compacted.iter().flat_map(|sr| sr.sst_views().iter()))
    }

    /// Total number of SST views referenced by this tree — L0 plus every
    /// SST in every sorted run. Used by the read path to size scan
    /// parallelism.
    pub(crate) fn total_ssts(&self) -> usize {
        self.l0.len()
            + self
                .compacted
                .iter()
                .map(|sr| sr.sst_views().len())
                .sum::<usize>()
    }

    /// Canonical merge of a single LSM tree, called by both
    /// [`Self::merge_from_writer`] and [`Self::merge_from_compactor`]. The
    /// writer owns L0 and the compactor owns compacted runs / markers, so the
    /// merge keeps each side's authoritative state and drops L0 entries the
    /// compactor has already absorbed.
    pub(crate) fn merge_writer_and_compactor(writer: &Self, compactor: &Self) -> Self {
        let last_compacted_view = compactor.last_compacted_l0_sst_view_id;
        let last_compacted_sst = compactor.last_compacted_l0_sst_id;
        // todo: this is brittle. we are relying on the l0 list always being
        //       updated in an expected order. We should instead encode the
        //       ordering in the l0 SST IDs and assert that it follows the
        //       order.
        let l0: VecDeque<SsTableView> =
            if last_compacted_view.is_some() || last_compacted_sst.is_some() {
                writer
                    .l0
                    .iter()
                    .cloned()
                    .take_while(|view| {
                        // Match by view ID (V2 manifests), and by SST ID only
                        // when no view ID is known (V1). A projected clone can
                        // hold several L0 views of one SST, so matching a V2
                        // watermark by SST ID would also drop the uncompacted
                        // newer views of that SST.
                        match (last_compacted_view, last_compacted_sst) {
                            (Some(id), _) => view.id != id,
                            (None, Some(id)) => view.sst.id.value() != id,
                            (None, None) => true,
                        }
                    })
                    .collect()
            } else {
                writer.l0.clone()
            };
        Self {
            last_compacted_l0_sst_view_id: last_compacted_view,
            last_compacted_l0_sst_id: last_compacted_sst,
            l0,
            compacted: compactor.compacted.clone(),
        }
    }
}

/// Configuration controlling how a clone projection narrows a source manifest.
///
/// Composition rules used by [`Manifest::projected`]:
/// - For each tree (the unsegmented `core.tree` participates as a logical
///   segment with the empty prefix), `segment_filter` is consulted first.
///   When it returns `false`, the tree is dropped entirely.
/// - Otherwise, the effective range is `segment_projection(prefix)` (validated
///   against `[prefix, prefix++)` with `Unbounded` ends resolved to the
///   segment edges) intersected with `global_range`. Either input is treated
///   as "no constraint" when absent.
#[derive(Clone, Default)]
pub(crate) struct ProjectionConfig {
    pub(crate) global_range: Option<BytesRange>,
    pub(crate) segment_filter: Option<SegmentFilterFn>,
    pub(crate) segment_projection: Option<SegmentProjectionFn>,
}

/// Outcome of resolving a projection against a single tree.
enum SegmentAction {
    /// Filter rejected this segment — drop it from the projected manifest.
    Drop,
    /// Apply the carried range as the effective per-tree projection.
    Project(BytesRange),
    /// No projection applies — keep the tree as-is.
    PassThrough,
}

impl ProjectionConfig {
    #[cfg(test)]
    pub(crate) fn from_global_range(range: BytesRange) -> Self {
        Self {
            global_range: Some(range),
            segment_filter: None,
            segment_projection: None,
        }
    }

    pub(crate) fn is_noop(&self) -> bool {
        self.global_range.is_none()
            && self.segment_filter.is_none()
            && self.segment_projection.is_none()
    }
}

/// Per-segment LSM state (RFC-0024). Each segment owns the contiguous key
/// interval `[prefix, prefix++)` and is compacted as an independent logical
/// LSM tree. Segments share the manifest-level WAL state and SST identity
/// counter with the unsegmented tree.
#[derive(Clone, PartialEq, Serialize, Debug)]
pub struct Segment {
    /// The segment's key prefix.
    pub(crate) prefix: Bytes,

    /// LSM state for this segment.
    pub(crate) tree: Arc<LsmTreeState>,
}

impl Segment {
    /// The segment's key prefix.
    pub fn prefix(&self) -> &Bytes {
        &self.prefix
    }

    /// The L0 SST views in this segment.
    pub fn l0(&self) -> &VecDeque<SsTableView> {
        &self.tree.l0
    }

    /// The compacted sorted runs in this segment.
    pub fn compacted(&self) -> &Vec<SortedRun> {
        &self.tree.compacted
    }

    /// The last compacted L0 SST view ID, if any.
    pub fn last_compacted_l0_sst_view_id(&self) -> Option<ulid::Ulid> {
        self.tree.last_compacted_l0_sst_view_id
    }

    /// The last compacted L0 SST ID, if any.
    pub fn last_compacted_l0_sst_id(&self) -> Option<ulid::Ulid> {
        self.tree.last_compacted_l0_sst_id
    }
}

/// Compactor-side segment merge: combine the writer's segments (`writer`)
/// into the compactor's segments (`local`).
///
/// The compactor never prunes its own drain markers — it preserves them
/// until the writer has observed the marker and pruned its own copy. The
/// only segment-removal action the compactor takes is to *follow* a writer
/// prune: when the compactor's local has a marker for a prefix that the
/// writer's manifest no longer carries, that absence is the writer's
/// signal that the marker has been observed and the segment can be dropped.
///
/// Both inputs are required to be sorted by `prefix`, and the output is
/// sorted by `prefix` (see [`ManifestCore::segments`]). The walk is a
/// linear two-cursor merge.
pub(crate) fn merge_segments_from_writer(local: &[Segment], writer: &[Segment]) -> Vec<Segment> {
    debug_assert!(
        is_sorted_by_prefix(writer),
        "writer segments must be sorted"
    );
    debug_assert!(is_sorted_by_prefix(local), "local segments must be sorted");
    let empty = LsmTreeState::default();
    let mut merged: Vec<Segment> = Vec::with_capacity(writer.len() + local.len());
    for step in MergeIter::new(writer, local) {
        match step {
            MergeStep::WriterOnly(w) => {
                // Compactor hasn't seen this prefix yet (newly-flushed).
                let tree = LsmTreeState::merge_writer_and_compactor(&w.tree, &empty);
                merged.push(Segment {
                    prefix: w.prefix.clone(),
                    tree: Arc::new(tree),
                });
            }
            MergeStep::CompactorOnly(c) => {
                // Expected: writer has pruned this prefix — drop to follow.
                // Any tree with data here is a protocol violation: the
                // compactor never produces data the writer hasn't seen.
                if !c.tree.is_empty() {
                    unreachable!(
                        "compactor-only segment with data: prefix={:?} tree={:?}",
                        c.prefix, c.tree
                    );
                }
            }
            MergeStep::Both(w, c) => {
                // Kernel merge. The compactor never prunes — any empty
                // result (drain marker or otherwise) flows through and is
                // pruned by the writer.
                let tree = LsmTreeState::merge_writer_and_compactor(&w.tree, &c.tree);
                merged.push(Segment {
                    prefix: w.prefix.clone(),
                    tree: Arc::new(tree),
                });
            }
        }
    }
    merged
}

/// Writer-side segment merge: combine the compactor's segments (`compactor`)
/// into the writer's segments (`local`).
///
/// The writer is the sole pruner. After the kernel per-tree merge, if the
/// result is a drain marker (no L0 above the watermark, no compacted runs,
/// watermark set) the writer drops the segment from its commit. The
/// compactor will observe the absence on its next read and follow.
///
/// Both inputs are required to be sorted by `prefix`, and the output is
/// sorted by `prefix`. The walk is a linear two-cursor merge.
pub(crate) fn merge_segments_from_compactor(
    local: &[Segment],
    compactor: &[Segment],
) -> Vec<Segment> {
    debug_assert!(is_sorted_by_prefix(local), "local segments must be sorted");
    debug_assert!(
        is_sorted_by_prefix(compactor),
        "compactor segments must be sorted"
    );
    let empty = LsmTreeState::default();
    let mut merged: Vec<Segment> = Vec::with_capacity(local.len() + compactor.len());
    for step in MergeIter::new(local, compactor) {
        let (prefix, tree) = match step {
            MergeStep::WriterOnly(w) => (
                w.prefix.clone(),
                LsmTreeState::merge_writer_and_compactor(&w.tree, &empty),
            ),
            MergeStep::CompactorOnly(c) => (
                c.prefix.clone(),
                LsmTreeState::merge_writer_and_compactor(&empty, &c.tree),
            ),
            MergeStep::Both(w, c) => (
                w.prefix.clone(),
                LsmTreeState::merge_writer_and_compactor(&w.tree, &c.tree),
            ),
        };
        // Writer is the sole pruner: drop any tree with no L0 and no SR.
        // The watermark signal, if any, has already been applied by the
        // `merge_writer_and_compactor` call above.
        if !tree.is_empty() {
            merged.push(Segment {
                prefix,
                tree: Arc::new(tree),
            });
        }
    }
    merged
}

/// One step of a linear two-cursor merge over a pair of sorted-by-prefix
/// segment slices. The first slice is interpreted as the writer's view and
/// the second as the compactor's view, advancing whichever cursor has the
/// smaller current prefix (or both, on a tie).
enum MergeStep<'a> {
    WriterOnly(&'a Segment),
    CompactorOnly(&'a Segment),
    Both(&'a Segment, &'a Segment),
}

/// Iterator over a sorted-by-prefix segment merge. Yields a [`MergeStep`]
/// for each prefix that appears in either input, in sorted order.
struct MergeIter<'a> {
    writer: &'a [Segment],
    compactor: &'a [Segment],
    i: usize,
    j: usize,
}

impl<'a> MergeIter<'a> {
    fn new(writer: &'a [Segment], compactor: &'a [Segment]) -> Self {
        Self {
            writer,
            compactor,
            i: 0,
            j: 0,
        }
    }
}

impl<'a> Iterator for MergeIter<'a> {
    type Item = MergeStep<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let step = match (self.writer.get(self.i), self.compactor.get(self.j)) {
            (None, None) => return None,
            (Some(w), None) => MergeStep::WriterOnly(w),
            (None, Some(c)) => MergeStep::CompactorOnly(c),
            (Some(w), Some(c)) => match w.prefix.cmp(&c.prefix) {
                Ordering::Less => MergeStep::WriterOnly(w),
                Ordering::Greater => MergeStep::CompactorOnly(c),
                Ordering::Equal => MergeStep::Both(w, c),
            },
        };
        match &step {
            MergeStep::WriterOnly(_) => self.i += 1,
            MergeStep::CompactorOnly(_) => self.j += 1,
            MergeStep::Both(_, _) => {
                self.i += 1;
                self.j += 1;
            }
        }
        Some(step)
    }
}

fn is_sorted_by_prefix(segments: &[Segment]) -> bool {
    segments.windows(2).all(|w| w[0].prefix < w[1].prefix)
}

/// Internal immutable in-memory view of a `.manifest` file.
#[derive(Clone, PartialEq, Serialize, Debug)]
pub(crate) struct ManifestCore {
    /// Flag to indicate whether initialization has finished. When creating the initial manifest for
    /// a root db (one that is not a clone), this flag will be set to true. When creating the initial
    /// manifest for a clone db, this flag will be set to false and then updated to true once clone
    /// initialization has completed.
    pub initialized: bool,

    /// LSM state for data that is not associated with any named segment. When
    /// segmentation is not configured this is the only tree; otherwise it sits
    /// alongside the named segments in `segments`.
    #[serde(flatten)]
    pub tree: Arc<LsmTreeState>,

    /// Per-segment LSM state (RFC-0024). Empty when no segment extractor is
    /// configured. Each segment carries the LSM state for the keys whose
    /// extracted prefix matches the segment's `prefix`.
    ///
    /// Invariant: sorted strictly ascending by `prefix`. Mutation sites
    /// (FlatBuffer decode, merge functions, future writer-flush insertion)
    /// must preserve this ordering. Lookup-by-prefix should use
    /// `binary_search_by_key`; range queries use `partition_point`.
    pub segments: Vec<Segment>,

    /// Name of the configured segment extractor (RFC-0024). Persisted so the
    /// writer can detect accidental reconfiguration on startup. `None` when
    /// no extractor is configured.
    pub segment_extractor_name: Option<String>,

    /// The next WAL SST ID to be assigned when creating a new WAL SST. The manifest FlatBuffer
    /// contains `wal_id_last_seen`, which is always one less than this value.
    pub next_wal_sst_id: u64,

    /// the WAL ID after which the WAL replay should start. Default to 0,
    /// which means all the WAL IDs should be greater than or equal to 1.
    /// When a new L0 is flushed, we update this field to the recent
    /// flushed WAL ID.
    pub replay_after_wal_id: u64,

    /// the `last_l0_clock_tick` includes all data in L0 and below --
    /// WAL entries will have their latest ticks recovered on replay
    /// into the in-memory state.
    pub last_l0_clock_tick: i64,

    /// it's persisted in the manifest, and only updated when a new L0
    /// SST is created in the manifest.
    pub last_l0_seq: u64,

    /// Minimum sequence number across all recent in-memory snapshots. The compactor
    /// needs this to determine whether it's safe to drop duplicate key writes. If a
    /// recent snapshot still references an older version of a key, it should not be
    /// recycled. This field is updated when a new L0 is flushed.
    pub recent_snapshot_min_seq: u64,

    /// A sequence tracker that maps sequence numbers to timestamps as defined in
    /// RFC-0012.
    pub sequence_tracker: SequenceTracker,

    /// A list of checkpoints that are currently open.
    pub checkpoints: Vec<Checkpoint>,

    /// The URI of the object store dedicated specifically for WAL, if any.
    pub wal_object_store_uri: Option<String>,
}

impl ManifestCore {
    pub(crate) fn new() -> Self {
        Self {
            initialized: true,
            tree: Arc::new(LsmTreeState::default()),
            segments: vec![],
            segment_extractor_name: None,
            next_wal_sst_id: 1,
            replay_after_wal_id: 0,
            last_l0_clock_tick: i64::MIN,
            last_l0_seq: 0,
            checkpoints: vec![],
            wal_object_store_uri: None,
            recent_snapshot_min_seq: 0,
            sequence_tracker: SequenceTracker::new(),
        }
    }

    pub(crate) fn new_with_wal_object_store(wal_object_store_uri: Option<String>) -> Self {
        let mut this = Self::new();
        this.wal_object_store_uri = wal_object_store_uri;
        this
    }

    /// Iterate all per-tree LSM states: the unsegmented `tree` followed by
    /// each named segment's `tree` (RFC-0024).
    pub(crate) fn trees(&self) -> impl Iterator<Item = &LsmTreeState> {
        std::iter::once(self.tree.as_ref()).chain(self.segments.iter().map(|s| s.tree.as_ref()))
    }

    /// Iterate every tree paired with its segment prefix: the empty-prefix
    /// root tree (RFC-0024 compatibility encoding) first, then each named
    /// segment in `segments` order. Use [`trees`] when only the LSM state
    /// matters.
    pub(crate) fn trees_with_prefix(&self) -> impl Iterator<Item = (Bytes, &LsmTreeState)> {
        std::iter::once((Bytes::new(), self.tree.as_ref())).chain(
            self.segments
                .iter()
                .map(|s| (s.prefix.clone(), s.tree.as_ref())),
        )
    }

    /// Look up the LSM tree for a given segment prefix. An empty `prefix`
    /// returns the root tree (compatibility-encoded `prefix=""` segment);
    /// a non-empty prefix returns the named segment's tree, or `None` if no
    /// segment with that prefix exists.
    pub(crate) fn tree_for_segment(&self, prefix: &[u8]) -> Option<&LsmTreeState> {
        if prefix.is_empty() {
            Some(self.tree.as_ref())
        } else {
            self.segments
                .binary_search_by(|s| s.prefix.as_ref().cmp(prefix))
                .ok()
                .map(|idx| self.segments[idx].tree.as_ref())
        }
    }

    /// Return a mutable reference to the LSM tree for the given segment
    /// prefix.  An empty `prefix` addresses the root tree; a non-empty
    /// prefix addresses a named segment.  Returns `None` when no segment
    /// with that prefix exists.
    pub(crate) fn tree_for_segment_mut(&mut self, prefix: &[u8]) -> Option<&mut LsmTreeState> {
        if prefix.is_empty() {
            Some(Arc::make_mut(&mut self.tree))
        } else {
            let idx = self
                .segments
                .binary_search_by(|s| s.prefix.as_ref().cmp(prefix))
                .ok()?;
            Some(Arc::make_mut(&mut self.segments[idx].tree))
        }
    }

    /// The max L0 ULID watermark across all trees (the unsegmented root and
    /// every segment), or `None` for a fresh manifest with no L0 history.
    ///
    /// This mirrors the GC's `newest_l0_dt` (see
    /// `garbage_collector::compacted_gc`) exactly, so the cutoff invariant
    /// compares against the same watermark the GC uses to delete compacted SSTs.
    /// Per tree: if the tree has active L0s, take the max of their physical SST
    /// ULIDs (`view.sst.id.value()` — the ID the GC keys its cutoff
    /// and deletion filter off, *not* the separately-allocated `view.id`);
    /// otherwise fall back to that tree's `last_compacted_l0_sst_view_id` marker.
    /// The result is the max of those per-tree values across the unsegmented root
    /// and every segment. Returned as a `Ulid` (not a raw timestamp) so the L0
    /// cutoff invariant ([`l0_ulid_cutoff`](crate::manifest::invariants::l0_ulid_cutoff))
    /// and the open-time skew check share one definition of the watermark.
    pub(crate) fn max_l0_ulid_timestamp_across_trees(&self) -> Option<ulid::Ulid> {
        self.trees()
            .filter_map(|tree| {
                if tree.l0.is_empty() {
                    // No active L0s: fall back to the last-compacted marker, as
                    // the GC does. (The GC uses the *view* id here; mirror it.)
                    tree.last_compacted_l0_sst_view_id
                } else {
                    tree.l0.iter().map(|view| view.sst.id.value()).max()
                }
            })
            .max()
    }

    /// Iterate every SST view referenced by this manifest — L0 views and
    /// sorted-run views across the unsegmented tree and every segment.
    pub(crate) fn all_sst_views(&self) -> impl Iterator<Item = &SsTableView> {
        self.trees().flat_map(|tree| tree.sst_views())
    }

    /// Compare a configured WAL-store URI against the manifest's
    /// persisted value. `ManifestV2` drops `wal_object_store_uri`
    /// entirely (commit 52cead43, PR #1473) and always decodes it as
    /// `None`, so the check is a no-op for V2 manifests; on `ManifestV1`
    /// it enforces that the user did not silently swap a separate WAL
    /// store on or off across opens.
    pub(crate) fn validate_wal_object_store_uri(
        &self,
        configured: Option<&str>,
    ) -> Result<(), SlateDBError> {
        if let Some(persisted) = self.wal_object_store_uri.as_deref() {
            if Some(persisted) != configured {
                return Err(SlateDBError::WalStoreReconfigurationError);
            }
        }
        Ok(())
    }

    /// RFC-0024 open-time reconciliation. Compares the configured
    /// extractor against the manifest's persisted state and rejects any
    /// reconfiguration that could route data inconsistently.
    ///
    /// Rules:
    ///
    /// - both `None` → fine.
    /// - both `Some` and `name()` matches → fine; additionally every
    ///   existing segment prefix must satisfy
    ///   `prefix_len(Prefix(p)) == Some(p.len())` so the current
    ///   extractor still claims the persisted prefixes.
    /// - persisted `Some`, configured `None` → reject (extractor was
    ///   removed; existing data is no longer addressable).
    /// - persisted `None`, configured `Some` → reject (extractor was
    ///   added to an existing database; pre-existing keys would route
    ///   wrong). Caller is responsible for skipping this call when
    ///   creating a fresh manifest.
    /// - both `Some` but `name()` differs → reject.
    pub(crate) fn validate_extractor_configuration(
        &self,
        configured: Option<&dyn crate::prefix_extractor::PrefixExtractor>,
    ) -> Result<(), SlateDBError> {
        let persisted = self.segment_extractor_name.as_deref();
        let configured_name = configured.map(|e| e.name());
        match (persisted, configured_name) {
            (None, None) => Ok(()),
            (Some(p), Some(c)) if p == c => self.validate_segment_prefixes_recognized(
                configured.expect("configured extractor present"),
            ),
            _ => Err(SlateDBError::SegmentExtractorMismatch {
                persisted: persisted.map(str::to_string),
                configured: configured_name.map(str::to_string),
            }),
        }
    }

    /// For each entry in `segments`, verify that `extractor` recognizes
    /// the prefix as a complete segment boundary
    /// (`prefix_len(Prefix(p)) == Some(p.len())`). A failure means the
    /// extractor's logic has shifted — the same `name()`, but it would
    /// no longer assign `p`-keys to the segment they currently live in.
    fn validate_segment_prefixes_recognized(
        &self,
        extractor: &dyn crate::prefix_extractor::PrefixExtractor,
    ) -> Result<(), SlateDBError> {
        for segment in &self.segments {
            let n = extractor.prefix_len(&crate::prefix_extractor::PrefixTarget::Prefix(
                segment.prefix.clone(),
            ));
            if n != Some(segment.prefix.len()) {
                return Err(SlateDBError::SegmentPrefixNotRecognized {
                    prefix: segment.prefix.clone(),
                    extractor: extractor.name().to_string(),
                });
            }
        }
        Ok(())
    }

    /// Verify that `candidate` could join the segment set without violating
    /// the antichain invariant. Returns `Ok` if `candidate` matches an
    /// existing segment exactly or has no nesting neighbor in `segments`;
    /// otherwise returns [`SlateDBError::InvalidSegmentPrefix`].
    ///
    /// Used at write time (RFC-0024 route-consistency) and from
    /// [`Self::maybe_insert_tree`]. Cheap — one binary search plus two
    /// `starts_with` comparisons against the sort-order neighbors.
    pub(crate) fn check_segment_prefix_antichain(
        &self,
        candidate: &[u8],
    ) -> Result<(), SlateDBError> {
        match self
            .segments
            .binary_search_by(|s| s.prefix.as_ref().cmp(candidate))
        {
            Ok(_) => Ok(()),
            Err(idx) => {
                // Sort-order neighbors are the only candidates for nesting:
                // by induction the existing list is already an antichain, so
                // any nest with `candidate` must involve an element
                // immediately less than or greater than it.
                if let Some(succ) = self.segments.get(idx) {
                    // succ.prefix > candidate; nested iff succ extends candidate.
                    if succ.prefix.starts_with(candidate) {
                        return Err(SlateDBError::InvalidSegmentPrefix {
                            prefix: Bytes::copy_from_slice(candidate),
                            conflict: succ.prefix.clone(),
                        });
                    }
                }
                if idx > 0 {
                    let pred = &self.segments[idx - 1];
                    // pred.prefix < candidate; nested iff candidate extends pred.
                    if candidate.starts_with(pred.prefix.as_ref()) {
                        return Err(SlateDBError::InvalidSegmentPrefix {
                            prefix: Bytes::copy_from_slice(candidate),
                            conflict: pred.prefix.clone(),
                        });
                    }
                }
                Ok(())
            }
        }
    }

    /// Return a mutable reference to the LSM tree of the segment with the
    /// given `prefix`, inserting an empty entry at the sorted position if
    /// the segment does not yet exist. Preserves the `segments` ascending-
    /// prefix invariant and rejects any insertion that would violate the
    /// antichain invariant against an existing neighbor.
    pub(crate) fn maybe_insert_tree(
        &mut self,
        prefix: &Bytes,
    ) -> Result<&mut LsmTreeState, SlateDBError> {
        match self.segments.binary_search_by(|s| s.prefix.cmp(prefix)) {
            Ok(idx) => Ok(Arc::make_mut(&mut self.segments[idx].tree)),
            Err(idx) => {
                self.check_segment_prefix_antichain(prefix.as_ref())?;
                self.segments.insert(
                    idx,
                    Segment {
                        prefix: prefix.clone(),
                        tree: Arc::new(LsmTreeState::default()),
                    },
                );
                Ok(Arc::make_mut(&mut self.segments[idx].tree))
            }
        }
    }

    pub(crate) fn init_clone_db(&self) -> ManifestCore {
        let mut clone = self.clone();
        clone.initialized = false;
        clone.checkpoints.clear();
        clone
    }

    pub(crate) fn log_db_runs(&self) {
        let l0s: Vec<_> = self.tree.l0.iter().map(|l0| l0.estimate_size()).collect();
        let compacted: Vec<_> = self
            .tree
            .compacted
            .iter()
            .map(|sr| (sr.id, sr.estimate_size()))
            .collect();
        debug!("DB Levels:");
        debug!("-----------------");
        debug!("{:?}", l0s);
        debug!("{:?}", compacted);
        debug!("-----------------");
    }

    pub(crate) fn find_checkpoint(&self, checkpoint_id: Uuid) -> Option<&Checkpoint> {
        self.checkpoints.iter().find(|c| c.id == checkpoint_id)
    }

    /// Returns the segments whose intervals may contain entries in
    /// `range`, or `None` when the database is unconfigured (no
    /// segments).  Used by the read path to route a query to the
    /// relevant tree(s) and by `SegmentRangeIterator` to
    /// binary-search seek across the chain.
    ///
    /// In an extractor-configured database (mandatory full
    /// segmentation: `tree` empty, `segments` non-empty), returns
    /// every segment whose `[prefix, prefix++)` interval overlaps
    /// `range`, in prefix-ascending order.  Because segments are
    /// pairwise disjoint and sorted, the matching set is a contiguous
    /// slice of `segments` located via binary search on the query
    /// bounds.
    ///
    /// A point query that lands outside every segment's interval —
    /// and any query against a configured-but-empty database —
    /// returns an empty slice: no tree can hold a matching key.
    ///
    /// When `None` is returned, callers should use
    /// [`default_segment`](Self::default_segment) to obtain the
    /// single unsegmented tree.
    pub(crate) fn select_segments(&self, range: &BytesRange) -> Option<&[Segment]> {
        if self.segments.is_empty() {
            return None;
        }
        let indices = self.overlapping_segment_indices(range);
        Some(&self.segments[indices])
    }

    /// Returns the unsegmented tree wrapped in a [`Segment`] with an
    /// empty prefix.  This is the fallback for unconfigured databases
    /// where [`select_segments`](Self::select_segments) returns `None`.
    ///
    /// The empty prefix's interval `[b"", +∞)` trivially covers every
    /// key, which is what unsegmented mode requires.
    pub(crate) fn default_segment(&self) -> Segment {
        Segment {
            prefix: Bytes::new(),
            tree: self.tree.clone(),
        }
    }

    /// Half-open `[start, end)` index range of segments whose intervals
    /// overlap `range`. Each side is computed with a single
    /// `partition_point`; combined with the antichain invariant this
    /// pinpoints the contiguous overlap window in O(log n).
    fn overlapping_segment_indices(&self, range: &BytesRange) -> std::ops::Range<usize> {
        let start = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(lo) | Bound::Excluded(lo) => {
                let lo = lo.as_ref();
                let after = self.segments.partition_point(|s| s.prefix.as_ref() <= lo);
                // The largest prefix `<= lo` (if any) is the unique
                // candidate that could contain `lo` (antichain
                // invariant). If `lo` starts with that prefix, the
                // segment overlaps the start of the query. Otherwise the
                // segment's interval ends before `lo`, so the first
                // overlapping segment (if any) sits at `after`.
                if after > 0 && lo.starts_with(&self.segments[after - 1].prefix) {
                    after - 1
                } else {
                    after
                }
            }
        };
        let end = match range.end_bound() {
            Bound::Unbounded => self.segments.len(),
            Bound::Excluded(hi) => self
                .segments
                .partition_point(|s| s.prefix.as_ref() < hi.as_ref()),
            Bound::Included(hi) => self
                .segments
                .partition_point(|s| s.prefix.as_ref() <= hi.as_ref()),
        };
        if start < end {
            start..end
        } else {
            0..0
        }
    }
}

#[derive(Clone, Serialize, PartialEq, Debug)]
pub(crate) struct Manifest {
    // todo: try to make this writable only from module
    pub(crate) external_dbs: Vec<ExternalDb>,
    #[serde(flatten)]
    pub(crate) core: ManifestCore,
    // todo: try to make this writable only from module
    pub(crate) writer_epoch: u64,
    pub(crate) compactor_epoch: u64,
}

/// A manifest snapshot paired with its version ID for monotonic ordering.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VersionedManifest {
    /// The version ID of the manifest.
    pub(crate) id: u64,
    /// The flattened manifest state at this version.
    #[serde(flatten)]
    pub(crate) manifest: Manifest,
}

impl VersionedManifest {
    pub(crate) fn from_manifest(id: u64, manifest: Manifest) -> Self {
        Self { id, manifest }
    }

    /// Returns the manifest version ID.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the writer epoch recorded in this manifest snapshot.
    pub fn writer_epoch(&self) -> u64 {
        self.manifest.writer_epoch
    }

    /// Returns the compactor epoch recorded in this manifest snapshot.
    pub fn compactor_epoch(&self) -> u64 {
        self.manifest.compactor_epoch
    }

    /// Returns the external DB references recorded in this manifest snapshot.
    pub fn external_dbs(&self) -> &Vec<ExternalDb> {
        &self.manifest.external_dbs
    }

    /// Returns whether initialization has completed.
    pub fn initialized(&self) -> bool {
        self.manifest.core.initialized
    }

    /// Checks the data trees of an unopened projected clone against its retained
    /// source snapshot. This is an identity check before writer activation, not
    /// a general equivalence test after compaction or application writes.
    /// Ownership, checkpoint lineage and initialization must be checked separately.
    pub fn matches_projected_data_from<R: RangeBounds<Bytes> + Clone>(
        &self,
        source: &Self,
        range: R,
    ) -> Result<bool, crate::Error> {
        let expected = Manifest::projected(
            &source.manifest,
            &ProjectionConfig {
                global_range: Some(BytesRange::from(range)),
                segment_filter: None,
                segment_projection: None,
            },
        )?;
        Ok(self.manifest.core.tree == expected.core.tree
            && self.manifest.core.segments == expected.core.segments
            && self.manifest.core.last_l0_seq == expected.core.last_l0_seq)
    }

    /// Returns the last compacted L0 SST view ID, if any.
    pub fn last_compacted_l0_sst_view_id(&self) -> Option<ulid::Ulid> {
        self.manifest.core.tree.last_compacted_l0_sst_view_id
    }

    /// Returns the last compacted L0 SST ID, if any.
    pub fn last_compacted_l0_sst_id(&self) -> Option<ulid::Ulid> {
        self.manifest.core.tree.last_compacted_l0_sst_id
    }

    /// Returns the current L0 SST views.
    pub fn l0(&self) -> &VecDeque<SsTableView> {
        &self.manifest.core.tree.l0
    }

    /// Returns the current compacted sorted runs.
    pub fn compacted(&self) -> &Vec<SortedRun> {
        &self.manifest.core.tree.compacted
    }

    /// Returns the next WAL SST ID to assign.
    pub fn next_wal_sst_id(&self) -> u64 {
        self.manifest.core.next_wal_sst_id
    }

    /// Returns the WAL replay watermark.
    pub fn replay_after_wal_id(&self) -> u64 {
        self.manifest.core.replay_after_wal_id
    }

    /// Returns the last persisted L0 clock tick.
    pub fn last_l0_clock_tick(&self) -> i64 {
        self.manifest.core.last_l0_clock_tick
    }

    /// Returns the last persisted L0 sequence number.
    pub fn last_l0_seq(&self) -> u64 {
        self.manifest.core.last_l0_seq
    }

    /// Returns the minimum sequence number still visible to recent snapshots.
    pub fn recent_snapshot_min_seq(&self) -> u64 {
        self.manifest.core.recent_snapshot_min_seq
    }

    /// Returns the persisted sequence tracker.
    pub fn sequence_tracker(&self) -> &SequenceTracker {
        &self.manifest.core.sequence_tracker
    }

    /// Returns the checkpoints tracked by this manifest snapshot.
    pub fn checkpoints(&self) -> &Vec<Checkpoint> {
        &self.manifest.core.checkpoints
    }

    /// Returns the dedicated WAL object store URI, if any.
    pub fn wal_object_store_uri(&self) -> Option<&str> {
        self.manifest.core.wal_object_store_uri.as_deref()
    }

    /// Returns the persisted segment extractor name (RFC-0024), if any.
    /// `None` means the database was created without a segment extractor.
    pub fn segment_extractor_name(&self) -> Option<&str> {
        self.manifest.core.segment_extractor_name.as_deref()
    }

    /// Every SST view in this manifest, across the root tree and every
    /// segment.
    pub fn all_sst_views(&self) -> impl Iterator<Item = &SsTableView> {
        self.manifest.core.all_sst_views()
    }

    pub(crate) fn core(&self) -> &ManifestCore {
        &self.manifest.core
    }

    pub(crate) fn external_ssts(&self) -> HashMap<SsTableId, object_store::path::Path> {
        self.manifest.external_ssts()
    }

    /// The named segments configured in this manifest (RFC-0024), in prefix
    /// order. Empty when no segment extractor is configured. The unsegmented
    /// default tree is accessed via [`Self::l0`] / [`Self::compacted`] /
    /// [`Self::last_compacted_l0_sst_view_id`] / [`Self::last_compacted_l0_sst_id`];
    /// when segments are configured the default tree is empty by construction
    /// (writes route only to segments, and the extractor cannot return an
    /// empty prefix).
    pub fn segments(&self) -> &[Segment] {
        &self.manifest.core.segments
    }

    /// Look up a single named segment by exact prefix match. Returns `None`
    /// if no segment with that prefix exists.
    pub fn segment(&self, prefix: &[u8]) -> Option<&Segment> {
        let idx = self
            .manifest
            .core
            .segments
            .binary_search_by(|s| s.prefix.as_ref().cmp(prefix))
            .ok()?;
        Some(&self.manifest.core.segments[idx])
    }
}

impl From<DirtyObject<Manifest>> for VersionedManifest {
    fn from(dirty: DirtyObject<Manifest>) -> Self {
        Self::from_manifest(dirty.id.id(), dirty.value)
    }
}

impl Manifest {
    pub(crate) fn initial(core: ManifestCore) -> Self {
        Self {
            external_dbs: vec![],
            core,
            writer_epoch: 0,
            compactor_epoch: 0,
        }
    }

    /// Create an initial manifest for a new clone. The returned
    /// manifest will set `initialized=false` to allow for additional
    /// initialization (such as copying wals).
    pub(crate) fn cloned(
        parent_manifest: &Manifest,
        parent_path: String,
        source_checkpoint_id: Uuid,
        rand: Arc<DbRand>,
    ) -> Self {
        let mut clone_external_dbs = vec![];

        // Carry over each inherited external_db that still holds SSTs, with a fresh
        // final_checkpoint_id. An entry whose SSTs were all re-localized contributes nothing,
        // and the detach collector may already have released the checkpoint it pinned.
        for parent_external_db in parent_manifest
            .external_dbs
            .iter()
            .filter(|external_db| !external_db.sst_ids.is_empty())
        {
            clone_external_dbs.push(ExternalDb {
                path: parent_external_db.path.clone(),
                // don't depend on the original source_checkpoint: it was supplied by the user and
                // might have been deleted; use slatedb-generated final checkpoint instead
                source_checkpoint_id: parent_external_db
                    .final_checkpoint_id
                    .expect("final_checkpoint_id must be present"),
                final_checkpoint_id: Some(rand.rng().gen_uuid()),
                sst_ids: parent_external_db.sst_ids.clone(),
            });
        }

        // Add a single external_db pointing at the parent for everything the
        // parent owns directly (across all trees, including segments).
        clone_external_dbs.push(ExternalDb {
            path: parent_path,
            source_checkpoint_id,
            final_checkpoint_id: Some(rand.rng().gen_uuid()),
            sst_ids: parent_manifest.owned_ssts(),
        });

        Self {
            external_dbs: clone_external_dbs,
            core: parent_manifest.core.init_clone_db(),
            writer_epoch: parent_manifest.writer_epoch,
            compactor_epoch: parent_manifest.compactor_epoch,
        }
    }

    /// Apply a `ProjectionConfig` to `source_manifest`, returning a new
    /// manifest with each tree narrowed to the effective range derived from the
    /// config. The unsegmented `core.tree` participates as a logical segment
    /// with the empty prefix.
    ///
    /// Errors with `SlateDBError::InvalidProjection` when a closure-supplied
    /// range falls outside the segment's `[prefix, prefix++)` bounds.
    pub(crate) fn projected(
        source_manifest: &Manifest,
        config: &ProjectionConfig,
    ) -> Result<Manifest, SlateDBError> {
        if config.is_noop() {
            return Ok(source_manifest.clone());
        }
        let mut projected = source_manifest.clone();

        // The unsegmented tree is treated as a logical segment with empty
        // prefix. Filter+project it through the same path as named segments.
        match Self::resolve_segment_action(b"", config)? {
            SegmentAction::Drop => projected.core.tree = Arc::new(LsmTreeState::default()),
            SegmentAction::Project(range) => {
                Self::project_tree_in_place(Arc::make_mut(&mut projected.core.tree), &range)
            }
            SegmentAction::PassThrough => {}
        }

        // For each named segment, resolve the action. If the segment is
        // filtered out, drop it. Otherwise project in place (or leave as-is
        // if no range applies) and drop segments that become empty.
        //
        // The projector is acting as the first writer of the resulting
        // clone, so segments are derived from data: any source-side drain
        // marker carries no meaning in the new DB and is dropped along
        // with the segment.
        let segments = std::mem::take(&mut projected.core.segments);
        let mut kept: Vec<Segment> = Vec::with_capacity(segments.len());
        for mut segment in segments {
            match Self::resolve_segment_action(&segment.prefix, config)? {
                SegmentAction::Drop => { /* filtered out */ }
                SegmentAction::Project(range) => {
                    Self::project_tree_in_place(Arc::make_mut(&mut segment.tree), &range);
                    if !segment.tree.l0.is_empty() || !segment.tree.compacted.is_empty() {
                        kept.push(segment);
                    }
                }
                SegmentAction::PassThrough => kept.push(segment),
            }
        }
        projected.core.segments = kept;

        projected.prune_external_sst_ids();
        projected.external_dbs.retain(|e| !e.sst_ids.is_empty());
        Ok(projected)
    }

    /// Decide what to do with the tree at `prefix` given the projection config.
    /// `Drop` means the filter rejected the segment; `Project` carries the
    /// effective range to apply; `PassThrough` means no projection applies and
    /// the tree is kept verbatim.
    fn resolve_segment_action(
        prefix: &[u8],
        config: &ProjectionConfig,
    ) -> Result<SegmentAction, SlateDBError> {
        if let Some(filter) = &config.segment_filter {
            if !filter(prefix) {
                return Ok(SegmentAction::Drop);
            }
        }
        let segment_range = if let Some(projection) = &config.segment_projection {
            Some(Self::canonicalize_segment_range(
                prefix,
                projection(prefix)?,
            )?)
        } else {
            None
        };
        let combined = match (segment_range, config.global_range.clone()) {
            (Some(s), Some(g)) => s.intersect(&g),
            (Some(s), None) => Some(s),
            (None, Some(g)) => Some(g),
            (None, None) => return Ok(SegmentAction::PassThrough),
        };
        match combined {
            Some(range) => Ok(SegmentAction::Project(range)),
            // Range constraints exist but produced an empty intersection:
            // drop everything in the tree.
            None => Ok(SegmentAction::Drop),
        }
    }

    /// Validate a closure-supplied range against `[prefix, prefix++)`,
    /// resolving `Unbounded` ends to the segment edges. Errors if the
    /// resolved range is not a subset of the segment.
    fn canonicalize_segment_range(
        prefix: &[u8],
        range: BytesRange,
    ) -> Result<BytesRange, SlateDBError> {
        let segment_range = BytesRange::from_prefix(prefix);
        let start = match range.start_bound() {
            Bound::Unbounded => segment_range.start_bound().cloned(),
            other => other.cloned(),
        };
        let end = match range.end_bound() {
            Bound::Unbounded => segment_range.end_bound().cloned(),
            other => other.cloned(),
        };
        let resolved =
            BytesRange::try_new(start, end).ok_or_else(|| SlateDBError::InvalidProjection {
                prefix: Bytes::copy_from_slice(prefix),
                reason: "empty range".into(),
            })?;
        // The resolved range must be a subset of the segment's keyspace:
        // (resolved ∩ segment) == resolved.
        let intersection = resolved.intersect(&segment_range);
        if intersection.as_ref() != Some(&resolved) {
            return Err(SlateDBError::InvalidProjection {
                prefix: Bytes::copy_from_slice(prefix),
                reason: format!("range {:?} is not contained in segment", resolved),
            });
        }
        Ok(resolved)
    }

    /// Filter `tree.l0` and `tree.compacted` views against `range` in place.
    /// Sorted runs that lose all views are removed. Watermark fields are
    /// untouched (the caller decides whether to keep them).
    fn project_tree_in_place(tree: &mut LsmTreeState, range: &BytesRange) {
        let l0: VecDeque<SsTableView> = Self::filter_view_handles(&tree.l0, true, range).into();
        let mut sorted_runs_filtered = vec![];
        for sr in &tree.compacted {
            let sst_views = Self::filter_view_handles(sr.sst_views().iter(), false, range);
            if !sst_views.is_empty() {
                sorted_runs_filtered.push(SortedRun::new(sr.id, sst_views));
            }
        }
        tree.l0 = l0;
        tree.compacted = sorted_runs_filtered;
    }

    fn filter_view_handles<'a, T>(
        views: T,
        views_overlap: bool,
        projection_range: &BytesRange,
    ) -> Vec<SsTableView>
    where
        T: IntoIterator<Item = &'a SsTableView>,
    {
        let mut iter = views.into_iter().peekable();
        let mut filtered_handles = vec![];
        while let Some(current_handle) = iter.next() {
            let next_handle = if views_overlap {
                None
            } else {
                iter.peek().copied()
            };
            if let Some(intersection) =
                current_handle.compacted_intersection(next_handle, projection_range)
            {
                // `compacted_intersection` bounds a sorted-run SST's coverage by
                // the next SST's start key, so `intersection` can fall into the
                // gap beyond this SST's physical keys. `try_with_visible_range`
                // drops the SST in that case instead of panicking on an empty
                // physical/visible intersection.
                if let Some(view) = current_handle.try_with_visible_range(intersection) {
                    filtered_handles.push(view);
                }
            }
        }
        filtered_handles
    }

    /// Return the `segment_extractor_name` shared by all sources, or
    /// `None` if every source has `None`. Per RFC-0024, all sources must
    /// agree exactly — either every source has `None` or every source
    /// has the same `Some(name)`. Mixed configurations are rejected
    /// because unsegmented data in a no-extractor source may match an
    /// extractor prefix from another source, and after the union a read
    /// for that key would route through the extractor to a segment that
    /// does not contain it.
    fn ensure_consistent_segment_extractor(
        sources: &[CloneSource],
    ) -> Result<Option<String>, SlateDBError> {
        let mut iter = sources.iter();
        let Some(first) = iter.next() else {
            return Ok(None);
        };
        let agreed = first.manifest.core.segment_extractor_name.as_ref();
        for source in iter {
            let cur = source.manifest.core.segment_extractor_name.as_ref();
            if agreed != cur {
                let extractors: Vec<Option<String>> = sources
                    .iter()
                    .map(|s| s.manifest.core.segment_extractor_name.clone())
                    .collect();
                return Err(SlateDBError::InvalidUnion(format!(
                    "clone sources disagree on segment extractor. extractors=`{:?}`",
                    extractors
                )));
            }
        }
        Ok(agreed.cloned())
    }

    /// Verify the antichain invariant on segment prefixes for a union: no
    /// prefix is a proper prefix of another. Defends against stale
    /// extractor-name matches where two sources happen to agree on
    /// `segment_extractor_name` but their persisted prefixes were
    /// produced by extractors of different lengths. Non-overlap of source
    /// ranges usually implies this, but the check is explicit per
    /// RFC-0024.
    ///
    /// Returns `Err` on violation.
    fn ensure_union_prefix_antichain<'a>(
        prefixes: impl IntoIterator<Item = &'a Bytes>,
    ) -> Result<(), SlateDBError> {
        let mut prefixes: Vec<&Bytes> = prefixes.into_iter().collect();
        prefixes.sort();
        for window in prefixes.windows(2) {
            let (a, b) = (window[0], window[1]);
            // After sort, a <= b. Prefixes are unique by construction
            // (collected via BTreeMap/HashMap dedup), so a < b strictly.
            // If b also starts with a, then a is a proper prefix of b.
            if b.starts_with(a) {
                return Err(SlateDBError::InvalidUnion(format!(
                    "segment prefixes are not an antichain: `{:?}` is a proper prefix of `{:?}`",
                    a, b
                )));
            }
        }
        Ok(())
    }

    /// No-extractor case. Concatenate every source's `core.tree` into
    /// the unioned `core.tree`. Rejects any source carrying segments —
    /// segments require a configured extractor. Watermarks are
    /// intentionally not carried over: the unioned manifest is a fresh
    /// DB that begins compaction tracking from scratch.
    fn build_unsegmented_lsm_state(
        core: &mut ManifestCore,
        sources: &[&CloneSource],
    ) -> Result<(), SlateDBError> {
        let stray_prefixes: Vec<Bytes> = sources
            .iter()
            .flat_map(|s| {
                s.manifest
                    .core
                    .segments
                    .iter()
                    .map(|seg| seg.prefix.clone())
            })
            .collect();
        if !stray_prefixes.is_empty() {
            return Err(SlateDBError::InvalidUnion(format!(
                "clone source has segments but no extractor is configured. prefixes=`{:?}`",
                stray_prefixes
            )));
        }
        for source in sources {
            let manifest = &source.manifest;
            Arc::make_mut(&mut core.tree)
                .l0
                .extend(manifest.core.tree.l0.iter().cloned());
            Arc::make_mut(&mut core.tree)
                .compacted
                .extend(manifest.core.tree.compacted.iter().cloned());
        }
        Ok(())
    }

    /// Extractor-configured case. Concatenate the per-source trees that
    /// [`Self::group_segments_for_union`] collected for each prefix into one
    /// tree per segment. Segments holding no SSTs are already absent from
    /// `segments`, so the unioned manifest carries no placeholders.
    /// Watermarks are intentionally not carried over: the unioned manifest is
    /// a fresh DB that begins compaction tracking from scratch.
    fn build_segmented_lsm_state(core: &mut ManifestCore, segments: SegmentContributors) {
        core.segments = segments
            .into_iter()
            .map(|(prefix, trees)| {
                let mut merged = LsmTreeState::default();
                for tree in trees {
                    merged.l0.extend(tree.l0.iter().cloned());
                    merged.compacted.extend(tree.compacted.iter().cloned());
                }
                Segment {
                    prefix,
                    tree: Arc::new(merged),
                }
            })
            .collect();
    }

    /// Collect the trees that each source contributes to each segment. Within
    /// a segment the trees are in key order, so the merged segment's `l0` and
    /// `compacted` lists come out sorted — the same thing the manifest-wide
    /// sort does for the unsegmented tree.
    ///
    /// Two things are rejected. If one segment prefix is a prefix of another,
    /// the two segments cover some of the same keys. If two sources hold the
    /// same keys inside one segment, the merged segment cannot say which
    /// source owns them.
    ///
    /// A segment with no SSTs contributes nothing, so it is left out.
    fn group_segments_for_union<'a>(
        sources: &[&'a CloneSource],
    ) -> Result<SegmentContributors<'a>, SlateDBError> {
        let all_prefixes: BTreeSet<&Bytes> = sources
            .iter()
            .flat_map(|source| source.manifest.core.segments.iter().map(|seg| &seg.prefix))
            .collect();
        Self::ensure_union_prefix_antichain(all_prefixes)?;

        let mut by_prefix: BTreeMap<Bytes, Vec<(BytesRange, &LsmTreeState)>> = BTreeMap::new();
        for source in sources {
            for segment in &source.manifest.core.segments {
                if let Some(range) = Self::bounding_range(segment.tree.sst_views()) {
                    by_prefix
                        .entry(segment.prefix.clone())
                        .or_default()
                        .push((range, segment.tree.as_ref()));
                }
            }
        }

        let mut grouped = SegmentContributors::new();
        for (prefix, mut members) in by_prefix {
            members.sort_by_key(|(range, _)| range.comparable_start_bound().cloned());
            let ranges: Vec<BytesRange> = members.iter().map(|(range, _)| range.clone()).collect();
            Self::ensure_disjoint_ranges(Some(&prefix), &ranges)?;
            grouped.insert(prefix, members.into_iter().map(|(_, tree)| tree).collect());
        }
        Ok(grouped)
    }

    /// Reject overlapping source ranges. `ranges` must be sorted by start
    /// bound. `segment` names the segment under check and is omitted from the
    /// message for the unsegmented tree.
    fn ensure_disjoint_ranges(
        segment: Option<&Bytes>,
        ranges: &[BytesRange],
    ) -> Result<(), SlateDBError> {
        for pair in ranges.windows(2) {
            if pair[1].intersect(&pair[0]).is_some() {
                let scope = match segment {
                    Some(prefix) => format!(" in segment `{:?}`", prefix),
                    None => String::new(),
                };
                return Err(SlateDBError::InvalidUnion(format!(
                    "clone sources have overlapping key ranges{}. ranges=`{:?}`",
                    scope, ranges
                )));
            }
        }
        Ok(())
    }

    /// Build the union's `external_dbs` list. Forwards every source's
    /// inherited `external_dbs` that still hold SSTs and adds one entry per
    /// source that owns SSTs directly. `final_checkpoint_id` is left as `None`; it is
    /// regenerated after the post-loop deduplication.
    fn build_external_dbs(sources: &[&CloneSource]) -> Vec<ExternalDb> {
        let mut external_dbs = vec![];
        for source in sources {
            let manifest = &source.manifest;
            for parent_external_db in manifest
                .external_dbs
                .iter()
                .filter(|external_db| !external_db.sst_ids.is_empty())
            {
                external_dbs.push(ExternalDb {
                    path: parent_external_db.path.clone(),
                    // don't depend on the original source_checkpoint: it was supplied by the user and
                    // might have been deleted; use slatedb-generated final checkpoint instead
                    source_checkpoint_id: parent_external_db
                        .final_checkpoint_id
                        .expect("final_checkpoint_id must be present"),
                    final_checkpoint_id: None,
                    sst_ids: parent_external_db.sst_ids.clone(),
                });
            }
            let owned_ssts = manifest.owned_ssts();
            if !owned_ssts.is_empty() {
                external_dbs.push(ExternalDb {
                    path: source.path.clone().into(),
                    source_checkpoint_id: source.checkpoint.id,
                    final_checkpoint_id: None,
                    sst_ids: owned_ssts,
                });
            }
        }
        external_dbs
    }

    /// Reassign every sorted run in `core` a fresh sequential id. The
    /// union concatenates per-source `compacted` lists across the
    /// unsegmented tree and every segment, so ids must be regenerated
    /// to avoid cross-source collisions. RFC-0024 requires SR ids to be
    /// globally unique across all trees.
    ///
    /// Ids are assigned in descending order of walk position so that
    /// within each tree, the first list entry gets the highest id and
    /// the last gets the lowest. This preserves the existing convention
    /// that `compacted` is sorted by descending id, mirroring the
    /// per-source state pre-union.
    fn renumber_union_sorted_runs(core: &mut ManifestCore) {
        let count: usize = core.tree.compacted.len()
            + core
                .segments
                .iter()
                .map(|s| s.tree.compacted.len())
                .sum::<usize>();
        let all_compacted = Arc::make_mut(&mut core.tree).compacted.iter_mut().chain(
            core.segments
                .iter_mut()
                .flat_map(|s| Arc::make_mut(&mut s.tree).compacted.iter_mut()),
        );
        for (idx, sr) in all_compacted.enumerate() {
            sr.id = (count - 1 - idx) as u32;
        }
    }

    pub(crate) fn cloned_from_union(
        sources: Vec<CloneSource>,
        rand: Arc<DbRand>,
    ) -> Result<Manifest, SlateDBError> {
        let mut ranges = vec![];
        for source in &sources {
            let range = source.manifest.range();
            if let Some(range) = range {
                ranges.push((source, range));
            } else {
                warn!("manifest has no SST files [manifest={:?}]", source.manifest);
            }
        }
        ranges.sort_by_key(|(_, range)| range.comparable_start_bound().cloned());

        let ordered_sources: Vec<&CloneSource> = ranges.iter().map(|(s, _)| *s).collect();
        let mut core = ManifestCore::new();
        core.segment_extractor_name = Self::ensure_consistent_segment_extractor(&sources)?;

        if core.segment_extractor_name.is_none() {
            let all: Vec<BytesRange> = ranges.iter().map(|(_, r)| (*r).clone()).collect();
            Self::ensure_disjoint_ranges(None, &all)?;
            Self::build_unsegmented_lsm_state(&mut core, &ordered_sources)?;
        } else {
            let segments = Self::group_segments_for_union(&ordered_sources)?;
            Self::build_segmented_lsm_state(&mut core, segments);
        }
        Self::renumber_union_sorted_runs(&mut core);

        for source in &sources {
            core.last_l0_seq = max(core.last_l0_seq, source.manifest.core.last_l0_seq);
            core.last_l0_clock_tick = max(
                core.last_l0_clock_tick,
                source.manifest.core.last_l0_clock_tick,
            );
        }

        // Coalesce borrows of the same physical ancestor, keyed on (path, sst_ids) rather than
        // source_checkpoint_id: the carried-forward checkpoint id is regenerated on every clone,
        // so keying on it would leave identical borrows from independently-created clones
        // (the real rescale topology) un-coalesced and external_dbs would grow without bound
        // across generations. Identical sst_ids mean the exact same physical SSTs, already
        // pinned by any member's checkpoint, so collapsing is safe. BTreeMap/BTreeSet keep
        // iteration order — and hence manifest bytes and UUID assignment — deterministic.
        let user_supplied_checkpoints: HashMap<String, Uuid> = ordered_sources
            .iter()
            .map(|source| (source.path.clone().into(), source.checkpoint.id))
            .collect();
        let external_dbs_merged = Self::build_external_dbs(&ordered_sources)
            .into_iter()
            .fold(
                BTreeMap::new(),
                |mut map: BTreeMap<(String, BTreeSet<SsTableId>), Uuid>, db| {
                    let sst_ids: BTreeSet<SsTableId> = db.sst_ids.into_iter().collect();
                    let user_supplied_cp = user_supplied_checkpoints.get(&db.path).copied();
                    map.entry((db.path, sst_ids))
                        .and_modify(|cp| match user_supplied_cp {
                            // The incoming entry carries the user-supplied checkpoint id from
                            // the clone source spec: it must win the group so clone retries can
                            // re-validate the spec against the stored manifest
                            // (validate_attached_to_external_db).
                            Some(user_cp) if user_cp == db.source_checkpoint_id => *cp = user_cp,
                            // The stored entry carries the user-supplied id: keep it.
                            Some(user_cp) if user_cp == *cp => {}
                            // Neither id is user-supplied (both are slatedb-generated): keep
                            // the smallest for a deterministic result.
                            _ => *cp = (*cp).min(db.source_checkpoint_id),
                        })
                        .or_insert(db.source_checkpoint_id);
                    map
                },
            )
            .into_iter()
            .map(|((path, sst_ids), source_checkpoint_id)| ExternalDb {
                path,
                source_checkpoint_id,
                final_checkpoint_id: Some(rand.rng().gen_uuid()),
                sst_ids: sst_ids.into_iter().collect(),
            })
            .collect();

        Ok(Self {
            external_dbs: external_dbs_merged,
            core,
            writer_epoch: 0,
            compactor_epoch: 0,
        })
    }

    fn range(&self) -> Option<BytesRange> {
        Self::bounding_range(self.core.all_sst_views())
    }

    /// Smallest range covering every view in `views`, or `None` when `views`
    /// is empty.
    fn bounding_range<'a>(views: impl Iterator<Item = &'a SsTableView>) -> Option<BytesRange> {
        let mut start_bound = None;
        let mut end_bound = None;
        for sst in views {
            let range = sst.compacted_effective_range();
            start_bound = start_bound
                .map(|b| min(b, range.comparable_start_bound()))
                .or_else(|| Some(range.comparable_start_bound()));
            end_bound = end_bound
                .map(|b| max(b, range.comparable_end_bound()))
                .or_else(|| Some(range.comparable_end_bound()));
        }
        match (start_bound, end_bound) {
            (Some(start), Some(end)) => {
                let start: Bound<&Bytes> = start.into();
                let end: Bound<&Bytes> = end.into();
                Some(BytesRange::new(start.cloned(), end.cloned()))
            }
            (_, _) => None,
        }
    }
}

#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct ExternalDb {
    pub path: String,
    pub source_checkpoint_id: Uuid,
    pub final_checkpoint_id: Option<Uuid>,
    pub sst_ids: Vec<SsTableId>,
}

impl Manifest {
    /// Returns a map from SST ID to the external DB path for all external SSTs.
    pub(crate) fn external_ssts(&self) -> HashMap<SsTableId, object_store::path::Path> {
        let mut external_ssts = HashMap::new();
        for external_db in &self.external_dbs {
            for id in &external_db.sst_ids {
                external_ssts.insert(*id, external_db.path.clone().into());
            }
        }
        external_ssts
    }

    pub(crate) fn owned_ssts(&self) -> Vec<SsTableId> {
        // SST IDs already tracked by the source's own external_dbs
        let source_external_sst_ids: HashSet<SsTableId> = self
            .external_dbs
            .iter()
            .flat_map(|db| db.sst_ids.iter().copied())
            .collect();
        // Owned SSTs = SSTs in any tree (unsegmented + each segment) not
        // already delegated to an external_db.
        self.core
            .all_sst_views()
            .map(|v| v.sst.id)
            .filter(|id| !source_external_sst_ids.contains(id))
            .collect()
    }

    /// Shrinks each `ExternalDb.sst_ids` to only IDs still referenced by this manifest's
    /// L0 and compacted sorted runs. `ExternalDb` entries are retained even when their
    /// `sst_ids` becomes empty — detaching a clone from its parent is done by the GC,
    /// not here, because it also requires that no live checkpoint references those IDs.
    pub(crate) fn prune_external_sst_ids(&mut self) {
        let used_sst_ids: HashSet<SsTableId> =
            self.core.all_sst_views().map(|v| v.sst.id).collect();
        for external_db in self.external_dbs.iter_mut() {
            external_db.sst_ids.retain(|id| used_sst_ids.contains(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bytes_range::BytesRange;
    use crate::manifest::store::{ManifestStore, StoredManifest};
    use slatedb_common::clock::{DefaultSystemClock, SystemClock};

    use super::{ExternalDb, Manifest, VersionedManifest};
    use crate::clone::{CloneSource, SegmentFilterFn, SegmentProjectionFn};
    use crate::config::CheckpointOptions;
    use crate::db_state::{SortedRun, SsTableHandle, SsTableId, SsTableInfo, SsTableView};
    use crate::error::SlateDBError;
    use crate::format::sst::SST_FORMAT_VERSION_LATEST;
    use crate::manifest::{LsmTreeState, ManifestCore, ProjectionConfig, Segment};
    use crate::Checkpoint;
    use bytes::Bytes;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::ObjectStore;
    use proptest::proptest;
    use rstest::rstest;
    use slatedb_common::DbRand;
    use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
    use std::ops::{Bound, Range, RangeBounds};
    use std::sync::Arc;
    use ulid::Ulid;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_init_clone_manifest() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        let parent_path = Path::from("/tmp/test_parent");
        let parent_manifest_store =
            Arc::new(ManifestStore::new(&parent_path, object_store.clone()));
        let mut parent_manifest = StoredManifest::create_new_db(
            parent_manifest_store,
            ManifestCore::new(),
            clock.clone(),
        )
        .await
        .unwrap();
        let checkpoint = parent_manifest
            .write_checkpoint(Uuid::new_v4(), &CheckpointOptions::default())
            .await
            .unwrap();

        let clone_path = Path::from("/tmp/test_clone");
        let clone_manifest_store = Arc::new(ManifestStore::new(&clone_path, object_store.clone()));
        let clone_stored_manifest = StoredManifest::store_uninitialized_clone(
            clone_manifest_store,
            Manifest::cloned(
                parent_manifest.manifest(),
                parent_path.to_string(),
                checkpoint.id,
                Arc::new(DbRand::default()),
            ),
            Arc::new(DefaultSystemClock::new()),
        )
        .await
        .unwrap();

        let clone_manifest = clone_stored_manifest.manifest();

        // There should be single external db, since parent is not deeply nested.
        assert_eq!(clone_manifest.external_dbs.len(), 1);
        assert_eq!(clone_manifest.external_dbs[0].path, parent_path.to_string());
        assert_eq!(
            clone_manifest.external_dbs[0].source_checkpoint_id,
            checkpoint.id
        );
        assert!(clone_manifest.external_dbs[0].final_checkpoint_id.is_some());

        // The clone manifest should not be initialized
        assert!(!clone_manifest.core.initialized);

        // Check epoch has been carried over
        assert_eq!(
            parent_manifest.manifest().writer_epoch,
            clone_manifest.writer_epoch
        );
        assert_eq!(
            parent_manifest.manifest().compactor_epoch,
            clone_manifest.compactor_epoch
        );
    }

    #[test]
    fn test_cloned_breaks_chain_via_final_checkpoint() {
        // When the parent is itself a clone, its external_dbs already carry an entry for
        // the grandparent. Cloning the parent must NOT propagate the grandparent's
        // user-supplied source_checkpoint_id forward — the user could delete that
        // checkpoint at any time, which would invalidate any future clones in the chain.
        // Instead, the carried-over entry's source_checkpoint_id is set to the parent's
        // final_checkpoint_id (slatedb-generated and pinned by the parent), breaking the
        // chain back to the user-supplied checkpoint.
        let rand = Arc::new(DbRand::default());

        let grandparent_sst = SsTableId::from(Ulid::new());
        let parent_owned_sst = SsTableId::from(Ulid::new());
        let grandparent_source_cp = Uuid::new_v4();
        let grandparent_final_cp = Uuid::new_v4();

        let mut parent = build_manifest(
            &SimpleManifest {
                l0: vec![SstEntry::projected("parent_owned", "a", "a"..)],
                sorted_runs: vec![],
            },
            |_| parent_owned_sst,
        );
        parent.external_dbs.push(ExternalDb {
            path: "/tmp/grandparent".to_string(),
            source_checkpoint_id: grandparent_source_cp,
            final_checkpoint_id: Some(grandparent_final_cp),
            sst_ids: vec![grandparent_sst],
        });

        let parent_cp = Uuid::new_v4();
        let cloned = Manifest::cloned(&parent, "/tmp/parent".to_string(), parent_cp, rand);

        // Two entries: the new immediate-parent entry, plus the carried-over grandparent.
        assert_eq!(cloned.external_dbs.len(), 2);

        let grandparent_entry = cloned
            .external_dbs
            .iter()
            .find(|e| e.path == "/tmp/grandparent")
            .expect("grandparent entry should be carried over");

        // The chain to the user-supplied grandparent_source_cp must be broken:
        // source_checkpoint_id is now the parent's final_checkpoint_id.
        assert_eq!(
            grandparent_entry.source_checkpoint_id, grandparent_final_cp,
            "carried-over source_checkpoint_id must be the parent's final_checkpoint_id"
        );
        assert_ne!(
            grandparent_entry.source_checkpoint_id, grandparent_source_cp,
            "carried-over source_checkpoint_id must not depend on the user-supplied checkpoint"
        );
        // A fresh final_checkpoint_id is generated for the new clone's entry.
        assert!(grandparent_entry.final_checkpoint_id.is_some());
        assert_ne!(
            grandparent_entry.final_checkpoint_id,
            Some(grandparent_final_cp),
            "final_checkpoint_id must be regenerated for the new clone"
        );
        assert_eq!(grandparent_entry.sst_ids, vec![grandparent_sst]);

        // The new immediate-parent entry uses the user-supplied checkpoint as expected.
        let parent_entry = cloned
            .external_dbs
            .iter()
            .find(|e| e.path == "/tmp/parent")
            .expect("parent entry should be added for the immediate parent");
        assert_eq!(parent_entry.source_checkpoint_id, parent_cp);
        assert_eq!(parent_entry.sst_ids, vec![parent_owned_sst]);

        // The chain-break must hold across the whole clone manifest: no entry
        // anywhere should still reference the user-supplied grandparent checkpoint.
        assert!(
            cloned
                .external_dbs
                .iter()
                .all(|e| e.source_checkpoint_id != grandparent_source_cp
                    && e.final_checkpoint_id != Some(grandparent_source_cp)),
            "no entry in the clone may reference the user-supplied grandparent_source_cp"
        );
    }

    #[test]
    fn test_cloned_drops_emptied_ancestor() {
        // Once compaction has re-localized every SST a parent borrowed from an ancestor, the
        // parent's entry for it is empty, and the detach collector may release the ancestor's
        // pin. A clone reads nothing from that ancestor, so it must not carry the entry:
        // carrying it asks the ancestor to pin a checkpoint that may no longer exist.
        let rand = Arc::new(DbRand::default());
        let parent_owned_sst = SsTableId::from(Ulid::new());
        let mut parent = build_manifest(
            &SimpleManifest {
                l0: vec![SstEntry::projected("parent_owned", "a", "a"..)],
                sorted_runs: vec![],
            },
            |_| parent_owned_sst,
        );
        parent.external_dbs.push(ExternalDb {
            path: "/tmp/grandparent".to_string(),
            source_checkpoint_id: Uuid::new_v4(),
            final_checkpoint_id: Some(Uuid::new_v4()),
            sst_ids: vec![],
        });

        let cloned = Manifest::cloned(&parent, "/tmp/parent".to_string(), Uuid::new_v4(), rand);

        assert!(
            cloned
                .external_dbs
                .iter()
                .all(|e| e.path != "/tmp/grandparent"),
            "an ancestor entry with no SSTs must not be carried over"
        );
        let parent_entry = cloned
            .external_dbs
            .iter()
            .find(|e| e.path == "/tmp/parent")
            .expect("the immediate parent is always an entry");
        assert_eq!(parent_entry.sst_ids, vec![parent_owned_sst]);
    }

    #[tokio::test]
    async fn test_write_new_checkpoint() {
        let object_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());

        let path = Path::from("/tmp/test_db");
        let manifest_store = Arc::new(ManifestStore::new(&path, object_store.clone()));
        let mut manifest = StoredManifest::create_new_db(
            Arc::clone(&manifest_store),
            ManifestCore::new(),
            clock.clone(),
        )
        .await
        .unwrap();

        let checkpoint = manifest
            .write_checkpoint(Uuid::new_v4(), &CheckpointOptions::default())
            .await
            .unwrap();

        let latest_manifest_id = manifest_store.read_latest_manifest().await.unwrap().id;
        assert_eq!(latest_manifest_id, checkpoint.manifest_id);
        assert_eq!(None, checkpoint.expire_time);
    }

    struct SstEntry {
        sst_alias: &'static str,
        first_entry: Bytes,
        visible_range: Option<BytesRange>,
    }

    impl SstEntry {
        fn regular(sst_alias: &'static str, first_entry: &'static str) -> Self {
            Self {
                sst_alias,
                first_entry: Bytes::copy_from_slice(first_entry.as_bytes()),
                visible_range: None,
            }
        }

        fn projected<T>(
            sst_alias: &'static str,
            first_entry: &'static str,
            visible_range: T,
        ) -> Self
        where
            T: RangeBounds<&'static str>,
        {
            Self {
                sst_alias,
                first_entry: Bytes::copy_from_slice(first_entry.as_bytes()),
                visible_range: Some(BytesRange::from_ref(visible_range)),
            }
        }
    }

    struct SimpleManifest {
        l0: Vec<SstEntry>,
        sorted_runs: Vec<Vec<SstEntry>>,
    }

    impl SimpleManifest {
        fn new(l0: Vec<SstEntry>, sorted_runs: Vec<(u32, Vec<SstEntry>)>) -> Self {
            Self {
                l0,
                sorted_runs: sorted_runs.into_iter().map(|(_, ssts)| ssts).collect(),
            }
        }
    }

    struct ProjectionTestCase {
        visible_range: Range<&'static str>,
        existing_manifest: SimpleManifest,
        expected_manifest: SimpleManifest,
    }

    #[rstest]
    #[case(ProjectionTestCase {
        visible_range: "h".."o",
        existing_manifest: SimpleManifest {
            l0: vec![
                SstEntry::regular("first", "a"),
                SstEntry::regular("second", "f"),
                SstEntry::regular("third", "m"),
            ],
            sorted_runs: vec![
                vec![
                    SstEntry::regular("sr0_first", "a"),
                ],
                vec![
                    SstEntry::regular("sr1_first", "a"),
                    SstEntry::regular("sr1_second", "f"),
                    SstEntry::regular("sr1_third", "m"),
                ],
            ],
        },
        expected_manifest: SimpleManifest {
            l0: vec![
                SstEntry::projected("first", "a", "h".."o"),
                SstEntry::projected("second", "f", "h".."o"),
                SstEntry::projected("third", "m", "m".."o"),
            ],
            sorted_runs: vec![
                vec![
                    // We can't filter this one out, because we don't know the
                    // end key, so it might still fall within the range
                    SstEntry::projected("sr0_first", "a", "h".."o"),
                ],
                vec![
                    SstEntry::projected("sr1_second", "f", "h".."m"),
                    SstEntry::projected("sr1_third", "m", "m".."o"),
                ],
            ],
        },
    })]
    #[case::distinct_ranges(ProjectionTestCase {
        visible_range: "c".."p",
        existing_manifest: SimpleManifest {
            l0: vec![
                SstEntry::projected("foo", "a", "a".."d"),
                SstEntry::projected("bar", "k", "n".."z"),
                SstEntry::projected("baz", "b", "s".."v"),
            ],
            sorted_runs: vec![],
        },
        expected_manifest: SimpleManifest {
            l0: vec![
                SstEntry::projected("foo", "a", "c".."d"),
                SstEntry::projected("bar", "k", "n".."p"),
            ],
            sorted_runs: vec![],
        },
    })]
    #[case::empty_sorted_run_excluded(ProjectionTestCase {
        visible_range: "a".."c",
        existing_manifest: SimpleManifest {
            l0: vec![],
            sorted_runs: vec![
                vec![
                    SstEntry::regular("sr0_first", "a"),
                    SstEntry::regular("sr0_second", "b"),
                ],
                vec![
                    SstEntry::regular("sr1_first", "a"),
                    SstEntry::regular("sr1_second", "e"),
                ],
                // sr2 is entirely outside "a".."c", so it should be excluded
                vec![
                    SstEntry::regular("sr2_first", "d"),
                    SstEntry::regular("sr2_second", "f"),
                ],
            ],
        },
        expected_manifest: SimpleManifest {
            l0: vec![],
            sorted_runs: vec![
                vec![
                    SstEntry::projected("sr0_first", "a", "a".."b"),
                    SstEntry::projected("sr0_second", "b", "b".."c"),
                ],
                vec![
                    SstEntry::projected("sr1_first", "a", "a".."c"),
                ],
            ],
        },
    })]
    fn test_projected(#[case] test_case: ProjectionTestCase) {
        let mut sst_ids = HashMap::new();
        let initial_manifest = build_manifest(&test_case.existing_manifest, |alias| {
            let sst_id = SsTableId::from(Ulid::new());
            if sst_ids.insert(alias.to_string(), sst_id).is_some() {
                unreachable!("duplicate sst alias")
            }
            sst_id
        });

        let projected = Manifest::projected(
            &initial_manifest,
            &ProjectionConfig::from_global_range(BytesRange::from_ref(test_case.visible_range)),
        )
        .unwrap();

        let expected_manifest = build_manifest(&test_case.expected_manifest, |alias| {
            *sst_ids.get(alias).unwrap()
        });

        assert_manifest_equal(&projected, &expected_manifest, &sst_ids);
    }

    struct UnionTestCase {
        manifests: Vec<SimpleManifest>,
        expected: SimpleManifest,
    }

    #[rstest]
    #[case::non_overlapping_l0s(UnionTestCase {
        manifests: vec![
            SimpleManifest {
                l0: vec![
                    SstEntry::projected("foo", "a", "a".."m"),
                    SstEntry::projected("bar", "f", "a".."m"),
                    SstEntry::projected("baz", "j", "a".."m")
                ],
                sorted_runs: vec![]
            },
            SimpleManifest {
                l0: vec![
                    SstEntry::projected("foo", "a", "m"..),
                    SstEntry::projected("bar", "f", "m"..),
                    SstEntry::projected("baz", "j", "m"..)
                ],
                sorted_runs: vec![]
            }
        ],
        expected: SimpleManifest {
            l0: vec![
                SstEntry::projected("foo", "a", "a".."m"),
                SstEntry::projected("bar", "f", "a".."m"),
                SstEntry::projected("baz", "j", "a".."m"),
                SstEntry::projected("foo", "a", "m"..),
                SstEntry::projected("bar", "f", "m"..),
                SstEntry::projected("baz", "j", "m"..)
                // This is not optimal, but it's a good start from correctness point of view. Eventually we want the manifest to look as follows:
                //
                // SstEntry::projected("foo", "a", "a"..),
                // SstEntry::projected("bar", "f", "a"..),
                // SstEntry::projected("baz", "j", "a"..),
            ],
            sorted_runs: vec![]
        },
    })]
    #[case::non_overlapping_l0s_with_gap(UnionTestCase {
        manifests: vec![
            SimpleManifest {
                l0: vec![
                    SstEntry::projected("foo", "a", "a".."m"),
                    SstEntry::projected("bar", "f", "a".."m"),
                    SstEntry::projected("baz", "j", "a".."m")
                ],
                sorted_runs: vec![]
            },
            SimpleManifest {
                l0: vec![
                    SstEntry::projected("foo", "a", "o"..),
                    SstEntry::projected("bar", "f", "o"..),
                    SstEntry::projected("baz", "j", "o"..)
                ],
                sorted_runs: vec![]
            }
        ],
        expected: SimpleManifest {
            l0: vec![
                SstEntry::projected("foo", "a", "a".."m"),
                SstEntry::projected("bar", "f", "a".."m"),
                SstEntry::projected("baz", "j", "a".."m"),
                SstEntry::projected("foo", "a", "o"..),
                SstEntry::projected("bar", "f", "o"..),
                SstEntry::projected("baz", "j", "o"..)
            ],
            sorted_runs: vec![]
        },
    })]
    fn test_union(#[case] test_case: UnionTestCase) {
        let mut sst_ids: HashMap<String, SsTableId> = HashMap::new();
        let rand = Arc::new(DbRand::default());
        let sources: Vec<CloneSource> = test_case
            .manifests
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let manifest = build_manifest(m, |alias| {
                    if let Some(sst_id) = sst_ids.get(alias) {
                        *sst_id
                    } else {
                        let sst_id = SsTableId::from(Ulid::new());
                        sst_ids.insert(alias.to_string(), sst_id);
                        sst_id
                    }
                });
                CloneSource {
                    manifest,
                    path: Path::from(format!("/tmp/db{}", i)),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                }
            })
            .collect();

        let expected_manifest =
            build_manifest(&test_case.expected, |alias| *sst_ids.get(alias).unwrap());

        let union = Manifest::cloned_from_union(sources, rand).unwrap();

        assert_manifest_equal(&union, &expected_manifest, &sst_ids);
    }

    #[test]
    fn test_lsm_tree_merge_invariants() {
        // Build a writer L0 with `n` views whose view IDs and SST IDs are all
        // distinct, so we can tell V2 (view-id) and V1 (sst-id) marker
        // matching apart.
        fn build_writer_l0(n: usize) -> VecDeque<SsTableView> {
            (0..n)
                .map(|i| {
                    let view_id = Ulid::from_parts(i as u64, 0);
                    let sst_id = Ulid::from_parts(i as u64, 1);
                    let handle = SsTableHandle::new(
                        SsTableId::from(sst_id),
                        SST_FORMAT_VERSION_LATEST,
                        SsTableInfo::default(),
                    );
                    SsTableView::new(view_id, handle)
                })
                .collect()
        }

        proptest!(|(
            n in 1usize..10,
            // 0 = no marker, 1 = V2 (view id) only, 2 = V1 (sst id) only, 3 = both
            marker_kind in 0u8..4,
            // Resolved against [0, n] below: index `n` means "marker doesn't
            // match any L0 entry," exercising the no-trim path even with a
            // marker present.
            cutoff_idx_raw in 0usize..32,
        )| {
            let writer_l0 = build_writer_l0(n);
            let cutoff_idx = cutoff_idx_raw % (n + 1);

            let (last_view, last_sst) = if marker_kind == 0 {
                (None, None)
            } else if cutoff_idx == n {
                // Marker that doesn't match any entry.
                let nonmatch = Ulid::from_parts(u64::MAX, 0);
                match marker_kind {
                    1 => (Some(nonmatch), None),
                    2 => (None, Some(nonmatch)),
                    _ => (Some(nonmatch), Some(nonmatch)),
                }
            } else {
                let target = &writer_l0[cutoff_idx];
                let view_id = target.id;
                let sst_id = target.sst.id.value();
                match marker_kind {
                    1 => (Some(view_id), None),
                    2 => (None, Some(sst_id)),
                    _ => (Some(view_id), Some(sst_id)),
                }
            };

            let writer = LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: writer_l0.clone(),
                compacted: vec![],
            };
            let compactor_compacted = vec![SortedRun::new(42, [])];
            let compactor = LsmTreeState {
                last_compacted_l0_sst_view_id: last_view,
                last_compacted_l0_sst_id: last_sst,
                l0: VecDeque::new(),
                compacted: compactor_compacted.clone(),
            };

            let merged = writer.merge_from_compactor(&compactor);

            // Effective trim point: cutoff_idx if a matching marker is set,
            // otherwise n (everything passes through).
            let effective_cutoff = if marker_kind == 0 || cutoff_idx == n {
                n
            } else {
                cutoff_idx
            };
            let expected_l0: Vec<_> = writer_l0.iter().take(effective_cutoff).cloned().collect();
            let actual_l0: Vec<_> = merged.l0.iter().cloned().collect();
            assert_eq!(actual_l0, expected_l0);

            // Markers and compacted are taken from the compactor unchanged.
            assert_eq!(merged.last_compacted_l0_sst_view_id, last_view);
            assert_eq!(merged.last_compacted_l0_sst_id, last_sst);
            assert_eq!(merged.compacted, compactor_compacted);

            // The two wrappers must agree for any (writer, compactor) pair —
            // catches accidental arg-swapping in the wrappers.
            let merged_via_writer_side = compactor.merge_from_writer(&writer);
            assert_eq!(merged, merged_via_writer_side);
        });
    }

    #[test]
    fn test_segment_merge_preserves_prefix_order() {
        // Both directions of the segment-list merge must produce a result
        // sorted by `prefix` for any well-formed sorted inputs. Compactor-
        // only prefixes must be drain markers (anything else trips the
        // merge's protocol-violation guard).
        use crate::manifest::{
            is_sorted_by_prefix, merge_segments_from_compactor, merge_segments_from_writer, Segment,
        };

        fn live_tree(seed: u64) -> LsmTreeState {
            let view_id = Ulid::from_parts(seed, 0);
            let handle = SsTableHandle::new(
                SsTableId::from(Ulid::from_parts(seed, 1)),
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo::default(),
            );
            LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![SsTableView::new(view_id, handle)]),
                compacted: vec![],
            }
        }
        fn marker_tree(seed: u64) -> LsmTreeState {
            LsmTreeState {
                last_compacted_l0_sst_view_id: Some(Ulid::from_parts(seed, 0)),
                last_compacted_l0_sst_id: None,
                l0: VecDeque::new(),
                compacted: vec![],
            }
        }

        // Each `kind` value picks a (writer-has, compactor-has) pair for
        // a given prefix:
        //   0: writer-only
        //   1: compactor-only (must be marker)
        //   2: both (writer live, compactor live)
        //   3: skip (prefix absent on both sides)
        proptest!(|(kinds in proptest::collection::vec(0u8..4, 0..8))| {
            let mut writer: Vec<Segment> = Vec::new();
            let mut compactor: Vec<Segment> = Vec::new();
            for (idx, kind) in kinds.iter().enumerate() {
                let prefix = Bytes::from(format!("p{:02}/", idx));
                match kind % 4 {
                    0 => writer.push(Segment { prefix: prefix.clone(), tree: Arc::new(live_tree(idx as u64)) }),
                    1 => compactor.push(Segment { prefix: prefix.clone(), tree: Arc::new(marker_tree(idx as u64)) }),
                    2 => {
                        writer.push(Segment { prefix: prefix.clone(), tree: Arc::new(live_tree(idx as u64)) });
                        compactor.push(Segment { prefix: prefix.clone(), tree: Arc::new(live_tree((idx as u64) + 100)) });
                    }
                    _ => {}
                }
            }

            // Both inputs are constructed in prefix order via the index.
            assert!(is_sorted_by_prefix(&writer));
            assert!(is_sorted_by_prefix(&compactor));

            let merged_writer_side = merge_segments_from_compactor(&writer, &compactor);
            let merged_compactor_side = merge_segments_from_writer(&compactor, &writer);

            assert!(is_sorted_by_prefix(&merged_writer_side),
                "writer-side merge must produce sorted output");
            assert!(is_sorted_by_prefix(&merged_compactor_side),
                "compactor-side merge must produce sorted output");
        });
    }

    fn make_view(seed: u64) -> SsTableView {
        let view_id = Ulid::from_parts(seed, 0);
        let handle = SsTableHandle::new(
            SsTableId::from(Ulid::from_parts(seed, 1)),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo::default(),
        );
        SsTableView::new(view_id, handle)
    }

    /// A manifest with L0 SSTs 1 and 2 and sorted-run SST 3 in the root tree,
    /// and L0 SST 4 in segment `s`.
    fn core_with_four_ssts() -> ManifestCore {
        let mut core = ManifestCore::new();
        core.tree = Arc::new(LsmTreeState {
            l0: VecDeque::from([make_view(1), make_view(2)]),
            compacted: vec![SortedRun::new(0, [make_view(3)])],
            ..LsmTreeState::default()
        });
        core.segments.push(Segment {
            prefix: Bytes::from_static(b"s"),
            tree: Arc::new(LsmTreeState {
                l0: VecDeque::from([make_view(4)]),
                ..LsmTreeState::default()
            }),
        });
        core
    }

    #[test]
    fn test_all_sst_views_covers_every_tree_and_level() {
        let manifest =
            VersionedManifest::from_manifest(1, Manifest::initial(core_with_four_ssts()));
        let ids: HashSet<SsTableId> = manifest.all_sst_views().map(|view| view.sst.id).collect();
        let expected: HashSet<SsTableId> = (1..=4).map(|seed| make_view(seed).sst.id).collect();
        assert_eq!(ids, expected);
    }

    #[test]
    fn maybe_insert_tree_inserts_at_sorted_position() {
        let mut core = ManifestCore::new();
        core.maybe_insert_tree(&Bytes::from_static(b"b")).unwrap();
        // Predecessor.
        core.maybe_insert_tree(&Bytes::from_static(b"a")).unwrap();
        // Successor.
        core.maybe_insert_tree(&Bytes::from_static(b"c")).unwrap();
        let prefixes: Vec<&[u8]> = core.segments.iter().map(|s| s.prefix.as_ref()).collect();
        assert_eq!(prefixes, vec![&b"a"[..], &b"b"[..], &b"c"[..]]);
    }

    #[test]
    fn maybe_insert_tree_returns_existing_on_hit() {
        let mut core = ManifestCore::new();
        let prefix = Bytes::from_static(b"a");
        core.maybe_insert_tree(&prefix)
            .unwrap()
            .l0
            .push_front(make_view(1));
        // Second call must return the same tree, with the previous mutation
        // still in place.
        let tree = core.maybe_insert_tree(&prefix).unwrap();
        assert_eq!(tree.l0.len(), 1);
        assert_eq!(core.segments.len(), 1, "no duplicate segment created");
    }

    #[test]
    fn maybe_insert_tree_handles_empty_prefix_when_alone() {
        let mut core = ManifestCore::new();
        let tree = core.maybe_insert_tree(&Bytes::new()).unwrap();
        tree.l0.push_front(make_view(1));
        assert_eq!(core.segments.len(), 1);
        assert!(core.segments[0].prefix.is_empty());
        assert_eq!(core.segments[0].tree.l0.len(), 1);
    }

    #[test]
    fn maybe_insert_tree_rejects_descendant_prefix() {
        use crate::error::SlateDBError;
        let mut core = ManifestCore::new();
        core.maybe_insert_tree(&Bytes::from_static(b"abc")).unwrap();
        // "abcd" extends "abc" — nested.
        let err = core
            .maybe_insert_tree(&Bytes::from_static(b"abcd"))
            .unwrap_err();
        assert!(matches!(
            err,
            SlateDBError::InvalidSegmentPrefix { ref prefix, ref conflict }
                if prefix.as_ref() == b"abcd" && conflict.as_ref() == b"abc"
        ));
    }

    #[test]
    fn maybe_insert_tree_rejects_ancestor_prefix() {
        use crate::error::SlateDBError;
        let mut core = ManifestCore::new();
        core.maybe_insert_tree(&Bytes::from_static(b"abcd"))
            .unwrap();
        // "abc" is a strict prefix of "abcd" — nested.
        let err = core
            .maybe_insert_tree(&Bytes::from_static(b"abc"))
            .unwrap_err();
        assert!(matches!(
            err,
            SlateDBError::InvalidSegmentPrefix { ref prefix, ref conflict }
                if prefix.as_ref() == b"abc" && conflict.as_ref() == b"abcd"
        ));
    }

    #[test]
    fn maybe_insert_tree_rejects_empty_alongside_named() {
        use crate::error::SlateDBError;
        let mut core = ManifestCore::new();
        core.maybe_insert_tree(&Bytes::from_static(b"a")).unwrap();
        // Empty prefix is a strict prefix of every non-empty prefix.
        let err = core.maybe_insert_tree(&Bytes::new()).unwrap_err();
        assert!(matches!(err, SlateDBError::InvalidSegmentPrefix { .. }));
    }

    #[test]
    fn check_segment_prefix_antichain_accepts_exact_and_disjoint() {
        let mut core = ManifestCore::new();
        core.maybe_insert_tree(&Bytes::from_static(b"a")).unwrap();
        core.maybe_insert_tree(&Bytes::from_static(b"c")).unwrap();
        // Exact match: routing into an existing segment is fine.
        core.check_segment_prefix_antichain(b"a").unwrap();
        // Disjoint sibling between a and c.
        core.check_segment_prefix_antichain(b"b").unwrap();
        // Disjoint past the tail.
        core.check_segment_prefix_antichain(b"d").unwrap();
    }

    #[test]
    fn check_segment_prefix_antichain_rejects_descendant_and_ancestor() {
        use crate::error::SlateDBError;
        let mut core = ManifestCore::new();
        core.maybe_insert_tree(&Bytes::from_static(b"abc")).unwrap();
        let err = core.check_segment_prefix_antichain(b"abcd").unwrap_err();
        assert!(matches!(
            err,
            SlateDBError::InvalidSegmentPrefix { ref prefix, ref conflict }
                if prefix.as_ref() == b"abcd" && conflict.as_ref() == b"abc"
        ));
        let err = core.check_segment_prefix_antichain(b"ab").unwrap_err();
        assert!(matches!(
            err,
            SlateDBError::InvalidSegmentPrefix { ref prefix, ref conflict }
                if prefix.as_ref() == b"ab" && conflict.as_ref() == b"abc"
        ));
    }

    #[test]
    fn check_segment_prefix_antichain_does_not_mutate_segments() {
        let mut core = ManifestCore::new();
        core.maybe_insert_tree(&Bytes::from_static(b"a")).unwrap();
        let before = core.segments.clone();
        // A passing check must not alter state.
        core.check_segment_prefix_antichain(b"b").unwrap();
        // Neither must a failing one.
        let _ = core.check_segment_prefix_antichain(b"abc");
        assert_eq!(core.segments, before);
    }

    /// Simulator-based protocol check: drive a random interleaving of writer
    /// flushes/commits and compactor compactions/drains/commits through the
    /// segment merge protocol, and verify a battery of invariants on the
    /// resulting manifest history. The strongest is that each L0 the writer
    /// flushes appears in the committed manifest exactly once: never
    /// fabricated, never lost, never resurrected after a drain. Other
    /// invariants check L0 provenance, watermark monotonicity, watermark
    /// trim correctness, and cross-segment L0 uniqueness.
    #[test]
    fn test_protocol_simulation_invariants() {
        use crate::manifest::{merge_segments_from_compactor, merge_segments_from_writer, Segment};
        use proptest::prelude::*;

        const NUM_PREFIXES: u8 = 3;

        #[derive(Debug, Clone)]
        enum Op {
            WriterFlush(u8),
            WriterCommit,
            CompactorReadCompact(u8, u8),
            CompactorReadDrain(u8),
            CompactorCommit,
        }

        fn arb_op() -> impl Strategy<Value = Op> {
            prop_oneof![
                (0..NUM_PREFIXES).prop_map(Op::WriterFlush),
                Just(Op::WriterCommit),
                (0..NUM_PREFIXES, 1u8..4).prop_map(|(p, c)| Op::CompactorReadCompact(p, c)),
                (0..NUM_PREFIXES).prop_map(Op::CompactorReadDrain),
                Just(Op::CompactorCommit),
            ]
        }

        fn make_prefix(idx: u8) -> Bytes {
            Bytes::from(format!("p{:02}/", idx))
        }

        fn make_view(seq: u64) -> SsTableView {
            let view_id = Ulid::from_parts(seq, 0);
            SsTableView::new(
                view_id,
                SsTableHandle::new(
                    SsTableId::from(Ulid::from_parts(seq, 1)),
                    SST_FORMAT_VERSION_LATEST,
                    SsTableInfo::default(),
                ),
            )
        }

        struct Simulator {
            // Published manifest's segment list — what's currently durable.
            store: Vec<Segment>,
            // Writer's in-memory state. Mutated by flushes; merged with
            // `store` on commit.
            writer: Vec<Segment>,
            // Compactor's in-memory state. Mutated by compactions/drains;
            // merged with `store` on commit.
            compactor: Vec<Segment>,
            next_l0_seq: u64,
            next_sr_id: u32,
            // Every L0 view ID the writer has ever flushed — the
            // provenance "ground truth."
            flushed_l0s: BTreeSet<Ulid>,
            // History of L0 view ID sets present in `store` after each
            // published commit. The resurrection check walks this.
            l0_history: Vec<BTreeSet<Ulid>>,
            // For each segment prefix, the watermark observed at each
            // commit (None if the prefix wasn't present). Used to
            // verify watermark monotonicity.
            watermark_history: Vec<HashMap<Bytes, Option<Ulid>>>,
        }

        impl Simulator {
            fn new() -> Self {
                Self {
                    store: Vec::new(),
                    writer: Vec::new(),
                    compactor: Vec::new(),
                    next_l0_seq: 0,
                    next_sr_id: 0,
                    flushed_l0s: BTreeSet::new(),
                    l0_history: Vec::new(),
                    watermark_history: Vec::new(),
                }
            }

            fn writer_flush(&mut self, prefix_idx: u8) {
                self.next_l0_seq += 1;
                let view = make_view(self.next_l0_seq);
                self.flushed_l0s.insert(view.id);
                let prefix = make_prefix(prefix_idx);
                match self
                    .writer
                    .binary_search_by(|s| s.prefix.as_ref().cmp(prefix.as_ref()))
                {
                    Ok(idx) => Arc::make_mut(&mut self.writer[idx].tree)
                        .l0
                        .push_front(view),
                    Err(idx) => self.writer.insert(
                        idx,
                        Segment {
                            prefix,
                            tree: Arc::new(LsmTreeState {
                                last_compacted_l0_sst_view_id: None,
                                last_compacted_l0_sst_id: None,
                                l0: VecDeque::from(vec![view]),
                                compacted: vec![],
                            }),
                        },
                    ),
                }
            }

            fn writer_commit(&mut self) {
                self.writer = merge_segments_from_compactor(&self.writer, &self.store);
                self.store = self.writer.clone();
                self.snapshot();
            }

            // Sync compactor's local state from the latest published manifest
            // before applying a compactor mutation. This models the compactor
            // reading writer's manifest at the start of each cycle.
            fn compactor_sync(&mut self) {
                self.compactor = merge_segments_from_writer(&self.compactor, &self.store);
            }

            fn compactor_read_compact(&mut self, prefix_idx: u8, count: u8) {
                self.compactor_sync();
                let prefix = make_prefix(prefix_idx);
                if let Ok(idx) = self
                    .compactor
                    .binary_search_by(|s| s.prefix.as_ref().cmp(prefix.as_ref()))
                {
                    let seg = &mut self.compactor[idx];
                    let tree = Arc::make_mut(&mut seg.tree);
                    let count = (count as usize).min(tree.l0.len());
                    if count == 0 {
                        return;
                    }
                    let newest = tree.l0[0].id;
                    let sr_views: Vec<_> = (0..count).map(|i| tree.l0[i].clone()).collect();
                    for _ in 0..count {
                        tree.l0.pop_front();
                    }
                    tree.last_compacted_l0_sst_view_id = Some(newest);
                    self.next_sr_id += 1;
                    tree.compacted
                        .insert(0, SortedRun::new(self.next_sr_id, sr_views));
                }
            }

            fn compactor_read_drain(&mut self, prefix_idx: u8) {
                self.compactor_sync();
                let prefix = make_prefix(prefix_idx);
                if let Ok(idx) = self
                    .compactor
                    .binary_search_by(|s| s.prefix.as_ref().cmp(prefix.as_ref()))
                {
                    let seg = &mut self.compactor[idx];
                    let tree = Arc::make_mut(&mut seg.tree);
                    if let Some(newest) = tree.l0.front() {
                        tree.last_compacted_l0_sst_view_id = Some(newest.id);
                    }
                    tree.l0.clear();
                    tree.compacted.clear();
                }
            }

            fn compactor_commit(&mut self) {
                self.compactor = merge_segments_from_writer(&self.compactor, &self.store);
                self.store = self.compactor.clone();
                self.snapshot();
            }

            fn snapshot(&mut self) {
                let mut l0_ids = BTreeSet::new();
                let mut watermarks: HashMap<Bytes, Option<Ulid>> = HashMap::new();
                for seg in &self.store {
                    for view in &seg.tree.l0 {
                        l0_ids.insert(view.id);
                    }
                    watermarks.insert(seg.prefix.clone(), seg.tree.last_compacted_l0_sst_view_id);
                }
                self.l0_history.push(l0_ids);
                self.watermark_history.push(watermarks);
            }

            // Drive any pending writer state into `store` and let the
            // compactor follow, so that "added at least once" can be
            // checked against `flushed_l0s`. Two cycles is enough for a
            // pending flush to traverse Live → potential drain → prune.
            fn settle(&mut self) {
                self.writer_commit();
                self.compactor_commit();
                self.writer_commit();
                self.compactor_commit();
            }

            fn check_invariants(&self) {
                self.check_no_l0_resurrection();
                self.check_l0_provenance();
                self.check_l0_unique_across_segments();
                self.check_watermark_trim();
                self.check_watermark_monotonic();
                self.check_l0_not_in_l0_and_sr_simultaneously();
            }

            // Each L0 ID has at most one NotSeen → Present transition;
            // an Absent → Present transition is a resurrection.
            fn check_no_l0_resurrection(&self) {
                let mut all_ids = BTreeSet::new();
                for snap in &self.l0_history {
                    all_ids.extend(snap.iter().copied());
                }
                for id in all_ids {
                    let mut state = 0u8; // 0 not seen, 1 present, 2 absent-after
                    for snap in &self.l0_history {
                        let present = snap.contains(&id);
                        state = match (state, present) {
                            (0, false) => 0,
                            (0, true) | (1, true) => 1,
                            (1, false) | (2, false) => 2,
                            (2, true) => panic!(
                                "L0 {} resurrected after removal; history={:?}",
                                id, self.l0_history
                            ),
                            _ => unreachable!(),
                        };
                    }
                }
            }

            // Every L0 ID appearing in any committed manifest must have
            // come from a writer flush — the merge must not fabricate IDs.
            fn check_l0_provenance(&self) {
                for snap in &self.l0_history {
                    for id in snap {
                        assert!(
                            self.flushed_l0s.contains(id),
                            "L0 {} appeared in manifest but was never flushed",
                            id
                        );
                    }
                }
                for seg in &self.store {
                    for sr in &seg.tree.compacted {
                        for view in sr.sst_views() {
                            assert!(
                                self.flushed_l0s.contains(&view.id),
                                "SR {} references L0 {} that was never flushed",
                                sr.id,
                                view.id
                            );
                        }
                    }
                }
            }

            // No L0 ID may appear in two different segments' L0 lists
            // simultaneously.
            fn check_l0_unique_across_segments(&self) {
                let mut seen: HashMap<Ulid, Bytes> = HashMap::new();
                for seg in &self.store {
                    for view in &seg.tree.l0 {
                        if let Some(other) = seen.get(&view.id) {
                            panic!(
                                "L0 {} appears in segment {:?} and {:?} simultaneously",
                                view.id, other, seg.prefix
                            );
                        }
                        seen.insert(view.id, seg.prefix.clone());
                    }
                }
            }

            // Within a segment, no L0 in the L0 list may have an ID at
            // or below the segment's watermark — those are supposed to
            // be trimmed by the merge.
            fn check_watermark_trim(&self) {
                for seg in &self.store {
                    if let Some(wm) = seg.tree.last_compacted_l0_sst_view_id {
                        for view in &seg.tree.l0 {
                            assert!(
                                view.id > wm,
                                "L0 {} survived in segment {:?} but watermark is {}",
                                view.id,
                                seg.prefix,
                                wm
                            );
                        }
                    }
                }
            }

            // For each segment prefix, the watermark must only advance
            // (or stay None) across the published-manifest history.
            fn check_watermark_monotonic(&self) {
                let mut all_prefixes = BTreeSet::new();
                for snap in &self.watermark_history {
                    all_prefixes.extend(snap.keys().cloned());
                }
                for prefix in all_prefixes {
                    let mut prev: Option<Ulid> = None;
                    for snap in &self.watermark_history {
                        let cur = snap.get(&prefix).copied().flatten();
                        if let (Some(p), Some(c)) = (prev, cur) {
                            assert!(
                                c >= p,
                                "watermark for {:?} regressed: {} → {}",
                                prefix,
                                p,
                                c
                            );
                        }
                        // Once a segment is dropped (cur == None), the
                        // next reincarnation starts fresh — don't carry
                        // the old watermark forward as a constraint.
                        if cur.is_some() {
                            prev = cur;
                        } else if !snap.contains_key(&prefix) {
                            prev = None;
                        }
                    }
                }
            }

            // Within any single committed manifest, an L0 ID present in
            // a segment's `l0` list must not also appear in any of that
            // segment's SRs.
            fn check_l0_not_in_l0_and_sr_simultaneously(&self) {
                for seg in &self.store {
                    let l0_ids: BTreeSet<Ulid> = seg.tree.l0.iter().map(|v| v.id).collect();
                    for sr in &seg.tree.compacted {
                        for view in sr.sst_views() {
                            assert!(
                                !l0_ids.contains(&view.id),
                                "L0 {} appears in both l0 list and SR {} of segment {:?}",
                                view.id,
                                sr.id,
                                seg.prefix
                            );
                        }
                    }
                }
            }

            // Every L0 the writer flushed must have appeared in at
            // least one published manifest's L0 list. Combined with the
            // resurrection check, this gives "added exactly once."
            fn check_added_exactly_once(&self) {
                for id in &self.flushed_l0s {
                    let appeared = self.l0_history.iter().any(|s| s.contains(id));
                    assert!(
                        appeared,
                        "L0 {} was flushed but never reached a committed manifest",
                        id
                    );
                }
            }
        }

        proptest!(|(ops in proptest::collection::vec(arb_op(), 0..40))| {
            let mut sim = Simulator::new();
            for op in &ops {
                match op {
                    Op::WriterFlush(p) => sim.writer_flush(*p),
                    Op::WriterCommit => sim.writer_commit(),
                    Op::CompactorReadCompact(p, c) => sim.compactor_read_compact(*p, *c),
                    Op::CompactorReadDrain(p) => sim.compactor_read_drain(*p),
                    Op::CompactorCommit => sim.compactor_commit(),
                }
                sim.check_invariants();
            }
            // Drain any pending writer state through the protocol so the
            // "added at least once" half of "exactly once" can hold.
            sim.settle();
            sim.check_invariants();
            sim.check_added_exactly_once();
        });
    }

    #[test]
    fn test_union_renumbers_sr_ids() {
        // Create manifest 1 with 2 sorted runs covering "a".."m"
        let manifest1 = build_manifest(
            &SimpleManifest {
                l0: vec![],
                sorted_runs: vec![
                    vec![SstEntry::projected("sr1_0_sst0", "a", "a".."g")],
                    vec![SstEntry::projected("sr1_1_sst0", "g", "g".."m")],
                ],
            },
            |_| SsTableId::from(Ulid::new()),
        );

        // Create manifest 2 with 3 sorted runs covering "m"..∞
        let manifest2 = build_manifest(
            &SimpleManifest {
                l0: vec![],
                sorted_runs: vec![
                    vec![SstEntry::projected("sr2_0_sst0", "m", "m".."s")],
                    vec![SstEntry::projected("sr2_1_sst0", "s", "s".."t")],
                    vec![SstEntry::projected("sr2_2_sst0", "t", "t"..)],
                ],
            },
            |_| SsTableId::from(Ulid::new()),
        );

        let rand = Arc::new(DbRand::default());
        let union = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: manifest1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: manifest2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            rand,
        )
        .unwrap();

        // After union, we should have 5 SRs with IDs 4, 3, 2, 1, 0
        // (renumber assigns descending ids in walk order so that within
        // each tree the first list entry has the highest id).
        assert_eq!(union.core.tree.compacted.len(), 5);

        let sr_ids: Vec<u32> = union.core.tree.compacted.iter().map(|sr| sr.id).collect();
        assert_eq!(
            sr_ids,
            vec![4, 3, 2, 1, 0],
            "SR IDs should descend in list order"
        );

        // Verify no duplicates
        let mut seen = HashSet::new();
        for id in &sr_ids {
            assert!(seen.insert(id), "Duplicate SR ID: {}", id);
        }
    }

    #[test]
    fn test_union_propagates_last_l0_seq() {
        let mut manifest1 = build_manifest(
            &SimpleManifest {
                l0: vec![],
                sorted_runs: vec![vec![SstEntry::projected("sr1", "a", "a".."m")]],
            },
            |_| SsTableId::from(Ulid::new()),
        );
        manifest1.core.last_l0_seq = 100;

        let mut manifest2 = build_manifest(
            &SimpleManifest {
                l0: vec![],
                sorted_runs: vec![vec![SstEntry::projected("sr2", "m", "m"..)]],
            },
            |_| SsTableId::from(Ulid::new()),
        );
        manifest2.core.last_l0_seq = 200;

        let union = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: manifest1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: manifest2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            Arc::new(DbRand::default()),
        )
        .unwrap();

        assert_eq!(union.core.last_l0_seq, 200);
    }

    #[rstest]
    #[case(100, 250, 250)]
    #[case(250, 100, 250)]
    #[case(i64::MIN, 250, 250)]
    #[case(i64::MIN, i64::MIN, i64::MIN)]
    fn test_union_propagates_last_l0_clock_tick(
        #[case] tick1: i64,
        #[case] tick2: i64,
        #[case] expected: i64,
    ) {
        let mut manifest1 = build_manifest(
            &SimpleManifest {
                l0: vec![],
                sorted_runs: vec![vec![SstEntry::projected("sr1", "a", "a".."m")]],
            },
            |_| SsTableId::from(Ulid::new()),
        );
        manifest1.core.last_l0_clock_tick = tick1;

        let mut manifest2 = build_manifest(
            &SimpleManifest {
                l0: vec![],
                sorted_runs: vec![vec![SstEntry::projected("sr2", "m", "m"..)]],
            },
            |_| SsTableId::from(Ulid::new()),
        );
        manifest2.core.last_l0_clock_tick = tick2;

        let union = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: manifest1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: manifest2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            Arc::new(DbRand::default()),
        )
        .unwrap();

        assert_eq!(union.core.last_l0_clock_tick, expected);
    }

    #[test]
    fn test_union_drops_emptied_ancestor() {
        // As test_cloned_drops_emptied_ancestor, for a union: a source's inherited entry
        // with no SSTs is not carried into the union.
        let rand = Arc::new(DbRand::default());
        let own_sst = SsTableId::from(Ulid::new());
        let mut manifest = build_manifest(
            &SimpleManifest {
                l0: vec![SstEntry::projected("own", "a", "a"..)],
                sorted_runs: vec![],
            },
            |_| own_sst,
        );
        manifest.external_dbs.push(ExternalDb {
            path: "/tmp/grandparent".to_string(),
            source_checkpoint_id: Uuid::new_v4(),
            final_checkpoint_id: Some(Uuid::new_v4()),
            sst_ids: vec![],
        });

        let union = Manifest::cloned_from_union(
            vec![CloneSource {
                manifest,
                path: Path::from("/tmp/db1"),
                checkpoint: new_checkpoint(Uuid::new_v4()),
            }],
            rand,
        )
        .unwrap();

        assert!(
            union
                .external_dbs
                .iter()
                .all(|e| e.path != "/tmp/grandparent"),
            "an ancestor entry with no SSTs must not be carried into a union"
        );
        assert!(union.external_dbs.iter().any(|e| e.path == "tmp/db1"));
    }

    #[test]
    fn test_union_external_dbs() {
        // manifest1 is clone-like: owns own_sst in core and inherits grandparent_sst
        // via an existing external_db entry. manifest2 is a plain source.
        // The union must: add each source as an ExternalDb (owned SSTs only),
        // carry over inherited chains, and resolve all SSTs to the correct paths.
        let rand = Arc::new(DbRand::default());

        let parent1_sst1 = SsTableId::from(Ulid::new());
        let parent2_sst1 = SsTableId::from(Ulid::new());
        let grandparent_sst = SsTableId::from(Ulid::new());
        let grandparent_source_cp = Uuid::new_v4();
        let grandparent_final_cp = Uuid::new_v4();

        let mut manifest1 = build_manifest(
            &SimpleManifest {
                l0: vec![SstEntry::projected("own", "a", "a".."m")],
                sorted_runs: vec![],
            },
            |_| parent1_sst1,
        );
        manifest1.external_dbs.push(ExternalDb {
            path: "/tmp/grandparent".to_string(),
            source_checkpoint_id: grandparent_source_cp,
            final_checkpoint_id: Some(grandparent_final_cp),
            sst_ids: vec![grandparent_sst],
        });

        let manifest2 = build_manifest(
            &SimpleManifest {
                l0: vec![SstEntry::projected("sst2", "m", "m"..)],
                sorted_runs: vec![],
            },
            |_| parent2_sst1,
        );

        let cp1 = Uuid::new_v4();
        let cp2 = Uuid::new_v4();
        let union = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: manifest1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(cp1),
                },
                CloneSource {
                    manifest: manifest2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(cp2),
                },
            ],
            rand,
        )
        .unwrap();

        // db1 (owned SSTs only), grandparent (carried over), db2
        assert_eq!(union.external_dbs.len(), 3);

        let db1 = union
            .external_dbs
            .iter()
            .find(|e| e.path == "tmp/db1")
            .unwrap();
        assert_eq!(db1.source_checkpoint_id, cp1);
        assert_eq!(db1.sst_ids, vec![parent1_sst1]); // grandparent_sst must not leak in

        let db2 = union
            .external_dbs
            .iter()
            .find(|e| e.path == "tmp/db2")
            .unwrap();
        assert_eq!(db2.source_checkpoint_id, cp2);
        assert_eq!(db2.sst_ids, vec![parent2_sst1]);

        let grandparent = union
            .external_dbs
            .iter()
            .find(|e| e.path == "/tmp/grandparent")
            .unwrap();
        // The carried-over entry's source_checkpoint_id must be replaced with the
        // parent's final_checkpoint_id, so the union does not depend on the original
        // user-supplied grandparent_source_cp (which the user could delete).
        assert_eq!(grandparent.source_checkpoint_id, grandparent_final_cp);
        assert_ne!(grandparent.source_checkpoint_id, grandparent_source_cp);
        // final_checkpoint_id must be regenerated — the union clone owns its own
        // checkpoint and must not claim ownership over the parent's.
        assert!(grandparent.final_checkpoint_id.is_some());
        assert_ne!(
            grandparent.final_checkpoint_id,
            Some(grandparent_final_cp),
            "inherited final_checkpoint_id must be regenerated"
        );

        // All three SSTs must resolve to their correct source paths
        let external_ssts = union.external_ssts();
        assert_eq!(
            external_ssts.get(&parent1_sst1),
            Some(&Path::from("/tmp/db1"))
        );
        assert_eq!(
            external_ssts.get(&parent2_sst1),
            Some(&Path::from("/tmp/db2"))
        );
        assert_eq!(
            external_ssts.get(&grandparent_sst),
            Some(&Path::from("/tmp/grandparent"))
        );
    }

    fn new_checkpoint(id: Uuid) -> Checkpoint {
        Checkpoint {
            id,
            manifest_id: 1,
            create_time: DefaultSystemClock::new().now(),
            expire_time: None,
            name: None,
        }
    }

    #[test]
    fn test_range_includes_compacted_ssts() {
        let manifest = build_manifest(
            &SimpleManifest::new(
                vec![],
                vec![(
                    0,
                    vec![
                        SstEntry::projected("sr_a", "a", "a".."m"),
                        SstEntry::projected("sr_n", "n", "m"..),
                    ],
                )],
            ),
            |_| SsTableId::from(Ulid::new()),
        );
        let range = manifest
            .range()
            .expect("range should be Some for manifest with sorted runs");
        assert_eq!(range.start_bound(), Bound::Included(&Bytes::from("a")));
        assert_eq!(range.end_bound(), Bound::Unbounded);
    }

    fn build_manifest<F>(manifest: &SimpleManifest, mut sst_id_fn: F) -> Manifest
    where
        F: FnMut(&str) -> SsTableId,
    {
        let mut core = ManifestCore::new();
        for entry in &manifest.l0 {
            let sst_id = sst_id_fn(entry.sst_alias);
            let view_id = sst_id.value();
            Arc::make_mut(&mut core.tree)
                .l0
                .push_back(SsTableView::new_projected(
                    view_id,
                    SsTableHandle::new(
                        sst_id,
                        SST_FORMAT_VERSION_LATEST,
                        SsTableInfo {
                            first_entry: Some(entry.first_entry.clone()),
                            ..SsTableInfo::default()
                        },
                    ),
                    entry.visible_range.clone(),
                ));
        }
        for (idx, sorted_run) in manifest.sorted_runs.iter().enumerate() {
            Arc::make_mut(&mut core.tree).compacted.push(SortedRun::new(
                idx as u32,
                sorted_run.iter().map(|entry| {
                    let sst_id = sst_id_fn(entry.sst_alias);
                    let view_id = sst_id.value();
                    SsTableView::new_projected(
                        view_id,
                        SsTableHandle::new(
                            sst_id,
                            SST_FORMAT_VERSION_LATEST,
                            SsTableInfo {
                                first_entry: Some(entry.first_entry.clone()),
                                ..SsTableInfo::default()
                            },
                        ),
                        entry.visible_range.clone(),
                    )
                }),
            ));
        }
        Manifest::initial(core)
    }

    fn assert_manifest_equal(
        actual: &Manifest,
        expected: &Manifest,
        sst_ids: &HashMap<String, SsTableId>,
    ) {
        let sst_aliases: HashMap<SsTableId, String> =
            sst_ids.iter().map(|(k, v)| (*v, k.clone())).collect();

        if actual.core.tree.l0 != expected.core.tree.l0 {
            let mut error_msg = String::from("Manifest L0 mismatch.\n\nActual: \n");

            // Format actual L0 entries
            for (idx, handle) in actual.core.tree.l0.iter().enumerate() {
                let id_str = sst_aliases
                    .get(&handle.sst.id)
                    .map(|a| a.as_str())
                    .unwrap_or("UNKNOWN");

                let first_entry = handle
                    .sst
                    .info
                    .first_entry
                    .as_ref()
                    .map(|k| format!("{:?}", k))
                    .unwrap();

                let visible_range = handle
                    .visible_range
                    .as_ref()
                    .map(format_range)
                    .unwrap_or_else(|| "None".to_string());

                let result = if expected.core.tree.l0.get(idx) == Some(handle) {
                    ""
                } else {
                    " --> Unexpected"
                };

                error_msg.push_str(&format!(
                    "{}. {} (first_entry: {}, visible_range: {}){}\n",
                    idx + 1,
                    id_str,
                    first_entry,
                    visible_range,
                    result
                ));
            }

            error_msg.push_str("\nExpected: \n");

            // Format expected L0 entries
            for (idx, handle) in expected.core.tree.l0.iter().enumerate() {
                let id_str = sst_aliases.get(&handle.sst.id).unwrap();

                let first_entry = handle
                    .sst
                    .info
                    .first_entry
                    .as_ref()
                    .map(|k| format!("{:?}", k))
                    .unwrap();

                let visible_range = handle
                    .visible_range
                    .as_ref()
                    .map(format_range)
                    .unwrap_or_else(|| "None".to_string());

                error_msg.push_str(&format!(
                    "{}. {} (first_entry: {}, visible_range: {})\n",
                    idx + 1,
                    id_str,
                    first_entry,
                    visible_range
                ));
            }

            panic!("{}", error_msg);
        }

        assert_eq!(
            actual.core.tree.compacted, expected.core.tree.compacted,
            "Sorted runs do not match."
        );
    }

    fn format_range(range: &BytesRange) -> String {
        let start = match range.start_bound() {
            Bound::Included(start) => format!("={:?}", start),
            Bound::Excluded(start) => format!("{:?}", start),
            Bound::Unbounded => "".to_string(),
        };
        let end = match range.end_bound() {
            Bound::Included(end) => format!("={:?}", end),
            Bound::Excluded(end) => format!("{:?}", end),
            Bound::Unbounded => "".to_string(),
        };
        format!("{}..{}", start, end)
    }

    #[test]
    fn test_projected_drops_unused_external_dbs() {
        let projection_range = BytesRange::from_ref("a".."b");

        let sst_id_1 = SsTableId::from(Ulid::new());
        let sst_id_2 = SsTableId::from(Ulid::new());
        let sst_id_3 = SsTableId::from(Ulid::new());
        let sst_id_4 = SsTableId::from(Ulid::new());

        let mut core = ManifestCore::new();

        Arc::make_mut(&mut core.tree)
            .l0
            .push_back(create_sst_view(sst_id_1, b"a")); // inside projection_range
        Arc::make_mut(&mut core.tree)
            .l0
            .push_back(create_sst_view(sst_id_2, b"c")); // outside projection_range
        Arc::make_mut(&mut core.tree)
            .l0
            .push_back(create_sst_view(sst_id_3, b"d")); // outside projection_range
        Arc::make_mut(&mut core.tree)
            .l0
            .push_back(create_sst_view(sst_id_4, b"e")); // outside projection_range

        let mut manifest = Manifest::initial(core);

        manifest.external_dbs = vec![
            ExternalDb {
                path: "/path/to/db1".to_string(),
                source_checkpoint_id: Uuid::new_v4(),
                final_checkpoint_id: None,
                sst_ids: vec![sst_id_1, sst_id_2],
            },
            ExternalDb {
                path: "/path/to/db2".to_string(),
                source_checkpoint_id: Uuid::new_v4(),
                final_checkpoint_id: None,
                sst_ids: vec![sst_id_3, sst_id_4],
            },
        ];

        assert_eq!(manifest.external_dbs.len(), 2);

        let projected = Manifest::projected(
            &manifest,
            &ProjectionConfig::from_global_range(projection_range),
        )
        .unwrap();

        assert_eq!(projected.external_dbs.len(), 1);
        assert_eq!(projected.external_dbs[0].path, "/path/to/db1");
        // The retained entry must have its out-of-range ID (sst_id_2) trimmed.
        // Carrying it forward would keep the parent SST pinned in detach GC even
        // though the projected tree no longer references it.
        assert_eq!(projected.external_dbs[0].sst_ids, vec![sst_id_1]);
    }

    #[test]
    fn test_prune_external_sst_ids_shrinks_and_keeps_entries() {
        let live_l0 = SsTableId::from(Ulid::new());
        let live_compacted = SsTableId::from(Ulid::new());
        let stale_a = SsTableId::from(Ulid::new());
        let stale_b = SsTableId::from(Ulid::new());

        let mut core = ManifestCore::new();
        Arc::make_mut(&mut core.tree)
            .l0
            .push_back(create_sst_view(live_l0, b"a"));
        Arc::make_mut(&mut core.tree)
            .compacted
            .push(SortedRun::new(0, [create_sst_view(live_compacted, b"b")]));

        let mut manifest = Manifest::initial(core);
        manifest.external_dbs = vec![
            // Mix of live and stale IDs: stale ones should be pruned, live kept.
            ExternalDb {
                path: "/path/to/partially_referenced".to_string(),
                source_checkpoint_id: Uuid::new_v4(),
                final_checkpoint_id: Some(Uuid::new_v4()),
                sst_ids: vec![live_l0, stale_a, live_compacted],
            },
            // No live IDs: entry must be retained (with empty sst_ids) so that GC can
            // later detach using the final_checkpoint_id.
            ExternalDb {
                path: "/path/to/fully_compacted".to_string(),
                source_checkpoint_id: Uuid::new_v4(),
                final_checkpoint_id: Some(Uuid::new_v4()),
                sst_ids: vec![stale_a, stale_b],
            },
        ];

        manifest.prune_external_sst_ids();

        assert_eq!(manifest.external_dbs.len(), 2);
        assert_eq!(
            manifest.external_dbs[0].sst_ids,
            vec![live_l0, live_compacted]
        );
        assert!(manifest.external_dbs[1].sst_ids.is_empty());
        assert!(manifest.external_dbs[1].final_checkpoint_id.is_some());
    }

    #[test]
    fn test_prune_external_sst_ids_retains_segment_referenced_ssts() {
        // Regression: prune_external_sst_ids must walk segment trees in
        // addition to the unsegmented tree. Otherwise SSTs referenced
        // only by a segment get treated as stale and dropped from
        // external_dbs.sst_ids, which would let parent detach/GC remove
        // files the clone still needs.
        let unsegmented_l0 = SsTableId::from(Ulid::new());
        let segment_l0 = SsTableId::from(Ulid::new());
        let segment_compacted = SsTableId::from(Ulid::new());
        let stale = SsTableId::from(Ulid::new());

        let mut core = ManifestCore::new();
        Arc::make_mut(&mut core.tree)
            .l0
            .push_back(create_sst_view(unsegmented_l0, b"a"));
        core.segments = vec![Segment {
            prefix: Bytes::from_static(b"seg/"),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![create_sst_view(segment_l0, b"seg/a")]),
                compacted: vec![SortedRun::new(
                    0,
                    [create_sst_view(segment_compacted, b"seg/b")],
                )],
            }),
        }];

        let mut manifest = Manifest::initial(core);
        manifest.external_dbs = vec![ExternalDb {
            path: "/path/to/parent".to_string(),
            source_checkpoint_id: Uuid::new_v4(),
            final_checkpoint_id: Some(Uuid::new_v4()),
            sst_ids: vec![unsegmented_l0, segment_l0, segment_compacted, stale],
        }];

        manifest.prune_external_sst_ids();

        assert_eq!(manifest.external_dbs.len(), 1);
        let retained: HashSet<SsTableId> =
            manifest.external_dbs[0].sst_ids.iter().copied().collect();
        let expected: HashSet<SsTableId> = [unsegmented_l0, segment_l0, segment_compacted]
            .into_iter()
            .collect();
        assert_eq!(retained, expected);
    }

    fn create_sst_view(sst_id: SsTableId, first_entry_bytes: &'static [u8]) -> SsTableView {
        SsTableView::new_projected(
            sst_id.value(),
            SsTableHandle::new(
                sst_id,
                SST_FORMAT_VERSION_LATEST,
                SsTableInfo {
                    first_entry: Some(Bytes::from_static(first_entry_bytes)),
                    ..SsTableInfo::default()
                },
            ),
            None,
        )
    }

    fn manifest_with_one_compacted_sst(
        sst_id: SsTableId,
        first_entry: &'static [u8],
        visible_range: BytesRange,
    ) -> Manifest {
        let mut core = ManifestCore::new();
        Arc::make_mut(&mut core.tree).compacted.push(SortedRun::new(
            0,
            [SsTableView::new_projected(
                sst_id.value(),
                SsTableHandle::new(
                    sst_id,
                    SST_FORMAT_VERSION_LATEST,
                    SsTableInfo {
                        first_entry: Some(Bytes::from_static(first_entry)),
                        ..SsTableInfo::default()
                    },
                ),
                Some(visible_range),
            )],
        ));
        Manifest::initial(core)
    }

    #[test]
    fn test_union_collapses_identical_borrows_of_same_ancestor() {
        let shared_path = "shared_ancestor".to_string();
        let shared_source_cp = Uuid::new_v4();
        let original_final_cp = Uuid::new_v4();

        let sst_a = SsTableId::from(Ulid::from_parts(1000, 0));
        let sst_b = SsTableId::from(Ulid::from_parts(1001, 0));

        // Two sources borrow the SAME sst_ids from the SAME ancestor. They must
        // collapse to a single external_dbs entry.
        let sst_own1 = SsTableId::from(Ulid::from_parts(2000, 0));
        let mut m1 =
            manifest_with_one_compacted_sst(sst_own1, b"a", BytesRange::from_ref("a".."m"));
        m1.external_dbs.push(ExternalDb {
            path: shared_path.clone(),
            source_checkpoint_id: shared_source_cp,
            final_checkpoint_id: Some(original_final_cp),
            sst_ids: vec![sst_a, sst_b],
        });

        let sst_own2 = SsTableId::from(Ulid::from_parts(3000, 0));
        let mut m2 = manifest_with_one_compacted_sst(sst_own2, b"m", BytesRange::from_ref("m"..));
        m2.external_dbs.push(ExternalDb {
            path: shared_path.clone(),
            source_checkpoint_id: shared_source_cp,
            final_checkpoint_id: Some(original_final_cp),
            sst_ids: vec![sst_b, sst_a],
        });

        let rand = Arc::new(DbRand::default());
        let sources = vec![
            CloneSource {
                manifest: m1,
                path: Path::from("/tmp/db1"),
                checkpoint: new_checkpoint(Uuid::new_v4()),
            },
            CloneSource {
                manifest: m2,
                path: Path::from("/tmp/db2"),
                checkpoint: new_checkpoint(Uuid::new_v4()),
            },
        ];

        let result = Manifest::cloned_from_union(sources, rand).unwrap();

        let shared_entries: Vec<_> = result
            .external_dbs
            .iter()
            .filter(|db| db.path == shared_path)
            .collect();
        assert_eq!(
            shared_entries.len(),
            1,
            "identical borrows of the same ancestor must collapse to one entry"
        );
        // Carry-forward replaces the user-supplied source_cp with the parent's
        // final_checkpoint_id, and no entry references the user-supplied cp.
        assert_eq!(shared_entries[0].source_checkpoint_id, original_final_cp);
        assert!(result
            .external_dbs
            .iter()
            .all(|db| db.source_checkpoint_id != shared_source_cp));

        let merged_ids: BTreeSet<SsTableId> = shared_entries[0].sst_ids.iter().copied().collect();
        assert_eq!(merged_ids, BTreeSet::from([sst_a, sst_b]));
    }

    #[test]
    fn test_union_keeps_distinct_sst_sets_of_same_ancestor_separate() {
        // Two borrows of the same ancestor with DIFFERENT sst_ids must NOT be
        // merged: each pins a distinct set of physical SSTs, so collapsing them
        // would conflate distinct borrows. All SSTs must remain resolvable.
        let shared_path = "shared_ancestor".to_string();
        let final_cp = Uuid::from_u128(7);

        let sst_a = SsTableId::from(Ulid::from_parts(1000, 0));
        let sst_b = SsTableId::from(Ulid::from_parts(1001, 0));
        let sst_c = SsTableId::from(Ulid::from_parts(1002, 0));

        let sst_own1 = SsTableId::from(Ulid::from_parts(2000, 0));
        let mut m1 =
            manifest_with_one_compacted_sst(sst_own1, b"a", BytesRange::from_ref("a".."m"));
        m1.external_dbs.push(ExternalDb {
            path: shared_path.clone(),
            source_checkpoint_id: Uuid::from_u128(70),
            final_checkpoint_id: Some(final_cp),
            sst_ids: vec![sst_a, sst_b],
        });

        let sst_own2 = SsTableId::from(Ulid::from_parts(3000, 0));
        let mut m2 = manifest_with_one_compacted_sst(sst_own2, b"m", BytesRange::from_ref("m"..));
        m2.external_dbs.push(ExternalDb {
            path: shared_path.clone(),
            source_checkpoint_id: Uuid::from_u128(71),
            final_checkpoint_id: Some(final_cp),
            sst_ids: vec![sst_b, sst_c],
        });

        let sources = vec![
            CloneSource {
                manifest: m1,
                path: Path::from("/tmp/db1"),
                checkpoint: new_checkpoint(Uuid::new_v4()),
            },
            CloneSource {
                manifest: m2,
                path: Path::from("/tmp/db2"),
                checkpoint: new_checkpoint(Uuid::new_v4()),
            },
        ];

        let result = Manifest::cloned_from_union(sources, Arc::new(DbRand::default())).unwrap();

        let shared_entries: Vec<_> = result
            .external_dbs
            .iter()
            .filter(|db| db.path == shared_path)
            .collect();
        assert_eq!(
            shared_entries.len(),
            2,
            "borrows with distinct sst_ids must not be merged"
        );
        // No SST is lost across the two entries.
        let all_ids: BTreeSet<SsTableId> = shared_entries
            .iter()
            .flat_map(|db| db.sst_ids.iter().copied())
            .collect();
        assert_eq!(all_ids, BTreeSet::from([sst_a, sst_b, sst_c]));
    }

    /// Reproduces the clone-union `external_dbs` accumulation bug: two clones
    /// created independently from the SAME physical ancestor each carry a borrow
    /// of that ancestor with IDENTICAL `sst_ids` but DIFFERENT
    /// `source_checkpoint_id`s. The latter happens because `cloned_from_union`
    /// stamps every carried-forward borrow with a freshly generated
    /// `final_checkpoint_id` (`gen_uuid()`), and that final becomes the borrow's
    /// `source_checkpoint_id` on the next generation. Keyed on
    /// `(path, source_checkpoint_id)`, the two borrows do NOT collapse and
    /// `external_dbs` grows 1:1 with union fan-in. Keyed on `(path, sst_ids)`,
    /// identical borrows collapse to a single entry.
    #[test]
    fn repro_union_collapses_shared_ancestor_with_distinct_finals() {
        let shared_path = "shared_ancestor".to_string();
        let sst_a = SsTableId::from(Ulid::from_parts(1000, 0));
        let sst_b = SsTableId::from(Ulid::from_parts(1001, 0));

        // Distinct finals: each independent clone rotated its own final for the
        // shared ancestor. After carry-forward these become the borrows'
        // (differing) source_checkpoint_ids. The first-folded source carries
        // the LARGER final so the min tie-break must actually update the
        // stored id, not just keep the first insert.
        let final_cp_1 = Uuid::from_u128(2);
        let final_cp_2 = Uuid::from_u128(1);

        let sst_own1 = SsTableId::from(Ulid::from_parts(2000, 0));
        let mut m1 =
            manifest_with_one_compacted_sst(sst_own1, b"a", BytesRange::from_ref("a".."m"));
        m1.external_dbs.push(ExternalDb {
            path: shared_path.clone(),
            source_checkpoint_id: Uuid::from_u128(101),
            final_checkpoint_id: Some(final_cp_1),
            sst_ids: vec![sst_a, sst_b],
        });

        let sst_own2 = SsTableId::from(Ulid::from_parts(3000, 0));
        let mut m2 = manifest_with_one_compacted_sst(sst_own2, b"m", BytesRange::from_ref("m"..));
        m2.external_dbs.push(ExternalDb {
            path: shared_path.clone(),
            source_checkpoint_id: Uuid::from_u128(102),
            final_checkpoint_id: Some(final_cp_2),
            sst_ids: vec![sst_a, sst_b],
        });

        let sources = vec![
            CloneSource {
                manifest: m1,
                path: Path::from("/tmp/db1"),
                checkpoint: new_checkpoint(Uuid::new_v4()),
            },
            CloneSource {
                manifest: m2,
                path: Path::from("/tmp/db2"),
                checkpoint: new_checkpoint(Uuid::new_v4()),
            },
        ];

        let result = Manifest::cloned_from_union(sources, Arc::new(DbRand::default())).unwrap();

        let shared_entries: Vec<_> = result
            .external_dbs
            .iter()
            .filter(|db| db.path == shared_path)
            .collect();
        assert_eq!(
            shared_entries.len(),
            1,
            "borrows of the same ancestor with identical sst_ids must collapse \
             regardless of differing (rotated) checkpoint ids"
        );
        assert_eq!(
            shared_entries[0].source_checkpoint_id, final_cp_2,
            "with no user-supplied id in the group, the smallest generated id must be kept"
        );
        let ids: BTreeSet<SsTableId> = shared_entries[0].sst_ids.iter().copied().collect();
        assert_eq!(ids, BTreeSet::from([sst_a, sst_b]));
    }

    /// Same defect, demonstrating the unbounded growth directly: N independent
    /// clones of the same ancestor (each with a distinct rotated final but
    /// identical `sst_ids`) must still collapse to a single `external_dbs` entry
    /// rather than producing N entries.
    #[test]
    fn repro_union_shared_ancestor_does_not_grow_with_number_of_sources() {
        let shared_path = "shared_ancestor".to_string();
        let sst_a = SsTableId::from(Ulid::from_parts(1000, 0));
        let sst_b = SsTableId::from(Ulid::from_parts(1001, 0));

        // Three non-overlapping key ranges so the union is valid.
        let ranges = [
            ("a", BytesRange::from_ref("a".."i")),
            ("i", BytesRange::from_ref("i".."r")),
            ("r", BytesRange::from_ref("r"..)),
        ];

        let sources: Vec<CloneSource> = ranges
            .iter()
            .enumerate()
            .map(|(i, (first, range))| {
                let owned = SsTableId::from(Ulid::from_parts(2000 + i as u64, 0));
                let mut m = manifest_with_one_compacted_sst(
                    owned,
                    first.as_bytes().to_vec().leak(),
                    range.clone(),
                );
                m.external_dbs.push(ExternalDb {
                    path: shared_path.clone(),
                    source_checkpoint_id: Uuid::from_u128(200 + i as u128),
                    // distinct rotated final per independent clone
                    final_checkpoint_id: Some(Uuid::from_u128(1 + i as u128)),
                    sst_ids: vec![sst_a, sst_b],
                });
                CloneSource {
                    manifest: m,
                    path: Path::from(format!("/tmp/db{}", i)),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                }
            })
            .collect();

        let result = Manifest::cloned_from_union(sources, Arc::new(DbRand::default())).unwrap();

        let shared_entries = result
            .external_dbs
            .iter()
            .filter(|db| db.path == shared_path)
            .count();
        assert_eq!(
            shared_entries, 1,
            "external_dbs must not grow 1:1 with union fan-in for a shared ancestor"
        );
    }

    /// A union source may itself be the ancestor another source borrows from:
    /// the direct source's owned entry (keyed by the user-supplied checkpoint
    /// id from the clone source spec) and the carried borrow (keyed by a
    /// rotated final) can then collapse when their sst_ids match. The
    /// user-supplied checkpoint id must survive the collapse — clone retries
    /// re-validate the spec against the stored manifest
    /// (validate_attached_to_external_db) and would otherwise fail permanently
    /// with CloneIncorrectExternalDbCheckpoint. Sources are folded in
    /// key-range order, so the parent's range position controls whether its
    /// owned entry is merged before the carried borrow (the borrow must not
    /// replace the stored id) or after it (the owned entry must overwrite the
    /// stored carried final).
    #[rstest]
    #[case::parent_folded_first(true)]
    #[case::borrow_folded_first(false)]
    fn test_union_collapse_keeps_user_supplied_checkpoint_id(#[case] parent_folded_first: bool) {
        let parent_path = Path::from("/tmp/parent");
        let parent_path_str: String = parent_path.clone().into();

        // Byte-wise largest so a plain min() would discard it in favor of the
        // carried final below.
        let user_cp = Uuid::from_u128(u128::MAX);
        let carried_final_cp = Uuid::from_u128(1);

        let lower = (b"a" as &'static [u8], BytesRange::from_ref("a".."m"));
        let upper = (b"m" as &'static [u8], BytesRange::from_ref("m"..));
        let (parent_range, descendant_range) = if parent_folded_first {
            (lower, upper)
        } else {
            (upper, lower)
        };

        // Direct source: the parent itself, owning a single SST.
        let sst_p = SsTableId::from(Ulid::from_parts(1000, 0));
        let parent_manifest =
            manifest_with_one_compacted_sst(sst_p, parent_range.0, parent_range.1);

        // Descendant clone of the parent, borrowing the exact same SSTs.
        let sst_own = SsTableId::from(Ulid::from_parts(2000, 0));
        let mut descendant_manifest =
            manifest_with_one_compacted_sst(sst_own, descendant_range.0, descendant_range.1);
        descendant_manifest.external_dbs.push(ExternalDb {
            path: parent_path_str.clone(),
            source_checkpoint_id: Uuid::from_u128(100),
            final_checkpoint_id: Some(carried_final_cp),
            sst_ids: vec![sst_p],
        });

        let sources = vec![
            CloneSource {
                manifest: parent_manifest,
                path: parent_path,
                checkpoint: new_checkpoint(user_cp),
            },
            CloneSource {
                manifest: descendant_manifest,
                path: Path::from("/tmp/descendant"),
                checkpoint: new_checkpoint(Uuid::new_v4()),
            },
        ];

        let result = Manifest::cloned_from_union(sources, Arc::new(DbRand::default())).unwrap();

        let parent_entries: Vec<_> = result
            .external_dbs
            .iter()
            .filter(|db| db.path == parent_path_str)
            .collect();
        assert_eq!(
            parent_entries.len(),
            1,
            "owned entry and identical carried borrow must still collapse"
        );
        assert_eq!(
            parent_entries[0].source_checkpoint_id, user_cp,
            "the direct source's user-supplied checkpoint id must survive the collapse"
        );
        let ids: BTreeSet<SsTableId> = parent_entries[0].sst_ids.iter().copied().collect();
        assert_eq!(ids, BTreeSet::from([sst_p]));
    }

    #[test]
    fn test_union_orders_external_dbs_and_sst_ids_deterministically() {
        let source1_cp = Uuid::from_u128(100);
        let source2_cp = Uuid::from_u128(200);
        let ancestor_a_final_cp = Uuid::from_u128(10);
        let ancestor_z_final_cp = Uuid::from_u128(30);

        let ancestor_a_sst1 = SsTableId::from(Ulid::from_parts(1000, 0));
        let ancestor_a_sst2 = SsTableId::from(Ulid::from_parts(1001, 0));
        let ancestor_z_sst1 = SsTableId::from(Ulid::from_parts(3000, 0));
        let ancestor_z_sst2 = SsTableId::from(Ulid::from_parts(3001, 0));
        let source1_sst = SsTableId::from(Ulid::from_parts(5000, 0));
        let source2_sst = SsTableId::from(Ulid::from_parts(6000, 0));

        let mut manifest1 =
            manifest_with_one_compacted_sst(source1_sst, b"a", BytesRange::from_ref("a".."m"));
        manifest1.external_dbs.push(ExternalDb {
            path: "z-parent".to_string(),
            source_checkpoint_id: Uuid::from_u128(31),
            final_checkpoint_id: Some(ancestor_z_final_cp),
            sst_ids: vec![ancestor_z_sst2, ancestor_z_sst1],
        });

        let mut manifest2 =
            manifest_with_one_compacted_sst(source2_sst, b"m", BytesRange::from_ref("m"..));
        manifest2.external_dbs.push(ExternalDb {
            path: "a-parent".to_string(),
            source_checkpoint_id: Uuid::from_u128(11),
            final_checkpoint_id: Some(ancestor_a_final_cp),
            sst_ids: vec![ancestor_a_sst2, ancestor_a_sst1],
        });

        let union = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: manifest1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(source1_cp),
                },
                CloneSource {
                    manifest: manifest2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(source2_cp),
                },
            ],
            Arc::new(DbRand::default()),
        )
        .unwrap();

        let actual: Vec<_> = union
            .external_dbs
            .iter()
            .map(|db| {
                (
                    db.path.as_str(),
                    db.source_checkpoint_id,
                    db.sst_ids.clone(),
                )
            })
            .collect();
        assert_eq!(
            actual,
            vec![
                (
                    "a-parent",
                    ancestor_a_final_cp,
                    vec![ancestor_a_sst1, ancestor_a_sst2],
                ),
                ("tmp/db1", source1_cp, vec![source1_sst]),
                ("tmp/db2", source2_cp, vec![source2_sst]),
                (
                    "z-parent",
                    ancestor_z_final_cp,
                    vec![ancestor_z_sst1, ancestor_z_sst2],
                ),
            ]
        );
    }

    fn segment_with_prefix(prefix: &[u8], seed: u64) -> Segment {
        let view_id = Ulid::from_parts(seed, 0);
        let handle = SsTableHandle::new(
            SsTableId::from(Ulid::from_parts(seed, 1)),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo::default(),
        );
        Segment {
            prefix: Bytes::copy_from_slice(prefix),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![SsTableView::new(view_id, handle)]),
                compacted: vec![],
            }),
        }
    }

    fn collect_prefixes(segments: &[Segment]) -> Vec<Bytes> {
        segments.iter().map(|s| s.prefix.clone()).collect()
    }

    #[test]
    fn test_select_segments_unconfigured_returns_unsegmented_tree() {
        // No segments configured -> select_segments returns None;
        // callers fall back to default_segment().
        let core = ManifestCore::new();
        assert!(core.select_segments(&BytesRange::unbounded()).is_none());
        let ds = core.default_segment();
        assert_eq!(ds.prefix, Bytes::new());
    }

    #[test]
    fn test_select_segments_point_inside_segment() {
        // Point query whose key starts with a segment prefix routes to
        // exactly that segment's tree.
        let mut core = ManifestCore::new();
        core.segments = vec![
            segment_with_prefix(b"a/", 1),
            segment_with_prefix(b"b/", 2),
            segment_with_prefix(b"c/", 3),
        ];
        let range = BytesRange::from_slice(b"b/k".as_ref()..=b"b/k".as_ref());
        let segments = core.select_segments(&range).unwrap();
        assert_eq!(collect_prefixes(segments), vec![Bytes::from_static(b"b/")]);
    }

    #[test]
    fn test_select_segments_point_outside_any_segment() {
        // Point query that falls in a gap between segment prefixes routes
        // to nothing — the key cannot exist in this database.
        let mut core = ManifestCore::new();
        core.segments = vec![segment_with_prefix(b"a/", 1), segment_with_prefix(b"c/", 3)];
        let range = BytesRange::from_slice(b"b/k".as_ref()..=b"b/k".as_ref());
        let segments = core.select_segments(&range).unwrap();
        assert!(segments.is_empty());
    }

    #[test]
    fn test_select_segments_range_overlapping_contiguous_segments() {
        // Range query collects every overlapping segment in prefix order.
        let mut core = ManifestCore::new();
        core.segments = vec![
            segment_with_prefix(b"a/", 1),
            segment_with_prefix(b"b/", 2),
            segment_with_prefix(b"c/", 3),
            segment_with_prefix(b"d/", 4),
        ];
        let range = BytesRange::from_slice(b"b/".as_ref()..b"d/".as_ref());
        let segments = core.select_segments(&range).unwrap();
        assert_eq!(
            collect_prefixes(segments),
            vec![Bytes::from_static(b"b/"), Bytes::from_static(b"c/")]
        );
    }

    #[test]
    fn test_select_segments_range_outside_all_segments() {
        // Range that falls entirely outside every segment interval routes
        // to no trees.
        let mut core = ManifestCore::new();
        core.segments = vec![segment_with_prefix(b"a/", 1), segment_with_prefix(b"c/", 3)];
        let range = BytesRange::from_slice(b"x/".as_ref()..b"y/".as_ref());
        let segments = core.select_segments(&range).unwrap();
        assert!(segments.is_empty());
    }

    #[test]
    fn test_select_segments_unbounded_range_returns_all_segments() {
        // Unbounded range covers every segment interval.
        let mut core = ManifestCore::new();
        core.segments = vec![
            segment_with_prefix(b"a/", 1),
            segment_with_prefix(b"b/", 2),
            segment_with_prefix(b"c/", 3),
        ];
        let segments = core.select_segments(&BytesRange::unbounded()).unwrap();
        assert_eq!(
            collect_prefixes(segments),
            vec![
                Bytes::from_static(b"a/"),
                Bytes::from_static(b"b/"),
                Bytes::from_static(b"c/"),
            ]
        );
    }

    #[test]
    fn test_select_segments_excluded_hi_at_segment_boundary() {
        // For Excluded(hi), a segment whose prefix == hi is skipped — its
        // interval starts exactly at hi, which is outside the half-open
        // query.
        let mut core = ManifestCore::new();
        core.segments = vec![
            segment_with_prefix(b"a/", 1),
            segment_with_prefix(b"b/", 2),
            segment_with_prefix(b"c/", 3),
        ];
        let range = BytesRange::from_slice(b"a/".as_ref()..b"b/".as_ref());
        let segments = core.select_segments(&range).unwrap();
        assert_eq!(collect_prefixes(segments), vec![Bytes::from_static(b"a/")]);
    }

    #[test]
    fn test_select_segments_included_hi_at_segment_boundary() {
        // For Included(hi), a segment whose prefix == hi is included — its
        // interval starts at hi, which is inside the closed query.
        let mut core = ManifestCore::new();
        core.segments = vec![
            segment_with_prefix(b"a/", 1),
            segment_with_prefix(b"b/", 2),
            segment_with_prefix(b"c/", 3),
        ];
        let range = BytesRange::from_slice(b"a/".as_ref()..=b"b/".as_ref());
        let segments = core.select_segments(&range).unwrap();
        assert_eq!(
            collect_prefixes(segments),
            vec![Bytes::from_static(b"a/"), Bytes::from_static(b"b/")]
        );
    }

    #[test]
    fn test_select_segments_lo_inside_segment_interval() {
        // `lo` lands deep inside a segment's interval; that segment must
        // be included even though its prefix is strictly less than `lo`.
        let mut core = ManifestCore::new();
        core.segments = vec![
            segment_with_prefix(b"a/", 1),
            segment_with_prefix(b"b/", 2),
            segment_with_prefix(b"c/", 3),
        ];
        let range = BytesRange::from_slice(b"b/middle".as_ref()..b"d/".as_ref());
        let segments = core.select_segments(&range).unwrap();
        assert_eq!(
            collect_prefixes(segments),
            vec![Bytes::from_static(b"b/"), Bytes::from_static(b"c/")]
        );
    }

    #[test]
    fn test_select_segments_matches_brute_force_filter() {
        // Property: binary-search routing agrees with a naive
        // intersect-based filter on every (segments, range) shape we
        // could generate. Catches off-by-ones around the lower-bound
        // antichain check and the included-vs-excluded upper bound.
        use proptest::prelude::*;

        fn brute_force(core: &ManifestCore, range: &BytesRange) -> Option<Vec<Bytes>> {
            if core.segments.is_empty() {
                return None;
            }
            Some(
                core.segments
                    .iter()
                    .filter(|s| {
                        BytesRange::from_prefix(&s.prefix)
                            .intersect(range)
                            .is_some()
                    })
                    .map(|s| s.prefix.clone())
                    .collect(),
            )
        }

        // Antichain-respecting prefix universe: short, distinct, and
        // none is a prefix of another.
        let prefixes = ["a/", "b/", "c/", "d/", "e/"];

        proptest!(|(
            mask in proptest::collection::vec(any::<bool>(), prefixes.len()),
            lo_kind in 0u8..3,
            hi_kind in 0u8..3,
            lo_key in "[a-f][/0-9]{0,3}",
            hi_key in "[a-f][/0-9]{0,3}",
        )| {
            let mut core = ManifestCore::new();
            core.segments = mask
                .iter()
                .zip(prefixes.iter())
                .enumerate()
                .filter(|(_, (keep, _))| **keep)
                .map(|(i, (_, p))| segment_with_prefix(p.as_bytes(), i as u64 + 1))
                .collect();

            let lo_bytes = Bytes::copy_from_slice(lo_key.as_bytes());
            let hi_bytes = Bytes::copy_from_slice(hi_key.as_bytes());
            let lo_bound = match lo_kind {
                0 => Bound::Unbounded,
                1 => Bound::Included(lo_bytes.clone()),
                _ => Bound::Excluded(lo_bytes.clone()),
            };
            let hi_bound = match hi_kind {
                0 => Bound::Unbounded,
                1 => Bound::Included(hi_bytes.clone()),
                _ => Bound::Excluded(hi_bytes.clone()),
            };
            let Some(range) = BytesRange::try_new(lo_bound, hi_bound) else {
                return Ok(());
            };

            let actual = core.select_segments(&range).map(collect_prefixes);
            let expected = brute_force(&core, &range);
            prop_assert_eq!(actual, expected);
        });
    }

    /// Helper for the segment-aware union tests: build a clone source with
    /// a single segment whose tree carries one L0 view and one sorted run.
    /// Each SST view is given an explicit `visible_range` so the
    /// non-overlap check in `cloned_from_union` sees disjoint sources.
    fn manifest_with_segment(
        prefix: &'static [u8],
        extractor_name: Option<&str>,
        first_entry: &'static [u8],
        visible_range: BytesRange,
    ) -> (Manifest, SsTableId, SsTableId) {
        let l0_sst = SsTableId::from(Ulid::new());
        let sr_sst = SsTableId::from(Ulid::new());
        let mut core = ManifestCore::new();
        core.segment_extractor_name = extractor_name.map(|s| s.to_string());
        core.segments = vec![Segment {
            prefix: Bytes::copy_from_slice(prefix),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: None,
                last_compacted_l0_sst_id: None,
                l0: VecDeque::from(vec![SsTableView::new_projected(
                    l0_sst.value(),
                    SsTableHandle::new(
                        l0_sst,
                        SST_FORMAT_VERSION_LATEST,
                        SsTableInfo {
                            first_entry: Some(Bytes::from_static(first_entry)),
                            ..SsTableInfo::default()
                        },
                    ),
                    Some(visible_range.clone()),
                )]),
                compacted: vec![SortedRun::new(
                    0, // gets renumbered globally by the union
                    [SsTableView::new_projected(
                        sr_sst.value(),
                        SsTableHandle::new(
                            sr_sst,
                            SST_FORMAT_VERSION_LATEST,
                            SsTableInfo {
                                first_entry: Some(Bytes::from_static(first_entry)),
                                ..SsTableInfo::default()
                            },
                        ),
                        Some(visible_range),
                    )],
                )],
            }),
        }];
        (Manifest::initial(core), l0_sst, sr_sst)
    }

    #[test]
    fn test_union_carries_through_segments() {
        // Two non-overlapping sources each contribute a segment; the union
        // contains both, sorted by prefix.
        let (m1, _, _) = manifest_with_segment(
            b"hour=11/",
            Some("hour"),
            b"a",
            BytesRange::from_ref("a".."m"),
        );
        let (m2, _, _) =
            manifest_with_segment(b"hour=12/", Some("hour"), b"m", BytesRange::from_ref("m"..));

        let rand = Arc::new(DbRand::default());
        let union = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: m1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: m2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            rand,
        )
        .unwrap();

        assert_eq!(union.core.segment_extractor_name.as_deref(), Some("hour"));
        let prefixes: Vec<&Bytes> = union.core.segments.iter().map(|s| &s.prefix).collect();
        assert_eq!(
            prefixes,
            vec![
                &Bytes::from_static(b"hour=11/"),
                &Bytes::from_static(b"hour=12/")
            ]
        );
    }

    #[test]
    fn test_renumber_union_sorted_runs_assigns_unique_sequential_ids() {
        // Property: after `renumber_union_sorted_runs`, every sorted run
        // across the unsegmented tree and every segment has an id in
        // 0..N (one per run) with no duplicates. Walk order is
        // unsegmented first, then segments in `core.segments` order.
        // Ids descend in walk order, so within each tree the first list
        // entry has the highest id (matching the descending-id-by-list-
        // position convention).
        fn make_sr(id: u32) -> SortedRun {
            SortedRun::new(id, []) // intentionally collides across trees pre-renumber
        }

        let mut core = ManifestCore::new();
        // Unsegmented tree carries two SRs sharing the same id.
        Arc::make_mut(&mut core.tree).compacted = vec![make_sr(7), make_sr(7)];
        core.segments = vec![
            Segment {
                prefix: Bytes::from_static(b"hour=11/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::new(),
                    compacted: vec![make_sr(0), make_sr(7)],
                }),
            },
            Segment {
                prefix: Bytes::from_static(b"hour=12/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::new(),
                    compacted: vec![make_sr(0)],
                }),
            },
        ];

        Manifest::renumber_union_sorted_runs(&mut core);

        let ids: Vec<u32> = core
            .trees()
            .flat_map(|t| t.compacted.iter().map(|sr| sr.id))
            .collect();
        // 5 runs total → ids descend from N-1 down to 0 in walk order.
        assert_eq!(ids, vec![4, 3, 2, 1, 0]);
    }

    #[test]
    fn test_union_renumbers_sr_ids_globally_across_trees() {
        // Two sources, each with a single segment under the same
        // extractor. After union, the SR ids drawn from both segments
        // must be unique and sequential — exercises that renumber walks
        // multiple segment trees, not just one.
        let (m1, _, _) = manifest_with_segment(
            b"hour=11/",
            Some("hour"),
            b"a",
            BytesRange::from_ref("a".."m"),
        );
        let (m2, _, _) =
            manifest_with_segment(b"hour=12/", Some("hour"), b"m", BytesRange::from_ref("m"..));

        let union = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: m1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: m2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            Arc::new(DbRand::default()),
        )
        .unwrap();

        let mut all_sr_ids: Vec<u32> = union
            .core
            .trees()
            .flat_map(|t| t.compacted.iter().map(|sr| sr.id))
            .collect();
        all_sr_ids.sort();
        // 2 segments × 1 run each = 2 total, ids 0..1.
        assert_eq!(all_sr_ids, vec![0, 1]);
    }

    #[test]
    fn test_union_owned_ssts_includes_segment_ssts() {
        // The clone source's owned_ssts must enumerate SSTs from segments
        // too, otherwise the resulting external_db wouldn't list them and
        // GC could later reap them.
        use std::collections::HashSet;

        let (m, l0_sst, sr_sst) =
            manifest_with_segment(b"hour=12/", Some("hour"), b"a", BytesRange::from_ref("a"..));
        let union = Manifest::cloned_from_union(
            vec![CloneSource {
                manifest: m,
                path: Path::from("/tmp/db1"),
                checkpoint: new_checkpoint(Uuid::new_v4()),
            }],
            Arc::new(DbRand::default()),
        )
        .unwrap();

        let owned_external: HashSet<SsTableId> = union
            .external_dbs
            .iter()
            .filter(|db| db.path == "tmp/db1")
            .flat_map(|db| db.sst_ids.iter().copied())
            .collect();
        assert!(owned_external.contains(&l0_sst));
        assert!(owned_external.contains(&sr_sst));
    }

    #[test]
    fn test_union_drops_drain_marker_segments() {
        // A source whose `core.segments` mixes a data-bearing segment
        // with a drain-marker segment (no L0, no compacted, watermark
        // set). The drain marker carries no meaning in the resulting
        // clone and must not produce an empty entry in `core.segments`.
        let (mut m1, _, _) = manifest_with_segment(
            b"hour=11/",
            Some("hour"),
            b"a",
            BytesRange::from_ref("a".."m"),
        );
        // Add a drain-marker segment alongside the data-bearing one.
        m1.core.segments.push(Segment {
            prefix: Bytes::from_static(b"hour=99/"),
            tree: Arc::new(LsmTreeState {
                last_compacted_l0_sst_view_id: Some(Ulid::new()),
                last_compacted_l0_sst_id: Some(Ulid::new()),
                l0: VecDeque::new(),
                compacted: vec![],
            }),
        });
        // `manifest.core.segments` invariant: sorted by prefix.
        m1.core.segments.sort_by(|a, b| a.prefix.cmp(&b.prefix));

        let (m2, _, _) =
            manifest_with_segment(b"hour=12/", Some("hour"), b"m", BytesRange::from_ref("m"..));

        let union = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: m1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: m2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            Arc::new(DbRand::default()),
        )
        .unwrap();

        // Only the two data-bearing segments survive; the drain marker
        // is dropped.
        assert_eq!(union.core.segments.len(), 2);
        let prefixes: Vec<&Bytes> = union.core.segments.iter().map(|s| &s.prefix).collect();
        assert_eq!(
            prefixes,
            vec![
                &Bytes::from_static(b"hour=11/"),
                &Bytes::from_static(b"hour=12/"),
            ]
        );
        // No surviving segment is empty.
        assert!(union.core.segments.iter().all(|s| !s.tree.is_empty()));
    }

    #[test]
    fn test_union_concatenates_segments_with_shared_prefix() {
        // Two sources both have segment hour=12/, with disjoint key
        // ranges within that prefix. The union concatenates their L0
        // and compacted lists; SR ids are regenerated globally.
        let l0_a = SsTableId::from(Ulid::new());
        let sr_a = SsTableId::from(Ulid::new());
        let l0_b = SsTableId::from(Ulid::new());
        let sr_b = SsTableId::from(Ulid::new());

        fn segment_with_views(
            prefix: &'static [u8],
            l0_id: SsTableId,
            sr_id: SsTableId,
            first_entry: &'static [u8],
            range: BytesRange,
        ) -> Segment {
            Segment {
                prefix: Bytes::from_static(prefix),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::from(vec![SsTableView::new_projected(
                        l0_id.value(),
                        SsTableHandle::new(
                            l0_id,
                            SST_FORMAT_VERSION_LATEST,
                            SsTableInfo {
                                first_entry: Some(Bytes::from_static(first_entry)),
                                ..SsTableInfo::default()
                            },
                        ),
                        Some(range.clone()),
                    )]),
                    compacted: vec![SortedRun::new(
                        0,
                        [SsTableView::new_projected(
                            sr_id.value(),
                            SsTableHandle::new(
                                sr_id,
                                SST_FORMAT_VERSION_LATEST,
                                SsTableInfo {
                                    first_entry: Some(Bytes::from_static(first_entry)),
                                    ..SsTableInfo::default()
                                },
                            ),
                            Some(range),
                        )],
                    )],
                }),
            }
        }

        let mut core1 = ManifestCore::new();
        core1.segment_extractor_name = Some("hour".into());
        core1.segments = vec![segment_with_views(
            b"hour=12/",
            l0_a,
            sr_a,
            b"hour=12/00",
            BytesRange::from_ref("hour=12/00".."hour=12/30"),
        )];
        let m1 = Manifest::initial(core1);

        let mut core2 = ManifestCore::new();
        core2.segment_extractor_name = Some("hour".into());
        core2.segments = vec![segment_with_views(
            b"hour=12/",
            l0_b,
            sr_b,
            b"hour=12/30",
            BytesRange::from_ref("hour=12/30".."hour=12/59"),
        )];
        let m2 = Manifest::initial(core2);

        let union = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: m1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: m2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            Arc::new(DbRand::default()),
        )
        .unwrap();

        // One segment under hour=12/, with both sources' L0 and SRs
        // concatenated.
        assert_eq!(union.core.segments.len(), 1);
        let seg = &union.core.segments[0];
        assert_eq!(seg.prefix, Bytes::from_static(b"hour=12/"));
        assert_eq!(seg.tree.l0.len(), 2);
        assert_eq!(seg.tree.compacted.len(), 2);
        // Watermark is reset, matching unsegmented union behavior.
        assert!(seg.tree.last_compacted_l0_sst_view_id.is_none());
        // SR ids are unique across the merged segment.
        let sr_ids: Vec<u32> = seg.tree.compacted.iter().map(|sr| sr.id).collect();
        assert_eq!(sr_ids.len(), 2);
        assert_ne!(sr_ids[0], sr_ids[1]);
    }

    /// Build a segmented manifest whose segments each hold a single L0 view
    /// over the given range. `segments` is `(prefix, first_entry, range)` and
    /// must be ordered by prefix, as `ManifestCore::segments` is.
    fn segmented_shard_manifest(
        extractor_name: &str,
        segments: Vec<(&'static [u8], &'static str, BytesRange)>,
    ) -> Manifest {
        let mut core = ManifestCore::new();
        core.segment_extractor_name = Some(extractor_name.to_string());
        core.segments = segments
            .into_iter()
            .map(|(prefix, first_entry, range)| {
                let sst = SsTableId::from(Ulid::new());
                Segment {
                    prefix: Bytes::from_static(prefix),
                    tree: Arc::new(LsmTreeState {
                        last_compacted_l0_sst_view_id: None,
                        last_compacted_l0_sst_id: None,
                        l0: VecDeque::from(vec![SsTableView::new_projected(
                            sst.value(),
                            SsTableHandle::new(
                                sst,
                                SST_FORMAT_VERSION_LATEST,
                                SsTableInfo {
                                    first_entry: Some(Bytes::from_static(first_entry.as_bytes())),
                                    ..SsTableInfo::default()
                                },
                            ),
                            Some(range),
                        )]),
                        compacted: vec![],
                    }),
                }
            })
            .collect();
        Manifest::initial(core)
    }

    fn union_of(manifests: Vec<Manifest>) -> Result<Manifest, SlateDBError> {
        Manifest::cloned_from_union(
            manifests
                .into_iter()
                .enumerate()
                .map(|(i, manifest)| CloneSource {
                    manifest,
                    path: Path::from(format!("/tmp/db{}", i)),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                })
                .collect(),
            Arc::new(DbRand::default()),
        )
    }

    /// Effective range of each L0 view in the named segment, in list order.
    fn segment_l0_ranges(manifest: &Manifest, prefix: &[u8]) -> Vec<BytesRange> {
        manifest
            .core
            .segments
            .iter()
            .find(|s| s.prefix == prefix)
            .expect("segment present")
            .tree
            .l0
            .iter()
            .map(|view| view.compacted_effective_range().clone())
            .collect()
    }

    #[test]
    fn test_union_validates_key_ranges_per_segment() {
        // Two tenant shards of a store keyed `data/{tenant}` and
        // `idx/{tenant}`. Each shard holds part of both segments, so the
        // shards' bounding ranges overlap — `data/metro` sorts below
        // `idx/bronx` — while neither segment does. A read routes to exactly
        // one segment, so the union is unambiguous and must be accepted.
        //
        // The shards also lead in different segments: `left` holds the lower
        // `data` keys, `right` the lower `idx` keys. Each segment's list must
        // follow its own key order rather than the manifest-wide source
        // order, so a source's entries stay contiguous and ascending within
        // the segment, as they do in the unsegmented case.
        fn shard(data: BytesRange, idx: BytesRange) -> Manifest {
            segmented_shard_manifest("kind", vec![(b"data", "data", data), (b"idx", "idx", idx)])
        }
        let left = shard(
            BytesRange::from_ref("data/bronx".."data/metro"),
            BytesRange::from_ref("idx/metro".."idx/zzz"),
        );
        let right = shard(
            BytesRange::from_ref("data/metro".."data/zzz"),
            BytesRange::from_ref("idx/bronx".."idx/metro"),
        );

        // The precondition the old manifest-wide check enforced is genuinely
        // violated here; only the per-segment check makes this union legal.
        let left_range = left.range().expect("left range");
        assert!(left_range
            .intersect(&right.range().expect("right range"))
            .is_some());

        let union = union_of(vec![left, right]).expect("union of disjoint segments");

        assert_eq!(union.core.segment_extractor_name.as_deref(), Some("kind"));
        assert_eq!(
            segment_l0_ranges(&union, b"data"),
            vec![
                BytesRange::from_ref("data/bronx".."data/metro"),
                BytesRange::from_ref("data/metro".."data/zzz"),
            ]
        );
        assert_eq!(
            segment_l0_ranges(&union, b"idx"),
            vec![
                BytesRange::from_ref("idx/bronx".."idx/metro"),
                BytesRange::from_ref("idx/metro".."idx/zzz"),
            ]
        );

        // Overlap *within* a shared segment is still rejected: the merged
        // segment's read chain could not say which source owns the key.
        let overlapping = shard(
            BytesRange::from_ref("data/lincoln".."data/zzz"),
            BytesRange::from_ref("idx/bronx".."idx/metro"),
        );
        let err = union_of(vec![
            shard(
                BytesRange::from_ref("data/bronx".."data/metro"),
                BytesRange::from_ref("idx/metro".."idx/zzz"),
            ),
            overlapping,
        ])
        .expect_err("overlap within `data`");
        let SlateDBError::InvalidUnion(msg) = err else {
            panic!("expected InvalidUnion, got {:?}", err);
        };
        // The message names the offending segment, not just the ranges.
        assert!(msg.contains("in segment"), "{}", msg);
        assert!(msg.contains("data"), "{}", msg);
    }

    #[test]
    fn test_union_unsegmented_sources_land_in_core_tree() {
        // Two sources with no extractor configured. The unioned manifest
        // must keep `segment_extractor_name = None`, place the merged
        // data in `core.tree`, and leave `core.segments` empty — not
        // route the data into `core.segments[""]`.
        let m1 = manifest_with_one_compacted_sst(
            SsTableId::from(Ulid::new()),
            b"a",
            BytesRange::from_ref("a".."m"),
        );
        let m2 = manifest_with_one_compacted_sst(
            SsTableId::from(Ulid::new()),
            b"m",
            BytesRange::from_ref("m"..),
        );

        let union = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: m1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: m2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            Arc::new(DbRand::default()),
        )
        .unwrap();

        assert!(union.core.segment_extractor_name.is_none());
        assert!(union.core.segments.is_empty());
        assert_eq!(union.core.tree.compacted.len(), 2);
    }

    #[test]
    fn test_union_routes_explicit_empty_prefix_segment_to_segments() {
        // A configured extractor that maps some keys to `""` produces a
        // segment with prefix `""` in `core.segments`. Under union, that
        // data must stay in `core.segments[""]`, NOT get extracted back
        // into `core.tree` — `core.tree` is only used when no extractor
        // is configured.
        let (m1, _, _) =
            manifest_with_segment(b"", Some("legacy"), b"a", BytesRange::from_ref("a".."m"));
        let (m2, _, _) =
            manifest_with_segment(b"", Some("legacy"), b"m", BytesRange::from_ref("m"..));

        let union = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: m1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: m2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            Arc::new(DbRand::default()),
        )
        .unwrap();

        assert_eq!(union.core.segment_extractor_name.as_deref(), Some("legacy"));
        // Data stays in segments under the empty prefix; core.tree is
        // untouched because the unioned manifest has an extractor.
        assert!(union.core.tree.l0.is_empty());
        assert!(union.core.tree.compacted.is_empty());
        assert_eq!(union.core.segments.len(), 1);
        let seg = &union.core.segments[0];
        assert_eq!(seg.prefix, Bytes::new());
        assert_eq!(seg.tree.l0.len(), 2);
        assert_eq!(seg.tree.compacted.len(), 2);
    }

    #[test]
    fn test_union_returns_error_on_non_antichain_prefixes() {
        // Two sources whose extractor names match but whose persisted
        // segment prefixes are in a proper-prefix relationship. Even
        // though the SST-view ranges happen to be non-overlapping, the
        // antichain invariant rejects the union.
        let m1 = {
            let mut core = ManifestCore::new();
            core.segment_extractor_name = Some("test".into());
            core.segments = vec![Segment {
                prefix: Bytes::from_static(b"foo/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::from(vec![SsTableView::new_projected(
                        Ulid::new(),
                        SsTableHandle::new(
                            SsTableId::from(Ulid::new()),
                            SST_FORMAT_VERSION_LATEST,
                            SsTableInfo {
                                first_entry: Some(Bytes::from_static(b"foo/a")),
                                ..SsTableInfo::default()
                            },
                        ),
                        Some(BytesRange::from_ref("foo/a".."foo/h")),
                    )]),
                    compacted: vec![],
                }),
            }];
            Manifest::initial(core)
        };
        let m2 = {
            let mut core = ManifestCore::new();
            core.segment_extractor_name = Some("test".into());
            core.segments = vec![Segment {
                prefix: Bytes::from_static(b"foo/bar/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::from(vec![SsTableView::new_projected(
                        Ulid::new(),
                        SsTableHandle::new(
                            SsTableId::from(Ulid::new()),
                            SST_FORMAT_VERSION_LATEST,
                            SsTableInfo {
                                first_entry: Some(Bytes::from_static(b"q")),
                                ..SsTableInfo::default()
                            },
                        ),
                        Some(BytesRange::from_ref("q".."z")),
                    )]),
                    compacted: vec![],
                }),
            }];
            Manifest::initial(core)
        };

        let result = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: m1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: m2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            Arc::new(DbRand::default()),
        );
        assert!(matches!(result, Err(SlateDBError::InvalidUnion(_))));
    }

    #[test]
    fn test_union_returns_error_on_extractor_mismatch() {
        let (m1, _, _) = manifest_with_segment(
            b"hour=11/",
            Some("hour"),
            b"a",
            BytesRange::from_ref("a".."m"),
        );
        let (m2, _, _) =
            manifest_with_segment(b"day=1/", Some("day"), b"m", BytesRange::from_ref("m"..));

        let result = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: m1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: m2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            Arc::new(DbRand::default()),
        );
        assert!(matches!(result, Err(SlateDBError::InvalidUnion(_))));
    }

    #[test]
    fn test_union_returns_error_on_mixed_extractor_presence() {
        // One source has an extractor configured; the other doesn't.
        // Even though SST-view ranges are disjoint, unsegmented data in
        // the no-extractor source may match an extractor prefix from the
        // other source, so the union is rejected (RFC-0024).
        let (m_with, _, _) = manifest_with_segment(
            b"hour=11/",
            Some("hour"),
            b"a",
            BytesRange::from_ref("a".."m"),
        );
        let m_without = manifest_with_one_compacted_sst(
            SsTableId::from(Ulid::new()),
            b"m",
            BytesRange::from_ref("m"..),
        );

        let result = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: m_with,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: m_without,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            Arc::new(DbRand::default()),
        );
        assert!(matches!(result, Err(SlateDBError::InvalidUnion(_))));
    }

    #[test]
    fn test_union_returns_error_on_overlapping_ranges() {
        // Two sources whose effective key ranges intersect must be
        // rejected since the unioned manifest cannot disambiguate which
        // source owns the overlap.
        let m1 = manifest_with_one_compacted_sst(
            SsTableId::from(Ulid::new()),
            b"a",
            BytesRange::from_ref("a".."m"),
        );
        let m2 = manifest_with_one_compacted_sst(
            SsTableId::from(Ulid::new()),
            b"f",
            BytesRange::from_ref("f".."z"),
        );

        let result = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: m1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: m2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            Arc::new(DbRand::default()),
        );
        assert!(matches!(result, Err(SlateDBError::InvalidUnion(_))));
    }

    #[test]
    fn test_union_returns_error_on_segments_without_extractor() {
        // Degenerate source manifests: no extractor configured but
        // `core.segments` is non-empty. The union must reject this
        // rather than silently dropping the segment data. Use two
        // non-overlapping prefixes so the antichain check passes and
        // we reach the `is_none()` branch.
        let (m1, _, _) = manifest_with_segment(b"foo/", None, b"a", BytesRange::from_ref("a".."m"));
        let (m2, _, _) = manifest_with_segment(b"bar/", None, b"m", BytesRange::from_ref("m"..));

        let result = Manifest::cloned_from_union(
            vec![
                CloneSource {
                    manifest: m1,
                    path: Path::from("/tmp/db1"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
                CloneSource {
                    manifest: m2,
                    path: Path::from("/tmp/db2"),
                    checkpoint: new_checkpoint(Uuid::new_v4()),
                },
            ],
            Arc::new(DbRand::default()),
        );
        assert!(matches!(result, Err(SlateDBError::InvalidUnion(_))));
    }

    #[test]
    fn test_range_includes_segment_ssts() {
        // A manifest whose only key data lives in a segment must still
        // produce a non-None range — otherwise `cloned_from_union` would
        // silently skip the source.
        let (m, _, _) =
            manifest_with_segment(b"hour=12/", Some("hour"), b"a", BytesRange::from_ref("a"..));
        assert!(m.range().is_some());
    }

    #[test]
    fn test_cloned_includes_segment_ssts() {
        // Single-source clone: parent_owned SSTs must include segment-
        // resident views.
        use std::collections::HashSet;

        let (parent, l0_sst, sr_sst) =
            manifest_with_segment(b"hour=12/", Some("hour"), b"a", BytesRange::from_ref("a"..));
        let clone = Manifest::cloned(
            &parent,
            "tmp/parent".into(),
            Uuid::new_v4(),
            Arc::new(DbRand::default()),
        );

        let parent_external: HashSet<SsTableId> = clone
            .external_dbs
            .iter()
            .filter(|db| db.path == "tmp/parent")
            .flat_map(|db| db.sst_ids.iter().copied())
            .collect();
        assert!(parent_external.contains(&l0_sst));
        assert!(parent_external.contains(&sr_sst));
    }

    #[test]
    fn test_projected_drops_segment_when_all_views_filtered_out() {
        // Projection range fully disjoint from the segment's data: the
        // segment drops out, but `segment_extractor_name` is preserved.
        let (manifest, _, _) = manifest_with_segment(
            b"hour=12/",
            Some("hour"),
            b"a",
            BytesRange::from_ref("a".."m"),
        );

        let projected = Manifest::projected(
            &manifest,
            &ProjectionConfig::from_global_range(BytesRange::from_ref("z"..)),
        )
        .unwrap();
        assert!(projected.core.segments.is_empty());
        assert_eq!(
            projected.core.segment_extractor_name.as_deref(),
            Some("hour")
        );
    }

    #[test]
    fn test_projected_keeps_in_range_segment_views() {
        // Two segments — one whose views overlap the projection range and
        // one whose views are fully disjoint. Only the overlapping
        // segment survives.
        let l0_a = SsTableId::from(Ulid::new());
        let sr_a = SsTableId::from(Ulid::new());
        let sr_b = SsTableId::from(Ulid::new());
        let mut core = ManifestCore::new();
        core.segment_extractor_name = Some("hour".into());
        core.segments = vec![
            Segment {
                prefix: Bytes::from_static(b"hour=11/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::from(vec![SsTableView::new_projected(
                        l0_a.value(),
                        SsTableHandle::new(
                            l0_a,
                            SST_FORMAT_VERSION_LATEST,
                            SsTableInfo {
                                first_entry: Some(Bytes::from_static(b"a")),
                                ..SsTableInfo::default()
                            },
                        ),
                        Some(BytesRange::from_ref("a".."d")),
                    )]),
                    compacted: vec![SortedRun::new(
                        0,
                        [SsTableView::new_projected(
                            sr_a.value(),
                            SsTableHandle::new(
                                sr_a,
                                SST_FORMAT_VERSION_LATEST,
                                SsTableInfo {
                                    first_entry: Some(Bytes::from_static(b"a")),
                                    ..SsTableInfo::default()
                                },
                            ),
                            Some(BytesRange::from_ref("a".."m")),
                        )],
                    )],
                }),
            },
            Segment {
                prefix: Bytes::from_static(b"hour=12/"),
                tree: Arc::new(LsmTreeState {
                    last_compacted_l0_sst_view_id: None,
                    last_compacted_l0_sst_id: None,
                    l0: VecDeque::new(),
                    compacted: vec![SortedRun::new(
                        1,
                        [SsTableView::new_projected(
                            sr_b.value(),
                            SsTableHandle::new(
                                sr_b,
                                SST_FORMAT_VERSION_LATEST,
                                SsTableInfo {
                                    first_entry: Some(Bytes::from_static(b"n")),
                                    ..SsTableInfo::default()
                                },
                            ),
                            Some(BytesRange::from_ref("n".."z")),
                        )],
                    )],
                }),
            },
        ];
        let manifest = Manifest::initial(core);

        let projected = Manifest::projected(
            &manifest,
            &ProjectionConfig::from_global_range(BytesRange::from_ref("a".."m")),
        )
        .unwrap();

        assert_eq!(projected.core.segments.len(), 1);
        assert_eq!(
            projected.core.segments[0].prefix,
            Bytes::from_static(b"hour=11/")
        );
        assert_eq!(projected.core.segments[0].tree.l0.len(), 1);
        assert_eq!(projected.core.segments[0].tree.compacted.len(), 1);
    }

    /// Build a single-sorted-run tree of two compacted SSTs with a key gap
    /// between them: SST1 physically covers `[a, c]`, SST2 covers `[m, p]`,
    /// so the keyspace `(c, m)` is owned by SST1 logically (up to SST2's start
    /// key) but holds no physical keys in SST1.
    fn manifest_with_two_sst_sorted_run_gap() -> Manifest {
        let sst1 = SsTableId::from(Ulid::new());
        let sst2 = SsTableId::from(Ulid::new());
        let make = |id: SsTableId, first: &'static [u8], last: &'static [u8], vis: BytesRange| {
            SsTableView::new_projected(
                id.value(),
                SsTableHandle::new(
                    id,
                    SST_FORMAT_VERSION_LATEST,
                    SsTableInfo {
                        first_entry: Some(Bytes::from_static(first)),
                        last_entry: Some(Bytes::from_static(last)),
                        ..SsTableInfo::default()
                    },
                ),
                Some(vis),
            )
        };
        let mut core = ManifestCore::new();
        Arc::make_mut(&mut core.tree).compacted.push(SortedRun::new(
            0,
            [
                make(sst1, b"a", b"c", BytesRange::from_ref("a".."d")),
                make(sst2, b"m", b"p", BytesRange::from_ref("m".."q")),
            ],
        ));
        Manifest::initial(core)
    }

    #[test]
    fn test_projected_drops_sorted_run_sst_when_range_falls_in_key_gap() {
        // Regression: a projection range that lands entirely in the gap between a sorted-run SST's
        // last physical key and the next SST's start key must drop the SST, not panic.
        // `compacted_intersection` bounds the SST by the next SST's start key (logical), so it
        // reports overlap for `(c, m)`, but the SST has no physical keys there.
        let manifest = manifest_with_two_sst_sorted_run_gap();

        // ["e", "g") sits in the gap (c, m): both SSTs are physically disjoint
        // from it, so the sorted run becomes empty and is dropped.
        let projected = Manifest::projected(
            &manifest,
            &ProjectionConfig::from_global_range(BytesRange::from_ref("e".."g")),
        )
        .unwrap();
        assert!(projected.core.tree.compacted.is_empty());
    }

    #[test]
    fn test_projected_keeps_sorted_run_sst_overlapping_physical_keys() {
        // Sanity check that the gap fix does not over-drop: a range that
        // overlaps SST1's physical keys keeps it (with the SST projected).
        let manifest = manifest_with_two_sst_sorted_run_gap();

        let projected = Manifest::projected(
            &manifest,
            &ProjectionConfig::from_global_range(BytesRange::from_ref("b".."f")),
        )
        .unwrap();
        assert_eq!(projected.core.tree.compacted.len(), 1);
        assert_eq!(projected.core.tree.compacted[0].sst_views().len(), 1);
    }

    #[test]
    fn test_merge_keeps_newer_views_of_a_partly_compacted_sst() {
        // given: a writer L0 holding two cut views of one SST, newest first,
        // and a compactor that compacted only the older view
        let id = SsTableId::from(Ulid::new());
        let handle = SsTableHandle::new(
            id,
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo {
                first_entry: Some(Bytes::from_static(b"a")),
                last_entry: Some(Bytes::from_static(b"z")),
                ..SsTableInfo::default()
            },
        );
        let newer = SsTableView::new_projected(
            id.value(),
            handle.clone(),
            Some(BytesRange::from_ref("b".."d")),
        );
        let older = SsTableView::new_projected(
            Ulid::new(),
            handle,
            Some(BytesRange::from_ref("m".."p")),
        );
        let writer = LsmTreeState {
            l0: VecDeque::from([newer.clone(), older.clone()]),
            ..LsmTreeState::default()
        };
        let compactor = LsmTreeState {
            last_compacted_l0_sst_view_id: Some(older.id),
            last_compacted_l0_sst_id: Some(id.value()),
            ..LsmTreeState::default()
        };

        // when
        let merged = LsmTreeState::merge_writer_and_compactor(&writer, &compactor);

        // then: the newer view of the same SST is still in L0
        assert_eq!(merged.l0.len(), 1);
        assert_eq!(merged.l0[0].id, newer.id);
    }

    #[test]
    fn test_projected_preserves_empty_prefix_segment_under_extractor() {
        // A source with `Some(extractor)` and a `""` segment must keep
        // that segment in `core.segments` after projection — projection
        // only filters; it never moves data into `core.tree` (which is
        // only populated when no extractor is configured).
        let (manifest, _, _) =
            manifest_with_segment(b"", Some("legacy"), b"a", BytesRange::from_ref("a".."z"));

        let projected = Manifest::projected(
            &manifest,
            &ProjectionConfig::from_global_range(BytesRange::from_ref("a".."m")),
        )
        .unwrap();
        assert_eq!(
            projected.core.segment_extractor_name.as_deref(),
            Some("legacy")
        );
        assert!(projected.core.tree.l0.is_empty());
        assert!(projected.core.tree.compacted.is_empty());
        assert_eq!(projected.core.segments.len(), 1);
        assert_eq!(projected.core.segments[0].prefix, Bytes::new());
    }

    #[test]
    fn test_cloned_preserves_empty_prefix_segment_under_extractor() {
        // Single-source clone of a parent with `Some(extractor)` + `""`
        // segment: the clone's core inherits the same shape. `core.tree`
        // stays empty; `core.segments` carries the `""` entry.
        let (parent, _, _) =
            manifest_with_segment(b"", Some("legacy"), b"a", BytesRange::from_ref("a"..));

        let clone = Manifest::cloned(
            &parent,
            "tmp/parent".into(),
            Uuid::new_v4(),
            Arc::new(DbRand::default()),
        );

        assert_eq!(clone.core.segment_extractor_name.as_deref(), Some("legacy"));
        assert!(clone.core.tree.l0.is_empty());
        assert!(clone.core.tree.compacted.is_empty());
        assert_eq!(clone.core.segments.len(), 1);
        assert_eq!(clone.core.segments[0].prefix, Bytes::new());
    }

    #[test]
    fn test_projected_external_db_pruning_considers_segment_ssts() {
        // An external_db whose SSTs are referenced only via a segment must
        // be retained after projection (otherwise the clone loses external
        // SSTs that segments still need).
        let (mut manifest, l0_sst, _) = manifest_with_segment(
            b"hour=12/",
            Some("hour"),
            b"a",
            BytesRange::from_ref("a".."m"),
        );
        manifest.external_dbs.push(ExternalDb {
            path: "tmp/parent".into(),
            source_checkpoint_id: Uuid::new_v4(),
            final_checkpoint_id: Some(Uuid::new_v4()),
            sst_ids: vec![l0_sst],
        });

        let projected = Manifest::projected(
            &manifest,
            &ProjectionConfig::from_global_range(BytesRange::from_ref("a".."m")),
        )
        .unwrap();
        assert!(
            projected
                .external_dbs
                .iter()
                .any(|db| db.sst_ids.contains(&l0_sst)),
            "external_db with segment-resident SST must be retained after projection"
        );
    }

    /// Build a multi-segment manifest where each segment owns a single L0 view
    /// spanning the full `[prefix, prefix++)` range.
    fn manifest_with_full_segments(extractor: &str, prefixes: &[&[u8]]) -> Manifest {
        let mut core = ManifestCore::new();
        core.segment_extractor_name = Some(extractor.to_string());
        core.segments = prefixes
            .iter()
            .map(|p| {
                let sst_id = SsTableId::from(Ulid::new());
                let full = BytesRange::from_prefix(p);
                Segment {
                    prefix: Bytes::copy_from_slice(p),
                    tree: Arc::new(LsmTreeState {
                        last_compacted_l0_sst_view_id: None,
                        last_compacted_l0_sst_id: None,
                        l0: VecDeque::from(vec![SsTableView::new_projected(
                            sst_id.value(),
                            SsTableHandle::new(
                                sst_id,
                                SST_FORMAT_VERSION_LATEST,
                                SsTableInfo {
                                    first_entry: Some(Bytes::copy_from_slice(p)),
                                    ..SsTableInfo::default()
                                },
                            ),
                            Some(full),
                        )]),
                        compacted: vec![],
                    }),
                }
            })
            .collect();
        Manifest::initial(core)
    }

    #[test]
    fn test_projected_segment_projection_applies_per_segment() {
        // Two segments. Closure returns a narrowing range for the first and
        // the full segment for the second. Each segment's L0 view should be
        // narrowed accordingly.
        let manifest = manifest_with_full_segments("hour", &[b"hour=11/", b"hour=12/"]);

        let projector: SegmentProjectionFn = Arc::new(|prefix: &[u8]| {
            Ok(if prefix == b"hour=11/" {
                BytesRange::from_ref("hour=11/a".."hour=11/m")
            } else {
                BytesRange::from_prefix(prefix)
            })
        });
        let config = ProjectionConfig {
            global_range: None,
            segment_filter: None,
            segment_projection: Some(projector),
        };

        let projected = Manifest::projected(&manifest, &config).unwrap();
        assert_eq!(projected.core.segments.len(), 2);
        let s11 = &projected.core.segments[0];
        let s12 = &projected.core.segments[1];
        assert_eq!(s11.prefix.as_ref(), b"hour=11/");
        assert_eq!(s12.prefix.as_ref(), b"hour=12/");
        let s11_view = &s11.tree.l0[0];
        let s12_view = &s12.tree.l0[0];
        assert_eq!(
            s11_view.visible_range().unwrap().start_bound(),
            Bound::Included(&Bytes::from("hour=11/a"))
        );
        assert_eq!(
            s11_view.visible_range().unwrap().end_bound(),
            Bound::Excluded(&Bytes::from("hour=11/m"))
        );
        // s12 keeps the whole segment range.
        assert_eq!(
            s12_view.visible_range().unwrap().start_bound(),
            Bound::Included(&Bytes::from("hour=12/"))
        );
    }

    #[test]
    fn test_projected_segment_filter_drops_named_segment() {
        let manifest = manifest_with_full_segments("hour", &[b"hour=11/", b"hour=12/"]);
        let filter: SegmentFilterFn = Arc::new(|p: &[u8]| p != b"hour=12/");
        let config = ProjectionConfig {
            global_range: None,
            segment_filter: Some(filter),
            segment_projection: None,
        };
        let projected = Manifest::projected(&manifest, &config).unwrap();
        assert_eq!(projected.core.segments.len(), 1);
        assert_eq!(projected.core.segments[0].prefix.as_ref(), b"hour=11/");
    }

    #[test]
    fn test_projected_segment_filter_drops_unsegmented_tree() {
        // Unsegmented DB with data in core.tree. Filter rejects the empty
        // prefix, so core.tree is cleared.
        let (manifest, _, _) =
            manifest_with_segment(b"", None, b"a", BytesRange::from_ref("a".."z"));
        let filter: SegmentFilterFn = Arc::new(|p: &[u8]| !p.is_empty());
        let config = ProjectionConfig {
            global_range: None,
            segment_filter: Some(filter),
            segment_projection: None,
        };
        let projected = Manifest::projected(&manifest, &config).unwrap();
        assert!(projected.core.tree.l0.is_empty());
        assert!(projected.core.tree.compacted.is_empty());
    }

    #[test]
    fn test_projected_intersects_segment_projection_with_global_range() {
        // Segment projection asks for [hour=11/a, hour=11/z), but the global
        // range restricts to [hour=11/c, hour=11/m). The effective per-SST
        // visible_range should be the intersection.
        let manifest = manifest_with_full_segments("hour", &[b"hour=11/"]);
        let projector: SegmentProjectionFn =
            Arc::new(|_prefix: &[u8]| Ok(BytesRange::from_ref("hour=11/a".."hour=11/z")));
        let config = ProjectionConfig {
            global_range: Some(BytesRange::from_ref("hour=11/c".."hour=11/m")),
            segment_filter: None,
            segment_projection: Some(projector),
        };
        let projected = Manifest::projected(&manifest, &config).unwrap();
        let view = &projected.core.segments[0].tree.l0[0];
        assert_eq!(
            view.visible_range().unwrap().start_bound(),
            Bound::Included(&Bytes::from("hour=11/c"))
        );
        assert_eq!(
            view.visible_range().unwrap().end_bound(),
            Bound::Excluded(&Bytes::from("hour=11/m"))
        );
    }

    #[test]
    fn test_projected_drops_segment_when_projection_and_global_range_disjoint() {
        // segment_projection narrows hour=11/ to a sub-range, but the global
        // range only covers hour=12/. The hour=11/ intersection is empty, so
        // it must be dropped. hour=12/ uses its full segment range and
        // intersects with the global range to a non-empty result.
        let manifest = manifest_with_full_segments("hour", &[b"hour=11/", b"hour=12/"]);
        let projector: SegmentProjectionFn = Arc::new(|prefix: &[u8]| {
            Ok(if prefix == b"hour=11/" {
                BytesRange::from_ref("hour=11/a".."hour=11/m")
            } else {
                BytesRange::from_prefix(prefix)
            })
        });
        let config = ProjectionConfig {
            global_range: Some(BytesRange::from_ref("hour=12/a".."hour=12/z")),
            segment_filter: None,
            segment_projection: Some(projector),
        };

        let projected = Manifest::projected(&manifest, &config).unwrap();
        assert_eq!(projected.core.segments.len(), 1);
        assert_eq!(projected.core.segments[0].prefix.as_ref(), b"hour=12/");
    }

    #[test]
    fn test_projected_errors_when_segment_range_outside_prefix() {
        let manifest = manifest_with_full_segments("hour", &[b"hour=11/"]);
        let projector: SegmentProjectionFn =
            Arc::new(|_prefix: &[u8]| Ok(BytesRange::from_ref("a".."m")));
        let config = ProjectionConfig {
            global_range: None,
            segment_filter: None,
            segment_projection: Some(projector),
        };
        let err = Manifest::projected(&manifest, &config).unwrap_err();
        match err {
            SlateDBError::InvalidProjection { prefix, .. } => {
                assert_eq!(prefix.as_ref(), b"hour=11/");
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    fn manifest_with_segments(prefixes: &[&[u8]]) -> VersionedManifest {
        let mut core = ManifestCore::new();
        if !prefixes.is_empty() {
            core.segment_extractor_name = Some("hour-bucket".to_string());
            core.segments = prefixes
                .iter()
                .map(|p| Segment {
                    prefix: Bytes::copy_from_slice(p),
                    tree: Arc::new(LsmTreeState::default()),
                })
                .collect();
        }
        VersionedManifest::from_manifest(1, Manifest::initial(core))
    }

    #[test]
    fn test_versioned_manifest_segments_empty_when_unconfigured() {
        let manifest = manifest_with_segments(&[]);
        assert!(manifest.segments().is_empty());
        assert!(manifest.segment(b"anything").is_none());
    }

    #[test]
    fn test_versioned_manifest_segments_returns_in_prefix_order() {
        let manifest = manifest_with_segments(&[b"ts/2026-03-09/14/", b"ts/2026-03-09/15/"]);
        let segments = manifest.segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].prefix().as_ref(), b"ts/2026-03-09/14/");
        assert_eq!(segments[1].prefix().as_ref(), b"ts/2026-03-09/15/");
    }

    #[test]
    fn test_versioned_manifest_segment_lookup_requires_exact_match() {
        let manifest = manifest_with_segments(&[b"ts/2026-03-09/14/"]);
        // Exact match resolves.
        assert!(manifest.segment(b"ts/2026-03-09/14/").is_some());
        // Strict prefix of the segment must not match.
        assert!(manifest.segment(b"ts/2026-").is_none());
        // Key extending the segment's prefix must not match either.
        assert!(manifest.segment(b"ts/2026-03-09/14/series-7").is_none());
        // Empty prefix is not a wildcard.
        assert!(manifest.segment(b"").is_none());
    }
}
