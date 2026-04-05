use rivet_metrics::{REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref REQ_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_outbound_req_total",
		"Count of serverless outbound requests made.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();

	pub static ref REQ_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_outbound_req_active",
		"Count of serverless outbound requests currently active.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();
}
