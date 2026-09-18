//! All depot Prometheus metrics live in this single flat module.
//!
//! Metrics are split into two conceptual layers:
//!
//! - **Live layer** (`sqlite_conveyer_*`, `sqlite_shard_cache_*`, `sqlite_branch_*`,
//!   `sqlite_restore_point_*`): the hot path served per pegboard node. These carry a `node_id` label
//!   where a node is available.
//! - **Background layer** (`sqlite_compaction_*`): hot install, cold publish, FDB reclaim, and
//!   cold-object retirement run on workflow workers. These carry no `node_id` label and deliberately
//!   omit per-branch/per-database labels to keep cardinality bounded.

use std::{future::Future, time::Instant};

use anyhow::Result;
use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

use crate::compaction::types::{
	CompactionJobStatus, InstallHotJobOutput, ReclaimFdbJobOutput, ReclaimRowStats,
	ReclaimRowVolume, StageHotSliceOutput, SweepCommitDeltaChunkOutput,
	SweepDeadShardVersionsOutput,
};

const SLOW_PHASE_WARN_THRESHOLD_SECONDS: f64 = 1.0;

// Shard/ref counts folded per compaction pass are small; keep a compact bucket set covering a
// single-slice up to a large multi-slice drain.
const SHARD_COUNT_BUCKETS: &[f64] = &[1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 512.0];

// Cold shard objects are single folded shard images, spanning a few pages up to a large shard.
const COLD_OBJECT_BYTES_BUCKETS: &[f64] = &[
	4096.0,
	32768.0,
	131072.0,
	524288.0,
	2097152.0,
	8388608.0,
	33554432.0,
	134217728.0,
];

// Compaction lag is measured in txids and can span from a single pending commit to a very deep
// backlog, so use a wide exponential-ish spread.
const LAG_TXID_BUCKETS: &[f64] = &[
	1.0, 8.0, 64.0, 256.0, 1024.0, 8192.0, 65536.0, 524288.0, 4194304.0,
];

// How far one reclaim chunk moves its commit scan cursor. A chunk that keeps landing in the low
// buckets against a deep backlog is walking retained history, which is the shape that strands
// reclaimable rows behind it.
const RECLAIM_CURSOR_ADVANCE_BUCKETS: &[f64] =
	&[0.0, 1.0, 8.0, 64.0, 512.0, 4096.0, 32768.0, 262144.0];

// Hot shard bytes moved by a single staging or install activity call. Bounded above by
// `CMP_STAGE_MAX_WRITE_BYTES` / `CMP_INSTALL_MAX_WRITE_BYTES` per transaction, but a call runs
// several transactions before its early timeout, so the top bucket sits well above the per-txn cap.
const HOT_SHARD_BYTES_BUCKETS: &[f64] = &[
	4096.0,
	65536.0,
	524288.0,
	2097152.0,
	8388608.0,
	33554432.0,
	134217728.0,
	536870912.0,
];

// Live-layer shard cache read outcomes.
pub const SHARD_CACHE_READ_FDB_HIT: &str = "fdb_hit";
pub const SHARD_CACHE_READ_COLD_HIT: &str = "cold_hit";
pub const SHARD_CACHE_READ_MISS: &str = "miss";

pub const SHARD_CACHE_FILL_SCHEDULED: &str = "scheduled";
pub const SHARD_CACHE_FILL_SUCCEEDED: &str = "succeeded";
pub const SHARD_CACHE_FILL_FAILED: &str = "failed";
pub const SHARD_CACHE_FILL_SKIPPED_QUEUE_FULL: &str = "skipped_queue_full";
pub const SHARD_CACHE_FILL_SKIPPED_DUPLICATE: &str = "skipped_duplicate";
pub const SHARD_CACHE_FILL_SKIPPED_NO_COLD_REF: &str = "skipped_no_cold_ref";

pub const SHARD_CACHE_EVICTION_CLEARED: &str = "cleared";

// Background-layer compaction pass kinds and result labels.
/// The companion drain's staging pass. Separate from `hot_install` because the two halves of hot
/// compaction write the same shard images to different subspaces and compete for the same
/// cluster-wide write budget, so a single `hot` pass rate cannot say which half is starving the
/// other.
pub const PASS_HOT_STAGE: &str = "hot_stage";
pub const PASS_HOT_INSTALL: &str = "hot_install";
pub const PASS_RECLAIM_FDB: &str = "reclaim_fdb";
/// The v2 commit/delta sweep. A separate pass from `reclaim_fdb` because the v2 drain runs both per
/// iteration, and they are differently shaped: one derives and clears a history window, the other
/// executes a planned cold/shard-cache slice. Sharing a label would double the pass rate for v2
/// branches and mix two distributions in one duration histogram.
pub const PASS_RECLAIM_SWEEP: &str = "reclaim_sweep";
/// The dead-shard version-retention sweep. One pass per activity dispatch, so a long sweep that
/// re-dispatches on its early timeout reports several `incomplete` passes before a `succeeded` one.
/// A commit that fit one message and one transaction.
pub const COMMIT_PATH_SINGLE_SHOT: &str = "single_shot";

/// A commit streamed as shard-aligned segments and published by a separate finalize.
pub const COMMIT_PATH_STAGED: &str = "staged";

pub const PASS_RECLAIM_DEAD_SHARD: &str = "reclaim_dead_shard";

pub const RESULT_SUCCEEDED: &str = "succeeded";
pub const RESULT_REJECTED: &str = "rejected";
pub const RESULT_FAILED: &str = "failed";
pub const RESULT_ERROR: &str = "error";
/// The pass returned early because a cluster-wide compaction throttle budget was spent. It did the
/// work it was admitted for (often none) and hands its cursor back to be re-dispatched.
pub const RESULT_THROTTLED: &str = "throttled";
/// The pass committed part of its window and handed a cursor back so the workflow re-dispatches it,
/// usually because it crossed the bulk-activity early timeout.
pub const RESULT_INCOMPLETE: &str = "incomplete";
/// The pass found nothing left in its drain window. Distinct from `succeeded` because the drain
/// calls staging once more after its last real slice to learn that it is done, so folding the two
/// together makes the success rate track how often drains end rather than how much work lands.
pub const RESULT_DRAINED: &str = "drained";
/// The pass found a commit in its drain window that the slice budget cannot admit even when empty,
/// so no later slice can admit it either. Distinct from `drained`: the branch's hot lane cannot
/// advance past that txid, its deltas are never folded, and reclaim can never free them. A nonzero
/// rate here is a wedged branch, not idleness.
pub const RESULT_STALLED: &str = "stalled";
/// The branch fell outside the compaction admission percent, so the pass read and wrote nothing and
/// parked. Distinct from `throttled`: no budget was spent and no backoff applies.
pub const RESULT_ADMISSION_BLOCKED: &str = "admission_blocked";

// Compaction throttle kinds. Bounded low-cardinality labels, matching the compaction-metric
// rule (no node/branch/actor ids).
/// Hot staging, run by the companion drain. Split from `hot_install` because both charge the same
/// write axis and the merged `hot` kind could not attribute a spent budget to either.
/// Where a hot fold wrote its shard image. A fold either stages it for install to copy across, or
/// writes it straight into the live shard rows.
pub const FOLD_DESTINATION_STAGING: &str = "staging";
pub const FOLD_DESTINATION_SHARD: &str = "shard";

pub const COMPACTION_KIND_HOT_STAGE: &str = "hot_stage";
pub const COMPACTION_KIND_HOT_INSTALL: &str = "hot_install";
pub const COMPACTION_KIND_RECLAIM: &str = "reclaim";
pub const COMPACTION_KIND_COLD: &str = "cold";
/// The manager refresh snapshot. Charges the read axis but is never gated by it, so this kind
/// appears on the read-bytes counter and never on the throttled counter.
pub const COMPACTION_KIND_MANAGER: &str = "manager";

pub const LAG_TIER_HOT: &str = "hot";
pub const LAG_TIER_COLD: &str = "cold";

pub const COLD_DELETE_PHASE_RETIRED: &str = "retired";
pub const COLD_DELETE_PHASE_ORPHAN: &str = "orphan";

// Row kinds a reclaim pass scans and clears. One label value per FDB row family the reclaim window
// touches, so a branch that is reclaiming plenty of one kind while starving another is visible
// without a per-branch label.
pub const RECLAIM_ROW_KIND_COMMIT: &str = "commit";
pub const RECLAIM_ROW_KIND_VTX: &str = "vtx";
pub const RECLAIM_ROW_KIND_DELTA: &str = "delta";
pub const RECLAIM_ROW_KIND_PITR_INTERVAL: &str = "pitr_interval";
/// Live `SHARD` rows the reclaim window demoted to the cold tier. Bytes on this kind left the hot
/// tier but still exist as a cold object, so they are not the same thing as bytes deleted outright.
pub const RECLAIM_ROW_KIND_SHARD_EVICT: &str = "shard_evict";
/// Dead `SHARD` versions the version-retention sweep deleted outright. Nothing else in the system
/// clears these, so this kind is the sweep's whole output.
pub const RECLAIM_ROW_KIND_SHARD: &str = "shard";
pub const RECLAIM_ROW_KIND_STAGING: &str = "staging";

/// The reclaim scan walked to the end of the reclaimable `COMMITS` range.
pub const RECLAIM_SCAN_COMPLETE: &str = "complete";
/// The scan stopped on its batch budget with range left below it, so the cursor is mid-range.
pub const RECLAIM_SCAN_PARTIAL: &str = "partial";

lazy_static::lazy_static! {
	// ---- Cold-tier / takeover ----

	pub static ref SQLITE_S3_REQUEST_FAILURES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_s3_request_failures_total",
		"Total sqlite cold-tier request failures.",
		&["node_id", "op"],
		*REGISTRY
	).unwrap();

	// ---- Live layer: conveyer commit / get_pages ----

	pub static ref SQLITE_PUMP_COMMIT_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_commit_duration",
		"Duration of stateless sqlite conveyer commit operations. For a staged commit this spans the whole begin-to-finalize sequence, not just the finalize transaction.",
		&["node_id", "path"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_GET_PAGES_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_get_pages_duration",
		"Duration of stateless sqlite conveyer get_pages operations.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_GET_PAGES_PHASE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_get_pages_phase_duration",
		"Duration of stateless sqlite conveyer get_pages transaction-attempt phases.",
		&["node_id", "phase", "attempt_result"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMMIT_PHASE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_commit_phase_duration",
		"Duration of stateless sqlite conveyer commit transaction-attempt phases.",
		&["node_id", "phase", "attempt_result"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_COMMIT_DIRTY_PAGE_COUNT: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_commit_dirty_page_count",
		"Number of dirty pages written per stateless sqlite conveyer commit.",
		&["node_id", "path"],
		// Reaches `MAX_COMMIT_DIRTY_PAGES`. The old top bucket was 8192, so every commit a staged
		// commit exists to carry landed in `+Inf` and its size was unmeasurable.
		vec![
			1.0, 4.0, 16.0, 64.0, 256.0, 320.0, 1024.0, 4096.0, 8192.0, 16384.0, 32768.0,
		],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_COMMIT_PAYLOAD_BYTES: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_commit_payload_bytes",
		"Total dirty-page payload bytes written per stateless sqlite conveyer commit.",
		&["node_id", "path"],
		vec![
			256.0, 1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 1048576.0, 4194304.0, 16777216.0,
		],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_GET_PAGES_PGNO_COUNT: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_get_pages_pgno_count",
		"Number of pages requested per stateless sqlite conveyer get_pages call.",
		&["node_id"],
		vec![1.0, 4.0, 16.0, 64.0, 256.0, 1024.0],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_GET_PAGES_RETURNED_BYTES: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_get_pages_returned_bytes",
		"Total page bytes returned per stateless sqlite conveyer get_pages call.",
		&["node_id"],
		vec![
			256.0, 1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 1048576.0, 4194304.0, 16777216.0,
		],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMMIT_STAGE_SEGMENTS: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_commit_stage_segments",
		"Segments a staged commit carried, observed once at finalize. One segment spans at most COMMIT_SEGMENT_MAX_SHARDS shards, so this is the round-trip count the commit paid.",
		&["node_id"],
		vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMMIT_TRANSACTION_BYTES: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_conveyer_commit_transaction_bytes",
		"Size of the transaction that published a commit, measured the way the database's own 10 MB transaction limit is rather than by the keys and values the commit carried. This is the bound MAX_COMMIT_DIRTY_PAGES exists to stay under, so the headroom left at the cap is readable here instead of inferred. Buckets reach the limit itself.",
		&["node_id", "path"],
		vec![
			4096.0, 16384.0, 65536.0, 262144.0, 1048576.0, 2097152.0, 4194304.0, 6291456.0,
			8388608.0, 10485760.0,
		],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMMIT_STAGE_SEGMENT_BYTES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_conveyer_commit_stage_segment_bytes_total",
		"Total bytes accepted by staged commit segments. Counted as the segments land, so it grows for commits that are later abandoned as well as for those that finalize.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMMIT_REJECTED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_conveyer_commit_rejected_total",
		"Commits the engine refused before publishing, by reason. `too_large` means the commit exceeded MAX_COMMIT_DIRTY_PAGES and no amount of staging would have made it fit.",
		&["node_id", "reason"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_PIDX_COLD_SCAN_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_conveyer_pidx_cold_scan_total",
		"Total stateless sqlite conveyer get_pages calls that performed a cold PIDX scan.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_READ_STALE_HOT_SHARD_SUPERSEDED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_conveyer_stale_hot_shard_superseded_total",
		"Total shard reads where the hot tier held an older image than the newest cold ref, so the cold ref was served instead.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_READ_SHARD_PAGE_MISSING_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_conveyer_shard_page_missing_total",
		"Total page reads that resolved to a shard image which does not carry the page, so the page was served as zeros. A shard image must be complete, so any count here is a compaction defect.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_HOT_FOLD_COLD_MERGE_BASE_BYTES: IntCounter = register_int_counter_with_registry!(
		"sqlite_compaction_hot_fold_cold_merge_base_bytes",
		"Total bytes of shard images a hot fold pulled back from the cold tier because the hot tier no longer held a merge base. Nonzero means eviction is demoting images that folds still need, which is expected once a branch has been fully evicted and then written to again.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_READ_STALE_MAIN_PAGE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_conveyer_stale_main_page_total",
		"Total page reads whose page one reported a database size that disagrees with the commit at the txid the read resolved to. Any count here means a read served page one from a different point in history than its size.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	// ---- Live layer: read-through shard cache ----

	pub static ref SQLITE_SHARD_CACHE_FILL_SKIPPED_QUEUE_FULL_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_shard_cache_fill_skipped_queue_full_total",
		"Total sqlite read-through shard cache fills skipped because the queue was full.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_SHARD_CACHE_READ_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_shard_cache_read_total",
		"Total sqlite shard cache read outcomes.",
		&["outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_SHARD_CACHE_FILL_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_shard_cache_fill_total",
		"Total sqlite read-through shard cache fill outcomes.",
		&["outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_SHARD_CACHE_FILL_BYTES_TOTAL: IntCounter = register_int_counter_with_registry!(
		"sqlite_shard_cache_fill_bytes_total",
		"Total bytes written by sqlite read-through shard cache fills.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_SHARD_CACHE_EVICTION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_shard_cache_eviction_total",
		"Total sqlite shard cache eviction outcomes.",
		&["outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_SHARD_CACHE_RESIDENT_BYTES: IntGauge = register_int_gauge_with_registry!(
		"sqlite_shard_cache_resident_bytes",
		"Sampled sqlite shard cache resident bytes.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_SHARD_CACHE_COLD_READ_DURATION: Histogram = register_histogram_with_registry!(
		"sqlite_shard_cache_cold_read_duration",
		"Duration of sqlite cold shard-cache read-through object fetches.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	// ---- Live layer: branch / restore point ----

	pub static ref SQLITE_BRANCH_FORK_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_branch_fork_total",
		"Total sqlite branch fork operations.",
		&["node_id", "op", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_BRANCH_DELETE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_branch_delete_total",
		"Total sqlite branch delete operations.",
		&["node_id", "op", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_RESTORE_POINT_CREATE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_restore_point_create_total",
		"Total sqlite restore point create operations.",
		&["node_id", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_RESTORE_POINT_RESOLVE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_restore_point_resolve_total",
		"Total sqlite restore point resolve operations.",
		&["node_id", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_RESTORE_POINT_RESOLVE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_restore_point_resolve_duration",
		"Duration of sqlite restore point resolve operations.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_BRANCH_ANCESTRY_WALK_DEPTH: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_branch_ancestry_walk_depth",
		"Observed sqlite database branch ancestry walk depth.",
		&["node_id"],
		vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_BRANCH_PIN_ADVANCE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_branch_pin_advance_total",
		"Total sqlite branch pin advances.",
		&["node_id", "kind"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PIN_STATUS: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_pin_status",
		"Sampled sqlite restore point status count.",
		&["node_id", "status"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_DR_POSTURE: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_dr_posture",
		"Sampled sqlite disaster-recovery posture.",
		&["node_id", "recoverable_from"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_RESTORE_POINT_COUNT_PER_BUCKET: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_restore_point_count_per_bucket",
		"Sampled sqlite restore point count per bucket.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COLD_LAG_VERSIONSTAMPS: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_cold_lag_versionstamps",
		"Sampled sqlite cold lag in versionstamp units by database.",
		&["node_id", "database_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_RESTORE_POINT_RESOLUTION_CHAIN_DEPTH: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_restore_point_resolution_chain_depth",
		"Observed sqlite restore point resolution parent-chain depth.",
		&["node_id"],
		vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0],
		*REGISTRY
	).unwrap();

	// ---- Background layer: compaction ----

	pub static ref SQLITE_COMPACTION_PASS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compaction_pass_total",
		"Total depot background compaction passes by kind and result.",
		&["kind", "result"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_PASS_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_compaction_pass_duration",
		"Duration of depot background compaction passes by kind and result.",
		&["kind", "result"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_HOT_SHARDS_INSTALLED: Histogram = register_histogram_with_registry!(
		"sqlite_compaction_hot_shards_installed",
		"Number of hot shards installed per successful depot hot compaction pass.",
		SHARD_COUNT_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	/// Labelled by the row family cleared. `sum without(row_kind)` reproduces the previous unlabelled
	/// total, so cross-migration continuity is preserved for aggregate queries.
	pub static ref SQLITE_COMPACTION_FDB_KEYS_RECLAIMED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compaction_fdb_keys_reclaimed_total",
		"Total FDB keys cleared by depot FDB reclaim passes, by row kind.",
		&["row_kind"],
		*REGISTRY
	).unwrap();

	/// Counts key plus value bytes, because that is what clearing the row actually frees, and it is
	/// what the quota credit for a billable row is computed from. The previous unlabelled counter
	/// counted value bytes alone, so this series steps up against its own history.
	pub static ref SQLITE_COMPACTION_FDB_BYTES_RECLAIMED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compaction_fdb_bytes_reclaimed_total",
		"Total FDB key plus value bytes cleared by depot FDB reclaim passes, by row kind.",
		&["row_kind"],
		*REGISTRY
	).unwrap();

	/// The denominator for reclaim scan efficiency. Reclaim charges read budget for every row it
	/// walks, retained or not, so a pass that scans far more than it clears is spending the branch's
	/// read budget to make no progress.
	pub static ref SQLITE_COMPACTION_RECLAIM_ROWS_SCANNED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compaction_reclaim_rows_scanned_total",
		"Total rows read into a depot reclaim window, cleared or retained, by row kind.",
		&["row_kind"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_RECLAIM_SCAN_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compaction_reclaim_scan_total",
		"Total depot reclaim commit scans by whether they reached the end of the reclaimable range.",
		&["outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_RECLAIM_CURSOR_ADVANCE_TXIDS: Histogram = register_histogram_with_registry!(
		"sqlite_compaction_reclaim_cursor_advance_txids",
		"Txids the commit scan cursor advanced per depot reclaim sweep chunk.",
		RECLAIM_CURSOR_ADVANCE_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	/// Hot shard image bytes a fold wrote, labelled by where they landed.
	///
	/// `destination="staging"` is the staging subspace, and pairs with
	/// `sqlite_compaction_hot_staging_cleared_bytes_total`: the difference between their rates is
	/// whether staging is accumulating. The absolute resident level is not recoverable from counters
	/// across a process restart, but the growth rate is, and the rate is what says whether the
	/// staging subspace is the one growing.
	///
	/// `destination="shard"` is a direct fold under `sqlite.compaction_hot_fold_direct_to_shard`,
	/// which writes the image into its live rows and stages nothing. Without the label the name would
	/// claim a staging write that never happened, and the two modes would be indistinguishable in the
	/// one series that says how much folding is going on.
	pub static ref SQLITE_COMPACTION_HOT_STAGED_BYTES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compaction_hot_staged_bytes_total",
		"Total hot shard image bytes a depot fold wrote, by where the image landed.",
		&["destination"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_HOT_STAGING_CLEARED_BYTES_TOTAL: IntCounter = register_int_counter_with_registry!(
		"sqlite_compaction_hot_staging_cleared_bytes_total",
		"Total hot shard image bytes cleared from the depot staging subspace by staging cleanup.",
		*REGISTRY
	).unwrap();

	/// Image bytes hot install published, whether it copied them out of the staging area or adopted
	/// them where a direct fold already wrote them. This is installed volume, so it does not collapse
	/// to zero when folds stop staging.
	///
	/// It is NOT write amplification on its own. Against a staging fold this equals the staged bytes
	/// because every image really is written twice; against a direct fold it equals them because the
	/// same image is counted once on each side. `sqlite_compaction_hot_install_copied_bytes_total` is
	/// the half that measures the second write.
	pub static ref SQLITE_COMPACTION_HOT_INSTALLED_BYTES_TOTAL: IntCounter = register_int_counter_with_registry!(
		"sqlite_compaction_hot_installed_bytes_total",
		"Total hot shard image bytes depot hot install published into live shard rows, copied or adopted in place.",
		*REGISTRY
	).unwrap();

	/// The half of `sqlite_compaction_hot_installed_bytes_total` that install actually rewrote.
	///
	/// This is hot compaction's write amplification: divide it by the fold bytes and a staging fold
	/// reads 1.0 (every image written to FDB a second time) while a direct fold reads 0.0. Watching
	/// this go to zero is how you confirm `sqlite.compaction_hot_fold_direct_to_shard` is doing what it
	/// claims, and it is the only counter here that distinguishes the two modes by volume.
	pub static ref SQLITE_COMPACTION_HOT_INSTALL_COPIED_BYTES_TOTAL: IntCounter = register_int_counter_with_registry!(
		"sqlite_compaction_hot_install_copied_bytes_total",
		"Total staged hot shard image bytes depot hot install rewrote into live shard rows. Zero for direct-to-shard folds.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_HOT_STAGED_BYTES: Histogram = register_histogram_with_registry!(
		"sqlite_compaction_hot_staged_bytes",
		"Hot shard image bytes staged per successful depot hot staging pass.",
		HOT_SHARD_BYTES_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_HOT_INSTALLED_BYTES: Histogram = register_histogram_with_registry!(
		"sqlite_compaction_hot_installed_bytes",
		"Hot shard image bytes copied per successful depot hot install pass.",
		HOT_SHARD_BYTES_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_COLD_OBJECTS_UPLOADED_TOTAL: IntCounter = register_int_counter_with_registry!(
		"sqlite_compaction_cold_objects_uploaded_total",
		"Total cold-tier shard objects uploaded by depot cold compaction passes.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_COLD_UPLOAD_BYTES_TOTAL: IntCounter = register_int_counter_with_registry!(
		"sqlite_compaction_cold_upload_bytes_total",
		"Total cold-tier shard object bytes uploaded by depot cold compaction passes.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_COLD_UPLOAD_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_compaction_cold_upload_duration",
		"Duration of individual depot cold compaction shard object uploads, by result.",
		&["result"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_COLD_OBJECT_BYTES: Histogram = register_histogram_with_registry!(
		"sqlite_compaction_cold_object_bytes",
		"Size of individual cold-tier shard objects uploaded by depot cold compaction.",
		COLD_OBJECT_BYTES_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_COLD_OBJECTS_DELETED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compaction_cold_objects_deleted_total",
		"Total cold-tier objects deleted by depot reclaim, by delete phase.",
		&["phase"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_COLD_OBJECT_DELETE_FAILURES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compaction_cold_object_delete_failures_total",
		"Total cold-tier objects a depot reclaim batch delete failed to remove, by delete phase.",
		&["phase"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_LAG_TXIDS: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_compaction_lag_txids",
		"Observed depot compaction backlog in txids per branch at manager refresh, by tier.",
		&["tier"],
		LAG_TXID_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_THROTTLED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compaction_throttled_total",
		"Total depot compaction activity calls that backed off because a compaction throttle budget (read or write) was spent, by kind.",
		&["kind"],
		*REGISTRY
	).unwrap();
}

#[cfg(debug_assertions)]
lazy_static::lazy_static! {
	pub static ref SQLITE_TAKEOVER_INVARIANT_VIOLATION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_takeover_invariant_violation_total",
		"Total debug sqlite takeover invariant violations.",
		&["node_id", "kind"],
		*REGISTRY
	).unwrap();
}

// ---- Live-layer phase helpers ----

pub fn observe_get_pages_phase(
	node_id: &str,
	phase: &'static str,
	start: Instant,
	result: &'static str,
) {
	let elapsed = start.elapsed();
	SQLITE_GET_PAGES_PHASE_DURATION
		.with_label_values(&[node_id, phase, result])
		.observe(elapsed.as_secs_f64());
	if elapsed.as_secs_f64() >= SLOW_PHASE_WARN_THRESHOLD_SECONDS {
		tracing::warn!(
			node_id,
			phase,
			result,
			duration_ms = elapsed.as_millis() as u64,
			"slow depot get_pages phase"
		);
	}
}

pub fn observe_commit_phase(
	node_id: &str,
	phase: &'static str,
	start: Instant,
	result: &'static str,
) {
	let elapsed = start.elapsed();
	SQLITE_COMMIT_PHASE_DURATION
		.with_label_values(&[node_id, phase, result])
		.observe(elapsed.as_secs_f64());
	if elapsed.as_secs_f64() >= SLOW_PHASE_WARN_THRESHOLD_SECONDS {
		tracing::warn!(
			node_id,
			phase,
			result,
			duration_ms = elapsed.as_millis() as u64,
			"slow depot commit phase"
		);
	}
}

pub async fn observe_get_pages_phase_result<T>(
	node_id: &str,
	phase: &'static str,
	future: impl Future<Output = Result<T>>,
) -> Result<T> {
	let start = Instant::now();
	let result = future.await;
	observe_get_pages_phase(node_id, phase, start, result_label(&result));
	result
}

pub async fn observe_commit_phase_result<T>(
	node_id: &str,
	phase: &'static str,
	future: impl Future<Output = Result<T>>,
) -> Result<T> {
	let start = Instant::now();
	let result = future.await;
	observe_commit_phase(node_id, phase, start, result_label(&result));
	result
}

fn result_label<T>(result: &Result<T>) -> &'static str {
	if result.is_ok() { "ok" } else { "error" }
}

// ---- Background-layer compaction helpers ----

/// Result label for a completed compaction pass. On `Err` the pass surfaced a transport/activity
/// error; otherwise it maps the durable job status onto a coarse outcome.
///
/// A pass that returns `Requested` is resumable work, not a failed pass: it committed whatever it
/// was admitted for and handed a cursor back for the workflow to re-dispatch. Labelling those
/// `failed` made a healthy throttled cluster look like it was failing every install, so
/// `throttled`/`incomplete` keep `failed` meaning "this pass hit an error".
///
/// A pass that parked on the admission percent is likewise not a failure and not resumable work it
/// chose to defer: it was never admitted. It reports `Requested` like a throttled pass, so callers
/// whose output carries an admission flag pass it here rather than letting it read as `incomplete`.
fn compaction_result_label<T>(
	result: &Result<T>,
	status: impl Fn(&T) -> &CompactionJobStatus,
	throttled: impl Fn(&T) -> bool,
	admission_blocked: impl Fn(&T) -> bool,
) -> &'static str {
	match result {
		Err(_) => RESULT_ERROR,
		Ok(output) => match status(output) {
			CompactionJobStatus::Succeeded => RESULT_SUCCEEDED,
			CompactionJobStatus::Rejected { .. } => RESULT_REJECTED,
			CompactionJobStatus::Failed { .. } => RESULT_FAILED,
			CompactionJobStatus::Requested => {
				if throttled(output) {
					RESULT_THROTTLED
				} else if admission_blocked(output) {
					RESULT_ADMISSION_BLOCKED
				} else {
					RESULT_INCOMPLETE
				}
			}
		},
	}
}

fn record_compaction_pass(kind: &'static str, start: Instant, result: &'static str) {
	SQLITE_COMPACTION_PASS_TOTAL
		.with_label_values(&[kind, result])
		.inc();
	SQLITE_COMPACTION_PASS_DURATION
		.with_label_values(&[kind, result])
		.observe(start.elapsed().as_secs_f64());
}

/// Records one hot staging pass.
///
/// Staging needs its own result mapping rather than the shared helper. It reports `Succeeded` for
/// four different things: a slice staged in full (`slice` set), a slice left partly staged at the
/// early timeout (`next_stage_cursor` set), a drain that found nothing left (neither set), and a
/// drain stalled on a commit the budget cannot admit (`stalled_at_txid` set). Under the shared
/// mapping all four read as `succeeded`, which makes a wide slice look like several successful
/// passes, lets drain-end no-ops dominate the success rate, and hides a wedged branch entirely.
pub fn record_hot_stage(
	start: Instant,
	result: &Result<StageHotSliceOutput>,
	direct_to_shard: bool,
) {
	let label = match result {
		Err(_) => RESULT_ERROR,
		Ok(output) => match &output.status {
			CompactionJobStatus::Succeeded => {
				if output.next_stage_cursor.is_some() {
					RESULT_INCOMPLETE
				} else if output.slice.is_some() {
					RESULT_SUCCEEDED
				} else if output.stalled_at_txid.is_some() {
					RESULT_STALLED
				} else {
					RESULT_DRAINED
				}
			}
			CompactionJobStatus::Rejected { .. } => RESULT_REJECTED,
			CompactionJobStatus::Failed { .. } => RESULT_FAILED,
			CompactionJobStatus::Requested => {
				if output.throttled {
					RESULT_THROTTLED
				} else if output.admission_blocked {
					RESULT_ADMISSION_BLOCKED
				} else {
					RESULT_INCOMPLETE
				}
			}
		},
	};
	record_compaction_pass(PASS_HOT_STAGE, start, label);

	// Bytes folded by a call that stopped at its early timeout count too: they are durable rows
	// either way, and dropping them would under-report exactly the wide slices that fold the most.
	if let Ok(output) = result
		&& output.staged_bytes > 0
	{
		let destination = if direct_to_shard {
			FOLD_DESTINATION_SHARD
		} else {
			FOLD_DESTINATION_STAGING
		};
		SQLITE_COMPACTION_HOT_STAGED_BYTES_TOTAL
			.with_label_values(&[destination])
			.inc_by(output.staged_bytes);
		SQLITE_COMPACTION_HOT_STAGED_BYTES.observe(output.staged_bytes as f64);
	}
}

pub fn record_hot_install(start: Instant, result: &Result<InstallHotJobOutput>) {
	let label = compaction_result_label(
		result,
		|output| &output.status,
		|output| output.throttled,
		// Hot install runs on the manager and is never gated by the admission percent; the companion
		// drain parks before a job ever reaches install.
		|_| false,
	);
	record_compaction_pass(PASS_HOT_INSTALL, start, label);
	if let Ok(output) = result {
		if output.status == CompactionJobStatus::Succeeded {
			SQLITE_COMPACTION_HOT_SHARDS_INSTALLED.observe(output.installed_shard_count as f64);
			if output.installed_shard_bytes > 0 {
				SQLITE_COMPACTION_HOT_INSTALLED_BYTES_TOTAL.inc_by(output.installed_shard_bytes);
				SQLITE_COMPACTION_HOT_INSTALLED_BYTES.observe(output.installed_shard_bytes as f64);
			}
			// Recorded separately from the installed total, and deliberately not gated on being
			// non-zero: a direct fold's zero is the signal, and skipping it would leave the counter
			// looking stale rather than flat.
			SQLITE_COMPACTION_HOT_INSTALL_COPIED_BYTES_TOTAL.inc_by(output.copied_shard_bytes);
		}
	}
}

/// Records the per-row-kind volume a reclaim pass scanned and cleared. Shared by the v2 sweep and
/// the v1 delete so the counters stay continuous across the migration; only the pass label differs.
///
/// Scanned is recorded even when nothing was cleared. A pass that reads a whole window of retained
/// history spends read budget and reports zero cleared rows, and that combination is the signal
/// worth seeing.
fn record_reclaim_row_stats(stats: &ReclaimRowStats) {
	for (row_kind, volume) in stats.by_kind() {
		if volume.scanned_key_count > 0 {
			SQLITE_COMPACTION_RECLAIM_ROWS_SCANNED_TOTAL
				.with_label_values(&[row_kind])
				.inc_by(u64::from(volume.scanned_key_count));
		}
		if volume.key_count > 0 {
			SQLITE_COMPACTION_FDB_KEYS_RECLAIMED_TOTAL
				.with_label_values(&[row_kind])
				.inc_by(u64::from(volume.key_count));
		}
		if volume.byte_count > 0 {
			SQLITE_COMPACTION_FDB_BYTES_RECLAIMED_TOTAL
				.with_label_values(&[row_kind])
				.inc_by(volume.byte_count);
		}
	}

	if stats.staging_blob_bytes_cleared > 0 {
		SQLITE_COMPACTION_HOT_STAGING_CLEARED_BYTES_TOTAL.inc_by(stats.staging_blob_bytes_cleared);
	}
}

pub fn record_reclaim_sweep_chunk(start: Instant, result: &Result<SweepCommitDeltaChunkOutput>) {
	let label = compaction_result_label(
		result,
		|output| &output.status,
		|output| output.throttled,
		|output| output.admission_blocked,
	);
	record_compaction_pass(PASS_RECLAIM_SWEEP, start, label);
	if let Ok(output) = result {
		if output.status == CompactionJobStatus::Succeeded {
			record_reclaim_row_stats(&output.row_stats);
			SQLITE_COMPACTION_RECLAIM_SCAN_TOTAL
				.with_label_values(&[if output.commit_scan_complete {
					RECLAIM_SCAN_COMPLETE
				} else {
					RECLAIM_SCAN_PARTIAL
				}])
				.inc();
			SQLITE_COMPACTION_RECLAIM_CURSOR_ADVANCE_TXIDS
				.observe(output.cursor_advance_txids as f64);
		}
	}
}

/// Records one dead-shard version-retention sweep dispatch.
///
/// The volume is passed separately from the result so a dispatch that errored partway still reports
/// the chunks it committed before failing. Those deletions are durable, and dropping them would make
/// the counter disagree with the FDB footprint it is meant to explain.
pub fn record_reclaim_dead_shard_sweep(
	start: Instant,
	result: &Result<SweepDeadShardVersionsOutput>,
	volume: &ReclaimRowVolume,
) {
	// The sweep is charged against the compaction throttle but never parks on it or on admission: it
	// only returns `Requested` for an early-timeout resume, which is `incomplete`.
	let label = compaction_result_label(result, |output| &output.status, |_| false, |_| false);
	record_compaction_pass(PASS_RECLAIM_DEAD_SHARD, start, label);
	if volume.scanned_key_count > 0 {
		SQLITE_COMPACTION_RECLAIM_ROWS_SCANNED_TOTAL
			.with_label_values(&[RECLAIM_ROW_KIND_SHARD])
			.inc_by(u64::from(volume.scanned_key_count));
	}
	if volume.key_count > 0 {
		SQLITE_COMPACTION_FDB_KEYS_RECLAIMED_TOTAL
			.with_label_values(&[RECLAIM_ROW_KIND_SHARD])
			.inc_by(u64::from(volume.key_count));
	}
	if volume.byte_count > 0 {
		SQLITE_COMPACTION_FDB_BYTES_RECLAIMED_TOTAL
			.with_label_values(&[RECLAIM_ROW_KIND_SHARD])
			.inc_by(volume.byte_count);
	}
}

pub fn record_reclaim_fdb(start: Instant, result: &Result<ReclaimFdbJobOutput>) {
	let label = compaction_result_label(
		result,
		|output| &output.status,
		|output| output.throttled,
		|output| output.admission_blocked,
	);
	record_compaction_pass(PASS_RECLAIM_FDB, start, label);
	if let Ok(output) = result {
		if output.status == CompactionJobStatus::Succeeded {
			for reference in &output.output_refs {
				record_reclaim_row_stats(&reference.row_stats);
			}
		}
	}
}

/// Records that a compaction activity backed off because a compaction throttle budget was spent.
pub fn record_compaction_throttled(kind: &'static str) {
	SQLITE_COMPACTION_THROTTLED_TOTAL
		.with_label_values(&[kind])
		.inc();
}

/// Records one cold-tier shard object upload. Volume counters only advance on a successful put so
/// they measure bytes actually moved into cold storage, while the duration histogram covers both
/// outcomes so a slow failing tier stays visible.
pub fn record_cold_object_upload(start: Instant, bytes: usize, result: &Result<()>) {
	let label = if result.is_ok() {
		RESULT_SUCCEEDED
	} else {
		RESULT_FAILED
	};
	SQLITE_COMPACTION_COLD_UPLOAD_DURATION
		.with_label_values(&[label])
		.observe(start.elapsed().as_secs_f64());
	if result.is_ok() {
		SQLITE_COMPACTION_COLD_OBJECTS_UPLOADED_TOTAL.inc();
		SQLITE_COMPACTION_COLD_UPLOAD_BYTES_TOTAL.inc_by(bytes as u64);
		SQLITE_COMPACTION_COLD_OBJECT_BYTES.observe(bytes as f64);
	}
}

/// Records cold-tier objects confirmed gone. Never count attempted keys here; a batch delete can
/// report success overall while individual keys fail.
pub fn record_cold_objects_deleted(phase: &'static str, count: usize) {
	if count == 0 {
		return;
	}
	SQLITE_COMPACTION_COLD_OBJECTS_DELETED_TOTAL
		.with_label_values(&[phase])
		.inc_by(count as u64);
}

/// Records cold-tier objects a batch delete left in place.
pub fn record_cold_object_delete_failures(phase: &'static str, count: usize) {
	if count == 0 {
		return;
	}
	SQLITE_COMPACTION_COLD_OBJECT_DELETE_FAILURES_TOTAL
		.with_label_values(&[phase])
		.inc_by(count as u64);
}

/// Records the observed hot and cold compaction backlog for a live branch at manager refresh. `hot`
/// is the txid span awaiting hot install; `cold` is the hot-installed but not-yet-cold-published
/// span.
pub fn observe_lag(hot_lag_txids: u64, cold_lag_txids: u64) {
	SQLITE_COMPACTION_LAG_TXIDS
		.with_label_values(&[LAG_TIER_HOT])
		.observe(hot_lag_txids as f64);
	SQLITE_COMPACTION_LAG_TXIDS
		.with_label_values(&[LAG_TIER_COLD])
		.observe(cold_lag_txids as f64);
}

#[cfg(test)]
#[path = "../tests/inline/metrics_compaction.rs"]
mod tests;
