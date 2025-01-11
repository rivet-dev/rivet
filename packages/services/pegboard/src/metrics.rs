use rivet_metrics::{prometheus::*, REGISTRY};

lazy_static::lazy_static! {
	pub static ref CLIENT_DUPLICATE_EVENT: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_client_duplicate_event",
		"Duplicate client event that was attempted to be inserted.",
		&["client_id", "index"],
		*REGISTRY
	).unwrap();

	pub static ref CLIENT_ACTORS_ALLOCATED: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_client_actors_allocated",
		"Total actors allocated on a client.",
		&["datacenter_id", "client_id", "flavor", "inactive"],
		*REGISTRY
	).unwrap();
}
