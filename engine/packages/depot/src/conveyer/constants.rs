/// Spec section 8 caps database branch ancestry so fork, read, and restore_point walks stay bounded.
pub const MAX_FORK_DEPTH: u8 = 16;

/// Spec section 8.1 caps bucket branch ancestry so bucket resolution stays bounded.
pub const MAX_BUCKET_DEPTH: u8 = 16;

/// Spec section 9 caps restore_points per bucket to bound pin recomputation work.
pub const MAX_RESTORE_POINTS_PER_BUCKET: u32 = 1024;

/// Spec section 12.1 caps FDB shard-version amplification when eviction lags hot compaction.
pub const MAX_SHARD_VERSIONS_PER_SHARD: u32 = 32;

/// Spec section 12.1 keeps hot commit and VTX history for recent restore point resolution.
pub const HOT_RETENTION_FLOOR_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// Spec section 12.3 buckets access touches to bound eviction-index churn to about one write per minute.
pub const ACCESS_TOUCH_THROTTLE_MS: i64 = 60_000;

/// Spec section 12.1 keeps an eviction safety margin behind the latest hot pass.
pub const SHARD_RETENTION_MARGIN: u64 = 64;

/// Spec section 13 retains frozen rollback targets by wall-clock age rather than fixed head txid.
pub const FROZEN_BRANCH_RETENTION_MS: i64 = 30 * 24 * 60 * 60 * 1000;

/// Spec section 12.3 keeps recently accessed hot data resident before eviction can clear it.
pub const HOT_CACHE_WINDOW_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// Spec section 12.2 delays pending-marker cleanup long enough for a live cold pass to finish.
pub const STALE_MARKER_AGE_MS: i64 = 10 * 60 * 1000;

/// Spec section 12.2 burst mode doubles the hot quota while the cold tier is degraded.
pub const HOT_BURST_MULTIPLIER: i64 = 2;

/// Spec section 12.2 derives burst mode from cold-drain lag, matching the cold trigger window.
pub const HOT_BURST_COLD_LAG_THRESHOLD_TXIDS: u64 = 2048;

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

/// Workflow compaction uploads at most one cold shard object per S3 activity.
pub const CMP_S3_UPLOAD_MAX_OBJECTS: usize = 1;

/// Workflow compaction caps cold shard upload activity payloads.
pub const CMP_S3_UPLOAD_LIMIT_BYTES: usize = 64 * 1024 * 1024;

/// Workflow compaction caps S3 delete activity batches.
pub const CMP_S3_DELETE_MAX_OBJECTS: usize = 100;

/// Workflow compaction waits this long after unpublishing a cold object before deleting it.
pub const CMP_COLD_OBJECT_DELETE_GRACE_MS: i64 = 500;

/// Workflow compaction splits planned activities expected to exceed this wall time.
pub const CMP_ACTIVITY_TARGET_MS: i64 = 30 * 1000;

/// DB manager schedules its next cold compaction check this far in the future after arming.
pub const MANAGER_COLD_COMPACTION_INTERVAL_MS: i64 = 2 * 60 * 1000;

/// DB manager schedules its next reclaim/GC check this far in the future after arming.
pub const MANAGER_RECLAIM_INTERVAL_MS: i64 = 10 * 60 * 1000;
