//! Metrics definitions for the sqlite-storage compactor.

use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref SQLITE_COMPACTOR_LAG: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_compactor_lag_seconds",
		"Estimated lag observed by stateless sqlite compaction.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTOR_LEASE_TAKE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compactor_lease_take_total",
		"Total sqlite compactor lease take attempts.",
		&["node_id", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTOR_LEASE_HELD_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_compactor_lease_held_seconds",
		"Duration sqlite compactor leases were held.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTOR_LEASE_RENEWAL_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compactor_lease_renewal_total",
		"Total sqlite compactor lease renewal attempts.",
		&["node_id", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTOR_PASS_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_compactor_pass_duration_seconds",
		"Duration of stateless sqlite compaction passes.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTOR_PAGES_FOLDED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compactor_pages_folded_total",
		"Total pages folded by stateless sqlite compaction.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTOR_DELTAS_FREED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compactor_deltas_freed_total",
		"Total delta blobs freed by stateless sqlite compaction.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTOR_COMPARE_AND_CLEAR_NOOP_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compactor_compare_and_clear_noop_total",
		"Total compactor PIDX compare-and-clear operations that left a newer value in place.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_EVICTION_OCC_ABORT_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_eviction_occ_abort_total",
		"Total sqlite eviction compactor OCC aborts.",
		&["node_id", "reason"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_SHARD_VERSIONS_PER_SHARD: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_shard_versions_per_shard",
		"Observed sqlite SHARD versions per shard before a hot compaction write.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMPACTOR_UPS_PUBLISH_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_compactor_ups_publish_total",
		"Total sqlite compactor UPS publish attempts.",
		&["node_id", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_STORAGE_USED_BYTES: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_storage_used_bytes",
		"Sampled sqlite storage bytes by database.",
		&["node_id", "database_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COLD_PASS_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_cold_pass_duration_seconds",
		"Duration of sqlite cold compactor phases.",
		&["node_id", "phase"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COLD_PASS_LAYERS_UPLOADED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_cold_pass_layers_uploaded_total",
		"Total sqlite cold compactor layer uploads.",
		&["node_id", "kind"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COLD_PASS_BYTES_UPLOADED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_cold_pass_bytes_uploaded_total",
		"Total sqlite cold compactor bytes uploaded.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COLD_LEASE_TAKE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_cold_lease_take_total",
		"Total sqlite cold compactor lease take attempts.",
		&["node_id", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_EVICTION_PASS_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_eviction_pass_duration_seconds",
		"Duration of sqlite eviction compactor passes.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_EVICTION_PASS_SHARDS_CLEARED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_eviction_pass_shards_cleared_total",
		"Total sqlite SHARD versions cleared by eviction passes.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_EVICTION_PASS_DELTAS_CLEARED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_eviction_pass_deltas_cleared_total",
		"Total sqlite DELTA chunks cleared by eviction passes.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PENDING_MARKER_ORPHAN_CLEANED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_pending_marker_orphan_cleaned_total",
		"Total stale sqlite cold pending markers cleaned.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_S3_REQUEST_FAILURES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_s3_request_failures_total",
		"Total sqlite cold-tier request failures.",
		&["node_id", "op"],
		*REGISTRY
	).unwrap();
}

#[cfg(debug_assertions)]
lazy_static::lazy_static! {
	pub static ref SQLITE_QUOTA_VALIDATE_MISMATCH_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_quota_validate_mismatch_total",
		"Total debug quota validation passes where the manual byte tally did not match the quota counter.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_TAKEOVER_INVARIANT_VIOLATION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_takeover_invariant_violation_total",
		"Total debug sqlite takeover invariant violations.",
		&["node_id", "kind"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_FENCE_MISMATCH_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_fence_mismatch_total",
		"Total debug sqlite fence mismatches.",
		&["node_id"],
		*REGISTRY
	).unwrap();
}
