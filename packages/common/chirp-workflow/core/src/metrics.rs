use rivet_metrics::{prometheus::*, BUCKETS, REGISTRY};

lazy_static::lazy_static! {
	pub static ref WORKER_LAST_PING: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"chirp_workflow_worker_last_ping",
		"Last ping of a worker instance as a unix ts.",
		&["worker_instance_id"],
		*REGISTRY,
	).unwrap();
	pub static ref LAST_PULL_WORKFLOWS_DURATION: GaugeVec = register_gauge_vec_with_registry!(
		"chirp_workflow_last_pull_workflows_duration",
		"Last duration of pulling workflow data.",
		&["worker_instance_id"],
		*REGISTRY,
	).unwrap();
	pub static ref LAST_PULL_WORKFLOWS_HISTORY_DURATION: GaugeVec = register_gauge_vec_with_registry!(
		"chirp_workflow_last_pull_workflows_history_duration",
		"Last duration of pulling workflow histories.",
		&["worker_instance_id"],
		*REGISTRY,
	).unwrap();
	pub static ref LAST_PULL_WORKFLOWS_FULL_DURATION: GaugeVec = register_gauge_vec_with_registry!(
		"chirp_workflow_last_pull_workflows_full_duration",
		"Last duration of pulling workflow data and history.",
		&["worker_instance_id"],
		*REGISTRY,
	).unwrap();
	pub static ref PULL_WORKFLOWS_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_workflow_pull_workflows_duration",
		"Duration of pulling workflow data.",
		&["worker_instance_id"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
	pub static ref PULL_WORKFLOWS_HISTORY_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_workflow_pull_workflows_history_duration",
		"Duration of pulling workflow histories.",
		&["worker_instance_id"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
	pub static ref PULL_WORKFLOWS_FULL_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_workflow_pull_workflows_full_duration",
		"Duration of pulling workflow data and history.",
		&["worker_instance_id"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();

	pub static ref FIND_WORKFLOW_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_workflow_find_workflows_duration",
		"Duration to find a workflow with a given name and tags.",
		&["workflow_name"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();

	pub static ref WORKFLOW_TOTAL: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"chirp_workflow_workflow_total",
		"Total workflows.",
		&["workflow_name"],
		*REGISTRY,
	).unwrap();
	pub static ref WORKFLOW_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"chirp_workflow_workflow_active",
		"Total active workflows.",
		&["workflow_name"],
		*REGISTRY,
	).unwrap();
	pub static ref WORKFLOW_DEAD: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"chirp_workflow_workflow_dead",
		"Total dead workflows.",
		&["workflow_name", "error_code"],
		*REGISTRY,
	).unwrap();
	pub static ref WORKFLOW_SLEEPING: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"chirp_workflow_workflow_sleeping",
		"Total sleeping workflows.",
		&["workflow_name"],
		*REGISTRY,
	).unwrap();
	pub static ref WORKFLOW_ERRORS: IntCounterVec = register_int_counter_vec_with_registry!(
		"chirp_workflow_workflow_errors",
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
	pub static ref SIGNAL_PUBLISHED: IntCounterVec = register_int_counter_vec_with_registry!(
		"chirp_workflow_signal_published",
		"Total published signals.",
		&["workflow_name", "signal_name"],
		*REGISTRY,
	).unwrap();
	pub static ref SIGNAL_SEND_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_workflow_signal_send_duration",
		"Total duration of a signal send.",
		&["workflow_name", "signal_name"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();

	pub static ref MESSAGE_PUBLISHED: IntCounterVec = register_int_counter_vec_with_registry!(
		"chirp_workflow_message_published",
		"Total published messages.",
		&["workflow_name", "message_name"],
		*REGISTRY,
	).unwrap();
	pub static ref MESSAGE_SEND_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_workflow_message_send_duration",
		"Total duration of a message send.",
		&["workflow_name", "message_name"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();

	pub static ref WORKFLOW_DISPATCHED: IntCounterVec = register_int_counter_vec_with_registry!(
		"chirp_workflow_workflow_dispatched",
		"Total dispatched workflows.",
		&["workflow_name", "sub_workflow_name"],
		*REGISTRY,
	).unwrap();
	pub static ref WORKFLOW_DISPATCH_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_workflow_workflow_dispatch_duration",
		"Total duration of an branch upsert.",
		&["workflow_name", "sub_workflow_name"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();

	pub static ref BRANCH_UPSERT_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_workflow_branch_upsert_duration",
		"Total duration of an branch upsert.",
		&["workflow_name"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();

	pub static ref LISTEN_WITH_TIMEOUT_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_workflow_listen_with_timeout_duration",
		"Total duration of a listen with timeout.",
		&["workflow_name"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
}
