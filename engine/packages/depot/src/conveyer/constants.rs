/// Spec section 8 caps database branch ancestry so fork, read, and restore_point walks stay bounded.
pub const MAX_FORK_DEPTH: u8 = 16;

/// Spec section 8.1 caps bucket branch ancestry so bucket resolution stays bounded.
pub const MAX_BUCKET_DEPTH: u8 = 16;

/// Spec section 9 caps restore_points per bucket to bound pin recomputation work.
pub const MAX_RESTORE_POINTS_PER_BUCKET: u32 = 1024;

/// Spec section 12.3 buckets access touches to bound eviction-index churn to about one write per minute.
pub const ACCESS_TOUCH_THROTTLE_MS: i64 = 60_000;

/// Workflow compaction signal payloads stay below Gasoline's durable signal size budget.
pub const CMP_SIGNAL_PAYLOAD_LIMIT_BYTES: usize = 64 * 1024;

/// Workflow compaction job descriptors stay small enough for durable workflow state.
pub const CMP_JOB_DESCRIPTOR_LIMIT_BYTES: usize = 256 * 1024;

/// Workflow compaction install and reclaim activities cap each FDB transaction by key count.
pub const CMP_FDB_BATCH_MAX_KEYS: usize = 500;

/// Workflow compaction install and reclaim activities cap each FDB transaction by value bytes.
pub const CMP_FDB_BATCH_MAX_VALUE_BYTES: usize = 2 * 1024 * 1024;

/// SQLite commits must stay comfortably below the compaction input caps so one txid always fits in
/// a hot/reclaim compaction batch. 320 pages leaves room under the 500-key cap for PIDX rows,
/// delta chunk keys, commit metadata, VTX rows, root updates, and future per-commit overhead.
pub const MAX_COMMIT_DIRTY_PAGES: usize = 320;
pub const MAX_COMMIT_RAW_DIRTY_BYTES: usize =
	MAX_COMMIT_DIRTY_PAGES * crate::conveyer::keys::PAGE_SIZE as usize;

/// Commit deltas are chunked into rows of this many bytes.
pub const DELTA_CHUNK_BYTES: usize = 10_000;

// Reclaim deletes one delta blob's chunk rows atomically, so a blob larger
// than an empty batch budget would livelock reclaim. The 10% margin covers
// LTX frame overhead on incompressible pages.
const _: () = assert!(
	(MAX_COMMIT_RAW_DIRTY_BYTES + MAX_COMMIT_RAW_DIRTY_BYTES / 10) / DELTA_CHUNK_BYTES + 2
		<= CMP_FDB_BATCH_MAX_KEYS
);
const _: () = assert!(
	MAX_COMMIT_RAW_DIRTY_BYTES + MAX_COMMIT_RAW_DIRTY_BYTES / 10 <= CMP_FDB_BATCH_MAX_VALUE_BYTES
);

/// Workflow compaction splits planned activities expected to exceed this wall time.
pub const CMP_ACTIVITY_TARGET_MS: i64 = 30 * 1000;

/// DB manager schedules its next reclaim/GC check this far in the future after arming.
pub const MANAGER_RECLAIM_INTERVAL_MS: i64 = 10 * 60 * 1000;

/// DB manager retries a rejected reclaim pass after this short backoff instead
/// of waiting a full reclaim interval; rejections are plan/execute races, not
/// failures.
pub const MANAGER_RECLAIM_RETRY_MS: i64 = 5 * 1000;
