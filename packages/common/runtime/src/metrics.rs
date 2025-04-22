use rivet_metrics::{prometheus::*, REGISTRY, TASK_POLL_BUCKETS};

lazy_static::lazy_static! {
	// MARK: Tokio
	pub static ref TOKIO_THREAD_COUNT: IntGauge =
		register_int_gauge_with_registry!(
			"tokio_thread_count",
			"Total number of Tokio threads.",
			*REGISTRY
		).unwrap();
	pub static ref TOKIO_GLOBAL_QUEUE_DEPTH: IntGauge =
		register_int_gauge_with_registry!(
			"tokio_global_queue_depth",
			"Number of pending tasks in the global queue.",
			*REGISTRY
		).unwrap();
	pub static ref TOKIO_WORKER_OVERFLOW_COUNT: IntGaugeVec = register_int_gauge_vec_with_registry!(
			"tokio_worker_overflow_count",
			"Number of times the given worker thread saturated its local queue.",
			&["worker"],
			*REGISTRY,
		)
		.unwrap();
	pub static ref TOKIO_WORKER_LOCAL_QUEUE_DEPTH: IntGaugeVec = register_int_gauge_vec_with_registry!(
			"tokio_worker_local_queue_depth",
			"Number of pending tasks in a worker's queue.",
			&["worker"],
			*REGISTRY,
		)
		.unwrap();
	pub static ref TOKIO_TASK_POLL_DURATION: Histogram = register_histogram_with_registry!(
		"tokio_task_poll_duration",
		"Duration to poll a task.",
		TASK_POLL_BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
}
