use rivet_metrics::{
	TASK_POLL_BUCKETS,
	otel::{global::*, metrics::*},
};

lazy_static::lazy_static! {
	static ref METER: Meter = meter("rivet-runtime");

	// MARK: Tokio
	pub static ref TOKIO_THREAD_COUNT: UpDownCounter<i64> = METER.i64_up_down_counter("rivet_tokio_thread_count")
		.with_description("Total number of Tokio threads.")
		.build();
	pub static ref TOKIO_GLOBAL_QUEUE_DEPTH: Gauge<u64> = METER.u64_gauge("rivet_tokio_global_queue_depth")
		.with_description("Number of pending tasks in the global queue.")
		.build();
	pub static ref TOKIO_TASK_TOTAL: Counter<u64> = METER.u64_counter("rivet_tokio_task_total")
		.with_description("Total number of spawned tasks.")
		.build();
	pub static ref TOKIO_ACTIVE_TASK_COUNT: Gauge<u64> = METER.u64_gauge("rivet_tokio_active_task_count")
		.with_description("Total number of active (running or sleeping) tasks.")
		.build();
	/// Expected attributes: "worker"
	pub static ref TOKIO_WORKER_OVERFLOW_COUNT: Gauge<u64> = METER.u64_gauge("rivet_tokio_worker_overflow_count")
		.with_description("Number of times the given worker thread saturated its local queue.")
		.build();
	/// Expected attributes: "worker"
	pub static ref TOKIO_WORKER_LOCAL_QUEUE_DEPTH: Gauge<u64> = METER.u64_gauge("rivet_tokio_worker_local_queue_depth")
		.with_description("Number of pending tasks in a worker's queue.")
		.build();
	pub static ref TOKIO_TASK_POLL_DURATION: Histogram<f64> = METER.f64_histogram("rivet_tokio_task_poll_duration")
		.with_description("Duration to poll a task.")
		.with_boundaries(TASK_POLL_BUCKETS.to_vec())
		.build();
}
