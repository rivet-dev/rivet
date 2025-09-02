use rivet_metrics::{
	BUCKETS, MICRO_BUCKETS,
	otel::{global::*, metrics::*},
};

lazy_static::lazy_static! {
	static ref METER: Meter = meter("rivet-pegboard");

	/// Expected attributes: "client_id", "index"
	pub static ref CLIENT_DUPLICATE_EVENT: Counter<u64> = METER.u64_counter("rivet_pegboard_client_duplicate_event")
		.with_description("Duplicate client event that was attempted to be inserted.")
		.build();

	/// Expected attributes: "client_id", "flavor", "state"
	pub static ref CLIENT_CPU_TOTAL: Gauge<f64> = METER.f64_gauge("rivet_pegboard_client_cpu_total")
		.with_description("Total millicores of cpu available on a client.")
		.build();

	/// Expected attributes: "client_id", "flavor", "state"
	pub static ref CLIENT_MEMORY_TOTAL: Gauge<f64> = METER.f64_gauge("rivet_pegboard_client_memory_total")
		.with_description("Total MiB of memory available on a client.")
		.build();

	/// Expected attributes: "client_id", "flavor", "state"
	pub static ref CLIENT_CPU_ALLOCATED: Gauge<f64> = METER.f64_gauge("rivet_pegboard_client_cpu_allocated")
		.with_description("Total millicores of cpu allocated on a client.")
		.build();

	/// Expected attributes: "client_id", "flavor", "state"
	pub static ref CLIENT_MEMORY_ALLOCATED: Gauge<f64> = METER.f64_gauge("rivet_pegboard_client_memory_allocated")
		.with_description("Total MiB of memory allocated on a client.")
		.build();

	/// Has no expected attributes
	pub static ref ACTOR_CPU_PENDING_ALLOCATION: Gauge<f64> = METER.f64_gauge("rivet_pegboard_actor_cpu_pending_allocation")
		.with_description("Total actor cpu waiting for availability.")
		.build();

	/// Has no expected attributes
	pub static ref ACTOR_MEMORY_PENDING_ALLOCATION: Gauge<f64> = METER.f64_gauge("rivet_pegboard_actor_memory_pending_allocation")
		.with_description("Total actor memory waiting for availability.")
		.build();

	/// Expected attributes: "did_reserve"
	pub static ref ACTOR_ALLOCATE_DURATION: Histogram<f64> = METER.f64_histogram("rivet_pegboard_actor_allocate_duration")
		.with_description("Total duration to reserve resources for an actor.")
		.with_boundaries(MICRO_BUCKETS.to_vec())
		.build();

	/// Has no expected attributes
	pub static ref ACTOR_START_DURATION: Histogram<f64> = METER.f64_histogram("rivet_pegboard_actor_start_duration")
		.with_description("Total duration from actor creation to starting state.")
		.with_boundaries(BUCKETS.to_vec())
		.build();

	/// Expected attributes: "env_id", "flavor"
	pub static ref ENV_CPU_USAGE: Gauge<f64> = METER.f64_gauge("rivet_pegboard_env_cpu_usage")
		.with_description("Total millicores used by an environment.")
		.build();

	/// Expected attributes: "env_id", "flavor"
	pub static ref ENV_MEMORY_USAGE: Gauge<f64> = METER.f64_gauge("rivet_pegboard_env_memory_usage")
		.with_description("Total MiB of memory used by an environment.")
		.build();
}
