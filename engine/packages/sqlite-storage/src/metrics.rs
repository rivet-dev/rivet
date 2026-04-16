//! Metrics definitions for sqlite-storage.

use std::time::Duration;

use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

use crate::types::DBHead;

lazy_static::lazy_static! {
	pub static ref SQLITE_COMMIT_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_v2_commit_duration_seconds",
		"Duration of sqlite v2 commit operations.",
		&["path"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMMIT_PAGES: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_v2_commit_pages",
		"Number of dirty pages per commit.",
		&["path"],
		vec![1.0, 4.0, 16.0, 64.0, 256.0, 1024.0, 4096.0],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMMIT_TOTAL: IntCounter = register_int_counter_with_registry!(
		"sqlite_v2_commit_total",
		"Total number of sqlite v2 commits.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_GET_PAGES_DURATION: Histogram = register_histogram_with_registry!(
		"sqlite_v2_get_pages_duration_seconds",
		"Duration of sqlite v2 get_pages operations.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_GET_PAGES_COUNT: Histogram = register_histogram_with_registry!(
		"sqlite_v2_get_pages_count",
		"Number of pages requested per get_pages call.",
		vec![1.0, 4.0, 16.0, 64.0, 256.0],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PIDX_HIT_TOTAL: IntCounter = register_int_counter_with_registry!(
		"sqlite_v2_pidx_hit_total",
		"Pages served from delta via PIDX lookup.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PIDX_MISS_TOTAL: IntCounter = register_int_counter_with_registry!(
		"sqlite_v2_pidx_miss_total",
		"Pages served from shard (no PIDX entry).",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_PASS_DURATION: Histogram = register_histogram_with_registry!(
		"sqlite_v2_compaction_pass_duration_seconds",
		"Duration of a single compaction pass (one shard).",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_PASS_TOTAL: IntCounter = register_int_counter_with_registry!(
		"sqlite_v2_compaction_pass_total",
		"Total compaction passes executed.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_PAGES_FOLDED: IntCounter = register_int_counter_with_registry!(
		"sqlite_v2_compaction_pages_folded_total",
		"Total pages folded from deltas into shards.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_DELTAS_DELETED: IntCounter = register_int_counter_with_registry!(
		"sqlite_v2_compaction_deltas_deleted_total",
		"Total delta entries fully consumed and deleted.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_DELTA_COUNT: IntGauge = register_int_gauge_with_registry!(
		"sqlite_v2_delta_count",
		"Current number of unfolded deltas across all actors.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTION_LAG_SECONDS: Histogram = register_histogram_with_registry!(
		"sqlite_v2_compaction_lag_seconds",
		"Time between commit and compaction of that commit's deltas.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_TAKEOVER_DURATION: Histogram = register_histogram_with_registry!(
		"sqlite_v2_takeover_duration_seconds",
		"Duration of sqlite v2 takeover operations.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_RECOVERY_ORPHANS_CLEANED: IntCounter = register_int_counter_with_registry!(
		"sqlite_v2_recovery_orphans_cleaned_total",
		"Total orphan deltas or stages cleaned during recovery.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_FENCE_MISMATCH_TOTAL: IntCounter = register_int_counter_with_registry!(
		"sqlite_v2_fence_mismatch_total",
		"Total fence mismatch errors returned.",
		*REGISTRY
	).unwrap();
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SqliteStorageMetrics;

impl SqliteStorageMetrics {
	pub fn observe_commit(&self, path: &'static str, dirty_pages: usize, duration: Duration) {
		SQLITE_COMMIT_DURATION
			.with_label_values(&[path])
			.observe(duration.as_secs_f64());
		SQLITE_COMMIT_PAGES
			.with_label_values(&[path])
			.observe(dirty_pages as f64);
	}

	pub fn inc_commit_total(&self) {
		SQLITE_COMMIT_TOTAL.inc();
	}

	pub fn observe_get_pages(&self, page_count: usize, duration: Duration) {
		SQLITE_GET_PAGES_DURATION.observe(duration.as_secs_f64());
		SQLITE_GET_PAGES_COUNT.observe(page_count as f64);
	}

	pub fn add_pidx_hits(&self, hits: usize) {
		if hits > 0 {
			SQLITE_PIDX_HIT_TOTAL.inc_by(hits as u64);
		}
	}

	pub fn add_pidx_misses(&self, misses: usize) {
		if misses > 0 {
			SQLITE_PIDX_MISS_TOTAL.inc_by(misses as u64);
		}
	}

	pub fn observe_compaction_pass(&self, duration: Duration) {
		SQLITE_COMPACTION_PASS_DURATION.observe(duration.as_secs_f64());
	}

	pub fn inc_compaction_pass_total(&self) {
		SQLITE_COMPACTION_PASS_TOTAL.inc();
	}

	pub fn add_compaction_pages_folded(&self, count: usize) {
		if count > 0 {
			SQLITE_COMPACTION_PAGES_FOLDED.inc_by(count as u64);
		}
	}

	pub fn add_compaction_deltas_deleted(&self, count: usize) {
		if count > 0 {
			SQLITE_COMPACTION_DELTAS_DELETED.inc_by(count as u64);
		}
	}

	pub fn set_delta_count_from_head(&self, head: &DBHead) {
		let delta_count = head.head_txid.saturating_sub(head.materialized_txid);
		SQLITE_DELTA_COUNT.set(delta_count.min(i64::MAX as u64) as i64);
	}

	pub fn observe_compaction_lag_seconds(&self, lag_seconds: f64) {
		if lag_seconds.is_finite() && lag_seconds >= 0.0 {
			SQLITE_COMPACTION_LAG_SECONDS.observe(lag_seconds);
		}
	}

	pub fn observe_takeover(&self, duration: Duration) {
		SQLITE_TAKEOVER_DURATION.observe(duration.as_secs_f64());
	}

	pub fn add_recovery_orphans_cleaned(&self, count: usize) {
		if count > 0 {
			SQLITE_RECOVERY_ORPHANS_CLEANED.inc_by(count as u64);
		}
	}

	pub fn inc_fence_mismatch_total(&self) {
		SQLITE_FENCE_MISMATCH_TOTAL.inc();
	}
}
