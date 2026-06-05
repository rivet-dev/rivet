//! Metrics definitions shared by depot runtime components.

use rivet_metrics::{REGISTRY, prometheus::*};

#[cfg(debug_assertions)]
lazy_static::lazy_static! {
	pub static ref SQLITE_TAKEOVER_INVARIANT_VIOLATION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_takeover_invariant_violation_total",
		"Total debug sqlite takeover invariant violations.",
		&["node_id", "kind"],
		*REGISTRY
	).unwrap();
}
