use rivet_metrics::{REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref SQLITE_ADMIN_OP_RATE_LIMITED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sqlite_admin_op_rate_limited_total",
		"Total sqlite admin operations rejected by namespace rate or concurrency gates.",
		&["namespace"],
		*REGISTRY
	).unwrap();
}
