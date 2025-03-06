use rivet_metrics::{prometheus::*, REGISTRY};

lazy_static::lazy_static! {
	pub static ref CLIENT_DUPLICATE_EVENT: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_client_duplicate_event",
		"Duplicate client event that was attempted to be inserted.",
		&["client_id", "index"],
		*REGISTRY
	).unwrap();

	pub static ref CLIENT_CPU_ALLOCATED: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_client_cpu_allocated",
		"Total millicores of cpu allocated on a client.",
		&["client_id", "flavor"],
		*REGISTRY
	).unwrap();

	pub static ref CLIENT_MEMORY_ALLOCATED: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_client_memory_allocated",
		"Total MiB of memory allocated on a client.",
		&["client_id", "flavor"],
		*REGISTRY
	).unwrap();
}
