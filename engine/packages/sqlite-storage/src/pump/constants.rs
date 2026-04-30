/// Spec section 8 caps database branch ancestry so fork, read, and bookmark walks stay bounded.
pub const MAX_FORK_DEPTH: u8 = 16;

/// Spec section 8.1 caps namespace branch ancestry so namespace resolution stays bounded.
pub const MAX_NAMESPACE_DEPTH: u8 = 16;

/// Spec section 9 caps pinned bookmarks per namespace to bound pin recomputation work.
pub const MAX_PINS_PER_NAMESPACE: u32 = 1024;

/// Spec section 12.1 caps FDB shard-version amplification when eviction lags hot compaction.
pub const MAX_SHARD_VERSIONS_PER_SHARD: u32 = 32;

/// Spec section 12.1 keeps hot commit and VTX history for recent bookmark resolution.
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
pub const HOT_BURST_COLD_LAG_THRESHOLD_TXIDS: u64 = 1024;
