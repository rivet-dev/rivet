use rivet_metrics::{prometheus::*, BUCKETS, REGISTRY};

lazy_static::lazy_static! {
	pub static ref WORKER_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"chirp_workflow_worker_active",
		"Total active workers.",
		&[],
		*REGISTRY,
	).unwrap();
	pub static ref PULL_WORKFLOWS_DURATION: GaugeVec = register_gauge_vec_with_registry!(
		"chirp_pull_workflows_duration",
		"Duration of just pulling workflow rows.",
		&["worker_instance_id"],
		*REGISTRY,
	).unwrap();
	pub static ref PULL_WORKFLOWS_HISTORY_DURATION: GaugeVec = register_gauge_vec_with_registry!(
		"chirp_pull_workflows_history_duration",
		"Duration of pulling workflow histories.",
		&["worker_instance_id"],
		*REGISTRY,
	).unwrap();

	pub static ref WORKFLOW_TOTAL: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"chirp_workflow_total",
		"Total workflows.",
		&["workflow_name"],
		*REGISTRY,
	).unwrap();
	pub static ref WORKFLOW_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"chirp_workflow_active",
		"Total active workflows.",
		&["workflow_name"],
		*REGISTRY,
	).unwrap();
	pub static ref WORKFLOW_DEAD: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"chirp_workflow_dead",
		"Total dead workflows.",
		&["workflow_name", "error_code"],
		*REGISTRY,
	).unwrap();
	pub static ref WORKFLOW_SLEEPING: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"chirp_workflow_sleeping",
		"Total sleeping workflows.",
		&["workflow_name"],
		*REGISTRY,
	).unwrap();
	pub static ref WORKFLOW_ERRORS: IntCounterVec = register_int_counter_vec_with_registry!(
		"chirp_workflow_errors",
		"All errors made in a workflow.",
		&["workflow_name", "error_code"],
		*REGISTRY,
	).unwrap();

	pub static ref ACTIVITY_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_workflow_activity_duration",
		"Total duration of an activity.",
		&["workflow_name", "activity_name", "error_code"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
	pub static ref ACTIVITY_ERRORS: IntCounterVec = register_int_counter_vec_with_registry!(
		"chirp_workflow_activity_errors",
		"All errors made in an activity.",
		&["workflow_name", "activity_name", "error_code"],
		*REGISTRY,
	).unwrap();

	pub static ref SIGNAL_PENDING: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"chirp_workflow_signal_pending",
		"Total pending signals.",
		&["signal_name"],
		*REGISTRY,
	).unwrap();
	pub static ref SIGNAL_RECV_LAG: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_workflow_signal_recv_lag",
		"Time between the publish timestamp and the timestamp the signal was received.",
		&["workflow_name", "signal_name"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();

	pub static ref SIGNAL_PULL_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_workflow_signal_pull_duration",
		"Total duration to pull signals.",
		&["workflow_name", "signal_name"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
}
