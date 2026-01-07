use rivet_metrics::{MICRO_BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref ACTOR_PENDING_ALLOCATION: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_actor_pending_allocation",
		"Total actors waiting for availability.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_ALLOCATE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_actor_allocate_duration",
		"Total duration to reserve resources for an actor.",
		&["did_reserve"],
		MICRO_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref RUNNER_VERSION_UPGRADE_DRAIN_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_runner_version_upgrade_drain_total",
		"Count of runners drained due to version upgrade.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();

	pub static ref SERVERLESS_OUTBOUND_REQ_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_runner_version_upgrade_drain_total",
		"Count of serverless outbound requests made.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();
}
