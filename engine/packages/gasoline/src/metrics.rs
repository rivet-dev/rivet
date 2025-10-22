use rivet_metrics::{
	BUCKETS,
	otel::{global::*, metrics::*},
};

lazy_static::lazy_static! {
	static ref METER: Meter = meter("rivet-gasoline");
	/// Expected attributes: "worker_instance_id"
	pub static ref WORKER_LAST_PING: Gauge<u64> = METER.u64_gauge("rivet_gasoline_worker_last_ping")
		.with_description("Last ping of a worker instance as a unix ts.")
		.build();
	/// Expected attributes: "worker_instance_id"
	pub static ref LAST_PULL_WORKFLOWS_DURATION: Gauge<f64> = METER.f64_gauge("rivet_gasoline_last_pull_workflows_duration")
		.with_description("Last duration of pulling workflow data.")
		.build();
	/// Expected attributes: "worker_instance_id"
	pub static ref LAST_PULL_WORKFLOWS_HISTORY_DURATION: Gauge<u64> = METER.u64_gauge("rivet_gasoline_last_pull_workflows_history_duration")
		.with_description("Last duration of pulling workflow histories.")
		.build();
	/// Expected attributes: "worker_instance_id"
	pub static ref LAST_PULL_WORKFLOWS_FULL_DURATION: Gauge<f64> = METER.f64_gauge("rivet_gasoline_last_pull_workflows_full_duration")
		.with_description("Last duration of pulling workflow data and history.")
		.build();
	/// Expected attributes: "worker_instance_id"
	pub static ref PULL_WORKFLOWS_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_pull_workflows_duration")
		.with_description("Duration of pulling workflow data.")
		.with_boundaries(BUCKETS.to_vec())
		.build();
	/// Expected attributes: "worker_instance_id"
	pub static ref PULL_WORKFLOWS_HISTORY_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_pull_workflows_history_duration")
		.with_description("Duration of pulling workflow histories.")
		.with_boundaries(BUCKETS.to_vec())
		.build();
	/// Expected attributes: "worker_instance_id"
	pub static ref PULL_WORKFLOWS_FULL_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_pull_workflows_full_duration")
		.with_description("Duration of pulling workflow data and history.")
		.with_boundaries(BUCKETS.to_vec())
		.build();

	/// Expected attributes: "workflow_name"
	pub static ref FIND_WORKFLOWS_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_find_workflows_duration")
		.with_description("Duration to find a workflow with a given name and tags.")
		.with_boundaries(BUCKETS.to_vec())
		.build();

	/// Expected attributes: "workflow_name"
	pub static ref WORKFLOW_TOTAL: Gauge<u64> = METER.u64_gauge("rivet_gasoline_workflow_total")
		.with_description("Total workflows.")
		.build();
	/// Expected attributes: "workflow_name"
	pub static ref WORKFLOW_ACTIVE: Gauge<u64> = METER.u64_gauge("rivet_gasoline_workflow_active")
		.with_description("Total active workflows.")
		.build();
	/// Expected attributes: "workflow_name", "error_code"
	pub static ref WORKFLOW_DEAD: Gauge<u64> = METER.u64_gauge("rivet_gasoline_workflow_dead")
		.with_description("Total dead workflows.")
		.build();
	/// Expected attributes: "workflow_name"
	pub static ref WORKFLOW_SLEEPING: Gauge<u64> = METER.u64_gauge("rivet_gasoline_workflow_sleeping")
		.with_description("Total sleeping workflows.")
		.build();
	/// Expected attributes: "workflow_name", "error_code"
	pub static ref WORKFLOW_ERRORS: Counter<u64> = METER.u64_counter("rivet_gasoline_workflow_errors")
		.with_description("All errors made in a workflow.")
		.build();

	/// Expected attributes: "workflow_name"
	pub static ref COMPLETE_WORKFLOW_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_complete_workflow_duration")
		.with_description("Duration to complete a workflow with a given name.")
		.with_boundaries(BUCKETS.to_vec())
		.build();
	/// Expected attributes: "workflow_name"
	pub static ref COMMIT_WORKFLOW_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_commit_workflow_duration")
		.with_description("Duration to commit a workflow with a given name.")
		.with_boundaries(BUCKETS.to_vec())
		.build();

	/// Expected attributes: "workflow_name", "activity_name", "error_code"
	pub static ref ACTIVITY_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_activity_duration")
		.with_description("Total duration of an activity.")
		.with_boundaries(BUCKETS.to_vec())
		.build();
	/// Expected attributes: "workflow_name", "activity_name", "error_code"
	pub static ref ACTIVITY_ERRORS: Counter<u64> = METER.u64_counter("rivet_gasoline_activity_errors")
		.with_description("All errors made in an activity.")
		.build();

	/// Expected attributes: "signal_name"
	pub static ref SIGNAL_PENDING: Gauge<u64> = METER.u64_gauge("rivet_gasoline_signal_pending")
		.with_description("Total pending signals.")
		.build();
	/// Expected attributes: "workflow_name", "signal_name"
	pub static ref SIGNAL_RECV_LAG: Histogram<f64> = METER.f64_histogram("rivet_gasoline_signal_recv_lag")
		.with_description("Time between the publish timestamp and the timestamp the signal was received.")
		.with_boundaries(BUCKETS.to_vec())
		.build();
	/// Expected attributes: "workflow_name", "signal_name"
	pub static ref SIGNAL_PULL_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_signal_pull_duration")
		.with_description("Total duration to pull signals.")
		.with_boundaries(BUCKETS.to_vec())
		.build();
	/// Expected attributes: "workflow_name", "signal_name"
	pub static ref SIGNAL_PUBLISHED: Counter<u64> = METER.u64_counter("rivet_gasoline_signal_published")
		.with_description("Total published signals.")
		.build();
	/// Expected attributes: "workflow_name", "signal_name"
	pub static ref SIGNAL_SEND_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_signal_send_duration")
		.with_description("Total duration of a signal send.")
		.with_boundaries(BUCKETS.to_vec())
		.build();

	/// Expected attributes: "workflow_name", "message_name"
	pub static ref MESSAGE_PUBLISHED: Counter<u64> = METER.u64_counter("rivet_gasoline_message_published")
		.with_description("Total published messages.")
		.build();
	/// Expected attributes: "workflow_name", "message_name"
	pub static ref MESSAGE_SEND_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_message_send_duration")
		.with_description("Total duration of a message send.")
		.with_boundaries(BUCKETS.to_vec())
		.build();

	/// Expected attributes: "workflow_name", "sub_workflow_name"
	pub static ref WORKFLOW_DISPATCHED: Counter<u64> = METER.u64_counter("rivet_gasoline_workflow_dispatched")
		.with_description("Total dispatched workflows.")
		.build();
	/// Expected attributes: "workflow_name", "sub_workflow_name"
	pub static ref WORKFLOW_DISPATCH_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_workflow_dispatch_duration")
		.with_description("Total duration of a workflow dispatch.")
		.with_boundaries(BUCKETS.to_vec())
		.build();

	/// Expected attributes: "workflow_name"
	pub static ref LOOP_ITERATION_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_loop_iteration_duration")
		.with_description("Total duration of a single loop iteration (excluding its body).")
		.with_boundaries(BUCKETS.to_vec())
		.build();

	// MARK: Ops
	/// Expected attributes: "operation_name"
	pub static ref OPERATION_PENDING: UpDownCounter<i64> = METER.i64_up_down_counter("rivet_gasoline_operation_pending")
		.with_description("Total number of operation calls in progress.")
		.build();
	/// Expected attributes: "operation_name"
	pub static ref OPERATION_TOTAL: Counter<u64> = METER.u64_counter("rivet_gasoline_operation_total")
		.with_description("Total number of operation calls.")
		.build();
	/// Expected attributes: "operation_name", "error"
	pub static ref OPERATION_DURATION: Histogram<f64> = METER.f64_histogram("rivet_gasoline_operation_duration")
		.with_description("Total duration of an op call.")
		.with_boundaries(BUCKETS.to_vec())
		.build();
	/// Expected attributes: "operation_name", "error"
	pub static ref OPERATION_ERRORS: Counter<u64> = METER.u64_counter("rivet_gasoline_operation_errors")
		.with_description("All errors made by this operation.")
		.build();
}
