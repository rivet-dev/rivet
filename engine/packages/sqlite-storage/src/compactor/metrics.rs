//! Metrics definitions for the sqlite-storage compactor.

use rivet_metrics::{REGISTRY, prometheus::*};

lazy_static::lazy_static! {
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
}
