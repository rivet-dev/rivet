use rivet_metrics::{prometheus::*, REGISTRY};

lazy_static::lazy_static! {
	pub static ref PEGBOARD_CLIENT_DUPLICATE_EVENT: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_client_duplicate_event",
		"Duplicate client event that was attempted to be inserted.",
		&["client_id", "index"],
		*REGISTRY
	).unwrap();

	pub static ref PEGBOARD_CLIENT_LAST_PING: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_client_last_ping",
		"Last client ping timestamp, in ms.",
		&["client_id"],
		*REGISTRY
	).unwrap();

	pub static ref PEGBOARD_CLIENT_ACTORS_ALLOCATED: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_client_actors_allocated",
		"Total actors allocated on a client.",
		&["client_id"],
		*REGISTRY
	).unwrap();
}
