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
}
