//! Metrics definitions for the stateless sqlite-storage pump.

use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref SQLITE_PUMP_COMMIT_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_pump_commit_duration_seconds",
		"Duration of stateless sqlite pump commit operations.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_GET_PAGES_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_pump_get_pages_duration_seconds",
		"Duration of stateless sqlite pump get_pages operations.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_COMMIT_DIRTY_PAGE_COUNT: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_pump_commit_dirty_page_count",
		"Number of dirty pages written per stateless sqlite pump commit.",
		&["node_id"],
		vec![0.0, 1.0, 4.0, 16.0, 64.0, 256.0, 1024.0, 4096.0, 8192.0],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_GET_PAGES_PGNO_COUNT: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_pump_get_pages_pgno_count",
		"Number of pages requested per stateless sqlite pump get_pages call.",
		&["node_id"],
		vec![0.0, 1.0, 4.0, 16.0, 64.0, 256.0, 1024.0],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PUMP_PIDX_COLD_SCAN_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_pump_pidx_cold_scan_total",
		"Total stateless sqlite pump get_pages calls that performed a cold PIDX scan.",
		&["node_id"],
		*REGISTRY
	).unwrap();

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

	pub static ref SQLITE_BOOKMARK_CREATE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_bookmark_create_total",
		"Total sqlite bookmark create operations.",
		&["node_id", "kind", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_BOOKMARK_RESOLVE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_bookmark_resolve_total",
		"Total sqlite bookmark resolve operations.",
		&["node_id", "outcome"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_BOOKMARK_RESOLVE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_bookmark_resolve_duration_seconds",
		"Duration of sqlite bookmark resolve operations.",
		&["node_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_BRANCH_ANCESTRY_WALK_DEPTH: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_branch_ancestry_walk_depth",
		"Observed sqlite actor branch ancestry walk depth.",
		&["node_id"],
		vec![0.0, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0],
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
		"Sampled sqlite pinned bookmark status count.",
		&["node_id", "status"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_DR_POSTURE: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_dr_posture",
		"Sampled sqlite disaster-recovery posture.",
		&["node_id", "recoverable_from"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_PINNED_BOOKMARK_COUNT_PER_NAMESPACE: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_pinned_bookmark_count_per_namespace",
		"Sampled sqlite pinned bookmark count per namespace.",
		&["node_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COLD_LAG_VERSIONSTAMPS: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_cold_lag_versionstamps",
		"Sampled sqlite cold lag in versionstamp units by actor.",
		&["node_id", "actor_id"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_BOOKMARK_RESOLUTION_CHAIN_DEPTH: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_bookmark_resolution_chain_depth",
		"Observed sqlite bookmark resolution parent-chain depth.",
		&["node_id"],
		vec![0.0, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0],
		*REGISTRY
	).unwrap();
}
