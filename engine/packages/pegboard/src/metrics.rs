use rivet_metrics::{
	MICRO_BUCKETS,
	otel::{global::*, metrics::*},
};

lazy_static::lazy_static! {
	static ref METER: Meter = meter("rivet-pegboard");

	/// Expected attributes: "namespace_id", "runner_name"
	pub static ref ACTOR_PENDING_ALLOCATION: Gauge<f64> = METER.f64_gauge("rivet_pegboard_actor_pending_allocation")
		.with_description("Total actors waiting for availability.")
		.build();

	/// Expected attributes: "did_reserve"
	pub static ref ACTOR_ALLOCATE_DURATION: Histogram<f64> = METER.f64_histogram("rivet_pegboard_actor_allocate_duration")
		.with_description("Total duration to reserve resources for an actor.")
		.with_boundaries(MICRO_BUCKETS.to_vec())
		.build();
}
