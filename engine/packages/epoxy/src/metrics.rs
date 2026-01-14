use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	// MARK: Consensus Operations
	pub static ref PROPOSALS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"epoxy_proposals_total",
		"Total number of proposals.",
		&["status"],
		*REGISTRY
	).unwrap();

	pub static ref PRE_ACCEPT_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"epoxy_pre_accept_total",
		"Total number of pre-accept operations.",
		&["result"],
		*REGISTRY
	).unwrap();

	pub static ref ACCEPT_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"epoxy_accept_total",
		"Total number of accept operations.",
		&["result"],
		*REGISTRY
	).unwrap();

	pub static ref COMMIT_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"epoxy_commit_total",
		"Total number of commit operations.",
		&["result"],
		*REGISTRY
	).unwrap();

	pub static ref PROPOSAL_DURATION: Histogram = register_histogram_with_registry!(
		"epoxy_proposal_duration",
		"Duration from propose to commit in seconds.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	// MARK: Message Handling
	pub static ref REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"epoxy_requests_total",
		"Total number of requests.",
		&["request_type", "result"],
		*REGISTRY
	).unwrap();

	pub static ref REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"epoxy_request_duration",
		"Duration of request handling in seconds.",
		&["request_type"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	// MARK: Quorum
	pub static ref QUORUM_ATTEMPTS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"epoxy_quorum_attempts_total",
		"Total number of quorum attempts.",
		&["quorum_type", "result"],
		*REGISTRY
	).unwrap();

	pub static ref QUORUM_SIZE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"epoxy_quorum_size",
		"Current quorum size.",
		&["quorum_type"],
		*REGISTRY
	).unwrap();

	// MARK: Replica State
	pub static ref BALLOT_EPOCH: IntGauge = register_int_gauge_with_registry!(
		"epoxy_ballot_epoch",
		"Current ballot epoch.",
		*REGISTRY
	).unwrap();

	pub static ref BALLOT_NUMBER: IntGauge = register_int_gauge_with_registry!(
		"epoxy_ballot_number",
		"Current ballot number.",
		*REGISTRY
	).unwrap();

	pub static ref INSTANCE_NUMBER: IntGauge = register_int_gauge_with_registry!(
		"epoxy_instance_number",
		"Current instance/slot number.",
		*REGISTRY
	).unwrap();

	// MARK: Cluster Health
	pub static ref REPLICAS_TOTAL: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"epoxy_replicas_total",
		"Total number of replicas.",
		&["status"],
		*REGISTRY
	).unwrap();

	// MARK: Errors
	pub static ref BALLOT_REJECTIONS_TOTAL: IntCounter = register_int_counter_with_registry!(
		"epoxy_ballot_rejections_total",
		"Total number of ballot rejections due to stale ballot.",
		*REGISTRY
	).unwrap();

	pub static ref INTERFERENCE_DETECTED_TOTAL: IntCounter = register_int_counter_with_registry!(
		"epoxy_interference_detected_total",
		"Total number of key conflict interferences detected.",
		*REGISTRY
	).unwrap();
}
