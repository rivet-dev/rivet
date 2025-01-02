use rivet_metrics::{prometheus::*, REGISTRY};

lazy_static::lazy_static! {
	pub static ref PEGBOARD_DUPLICATE_CLIENT_EVENT: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_duplicate_client_event",
		"Duplicate client event that was attempted to be inserted.",
		&["client_id", "index"],
		*REGISTRY
	).unwrap();
}
