use epoxy_protocol::protocol;
use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	// MARK: Proposals
	pub static ref PROPOSAL_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"epoxy_proposals_total",
		"Total number of per-key proposal outcomes.",
		&["result"],
		*REGISTRY
	).unwrap();

	pub static ref PROPOSAL_DURATION: Histogram = register_histogram_with_registry!(
		"epoxy_proposal_duration",
		"Duration from propose to commit in seconds.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref FAST_PATH_TOTAL: IntCounter = register_int_counter_with_registry!(
		"epoxy_fast_path_total",
		"Total number of fast-path proposal attempts.",
		*REGISTRY
	).unwrap();

	pub static ref SLOW_PATH_TOTAL: IntCounter = register_int_counter_with_registry!(
		"epoxy_slow_path_total",
		"Total number of slow-path proposal attempts.",
		*REGISTRY
	).unwrap();

	pub static ref PREPARE_TOTAL: IntCounter = register_int_counter_with_registry!(
		"epoxy_prepare_total",
		"Total number of recovery Prepare phases triggered.",
		*REGISTRY
	).unwrap();

	pub static ref PREPARE_RETRY_TOTAL: IntCounter = register_int_counter_with_registry!(
		"epoxy_prepare_retry_total",
		"Total number of Prepare retries caused by contention.",
		*REGISTRY
	).unwrap();

	pub static ref BALLOT_BUMP_TOTAL: IntCounter = register_int_counter_with_registry!(
		"epoxy_ballot_bump_total",
		"Total number of proposal ballot bumps caused by contention.",
		*REGISTRY
	).unwrap();

	// MARK: Changelog
	pub static ref CHANGELOG_SIZE: IntGauge = register_int_gauge_with_registry!(
		"epoxy_changelog_size",
		"Current number of changelog entries on the local replica.",
		*REGISTRY
	).unwrap();

	// MARK: HTTP request-level
	pub static ref REQUEST_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"epoxy_requests_total",
		"Total number of replica HTTP requests.",
		&["request_type", "result"],
		*REGISTRY
	).unwrap();

	pub static ref REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"epoxy_request_duration",
		"Duration of replica HTTP request handling in seconds.",
		&["request_type"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	// MARK: Cluster state
	pub static ref REPLICAS_TOTAL: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"epoxy_replicas_total",
		"Total number of replicas by status.",
		&["status"],
		*REGISTRY
	).unwrap();
}

pub fn record_proposal_result(result: &str) {
	PROPOSAL_TOTAL.with_label_values(&[result]).inc();
}

pub fn record_ballot_bump() {
	BALLOT_BUMP_TOTAL.inc();
}

pub fn record_prepare_retry() {
	PREPARE_RETRY_TOTAL.inc();
}

pub fn record_changelog_append() {
	CHANGELOG_SIZE.inc();
}

pub fn record_request(request_type: &str, result: &str, duration: std::time::Duration) {
	REQUEST_TOTAL.with_label_values(&[request_type, result]).inc();
	REQUEST_DURATION
		.with_label_values(&[request_type])
		.observe(duration.as_secs_f64());
}

pub fn record_replicas(config: &protocol::ClusterConfig) {
	REPLICAS_TOTAL.reset();
	for replica in &config.replicas {
		REPLICAS_TOTAL
			.with_label_values(&[match replica.status {
				protocol::ReplicaStatus::Active => "active",
				protocol::ReplicaStatus::Learning => "learning",
				protocol::ReplicaStatus::Joining => "joining",
			}])
			.inc();
	}
}
