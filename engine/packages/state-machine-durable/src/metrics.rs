//! Durable runner telemetry. `machine` is a state machine's `DEBUG_NAME`, so every label is
//! bounded.

use rivet_metrics::{REGISTRY, START_BUCKETS, prometheus::*};

lazy_static::lazy_static! {
	/// Time an event waited for the machine's partition lock, which serializes its events.
	pub static ref LOCK_WAIT_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"state_machine_durable_lock_wait_seconds",
			"Time a durable state machine event waited for the machine's partition lock.",
		)
		.buckets(START_BUCKETS.to_vec()),
		&["machine"],
		*REGISTRY
	).unwrap();

	/// Time the volume commit of one durable transition took, one quorum append when nothing
	/// else is queued on the volume.
	pub static ref COMMIT_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"state_machine_durable_commit_seconds",
			"Time the volume commit of one durable state machine transition took.",
		)
		.buckets(START_BUCKETS.to_vec()),
		&["machine"],
		*REGISTRY
	).unwrap();

	/// Events whose transitions changed nothing durable, so the runner skipped their commit.
	pub static ref SKIPPED_COMMITS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"state_machine_durable_skipped_commits_total",
			"Durable state machine events that changed nothing durable and skipped their commit.",
		),
		&["machine"],
		*REGISTRY
	).unwrap();

	/// Events applied in one durable commit.
	pub static ref EVENTS_PER_COMMIT: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"state_machine_durable_events_per_commit",
			"Events of one durable state machine applied in one commit.",
		)
		.buckets(vec![1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 16.0, 32.0, 64.0]),
		&["machine"],
		*REGISTRY
	).unwrap();
}
