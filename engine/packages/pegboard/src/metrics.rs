use rivet_metrics::{BUCKETS, MICRO_BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref ACTOR_PENDING_ALLOCATION: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_actor_pending_allocation",
		"Total actors waiting for availability.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_actor_active",
		"Total actors currently allocated.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();

	pub static ref SERVERLESS_DESIRED_SLOTS: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_serverless_desired_slots",
		"Total amount of desired slots for serverless runners.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_ALLOCATE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_actor_allocate_duration",
		"Total duration to allocate an actor.",
		&["kind", "result"],
		MICRO_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_START_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_actor_start_duration",
		"Total duration from allocation to running.",
		&["namespace_id", "pool_name", "kind"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref RUNNER_VERSION_UPGRADE_DRAIN_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_runner_version_upgrade_drain_total",
		"Count of runners drained due to version upgrade.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_VERSION_UPGRADE_DRAIN_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_version_upgrade_drain_total",
		"Count of envoys drained due to version upgrade.",
		&["namespace_id", "pool_name"],
		*REGISTRY
	).unwrap();

	pub static ref SERVERLESS_OUTBOUND_REQ_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_serverless_outbound_req_total",
		"Count of serverless outbound requests made.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();

	pub static ref SERVERLESS_OUTBOUND_REQ_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_serverless_outbound_req_active",
		"Count of serverless outbound requests currently active.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_MIGRATION_ATTEMPTS_TOTAL: IntCounter = register_int_counter_with_registry!(
		"pegboard_sqlite_migration_attempts_total",
		"Total number of sqlite v1 to v2 migration attempts.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_MIGRATION_SUCCESSES_TOTAL: IntCounter = register_int_counter_with_registry!(
		"pegboard_sqlite_migration_successes_total",
		"Total number of sqlite v1 to v2 migrations that completed successfully.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_MIGRATION_FAILURES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_sqlite_migration_failures_total",
		"Total number of sqlite v1 to v2 migration failures by phase.",
		&["phase"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_MIGRATION_DURATION: Histogram = register_histogram_with_registry!(
		"pegboard_sqlite_migration_duration_seconds",
		"Duration of sqlite v1 to v2 migrations.",
		MICRO_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_MIGRATION_PAGES: Histogram = register_histogram_with_registry!(
		"pegboard_sqlite_migration_pages",
		"Number of pages imported during sqlite v1 to v2 migration.",
		MICRO_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_MIGRATION_ABANDONED_SIDECAR_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_sqlite_migration_abandoned_sidecar_total",
		"Total number of sidecars abandoned during sqlite v1 to v2 migration.",
		&["sidecar"],
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_EXPIRE_SCHEDULER_ENQUEUED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"envoy_expire_scheduler_enqueued_total",
		"Count of read-path envoy expire scheduler enqueue attempts.",
		&["namespace_id", "result"],
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_EXPIRE_SCHEDULER_COMPLETED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"envoy_expire_scheduler_completed_total",
		"Count of read-path envoy expire scheduler worker completions.",
		&["namespace_id", "result"],
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_EXPIRE_SCHEDULER_IN_FLIGHT: IntGauge = register_int_gauge_with_registry!(
		"envoy_expire_scheduler_in_flight",
		"Count of read-path envoy expire scheduler workers currently invoking expire.",
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_EXPIRE_SCHEDULER_PENDING: IntGauge = register_int_gauge_with_registry!(
		"envoy_expire_scheduler_pending",
		"Count of read-path envoy expire scheduler envoys currently queued or in flight.",
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_EXPIRE_SCHEDULER_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"envoy_expire_scheduler_duration_seconds",
		"Duration of read-path envoy expire scheduler workers, including semaphore wait time.",
		&["namespace_id"],
		MICRO_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_LB_ALLOCATION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"envoy_lb_allocation_total",
		"Count of successful envoy load-balancer allocations.",
		&["namespace_id", "pool_name", "strategy"],
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_LB_NO_ENVOY_AVAILABLE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"envoy_lb_no_envoy_available_total",
		"Count of envoy load-balancer allocation attempts with no envoy available.",
		&["namespace_id", "pool_name", "strategy"],
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_LB_ALLOC_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"envoy_lb_alloc_duration_seconds",
		"Duration of envoy load-balancer allocation attempts.",
		&["namespace_id", "pool_name", "strategy"],
		MICRO_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_LB_SCAN_DEPTH: HistogramVec = register_histogram_vec_with_registry!(
		"envoy_lb_scan_depth",
		"Count of envoy load-balancer index entries scanned per allocation attempt.",
		&["namespace_id", "pool_name", "strategy"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_LB_WRAP_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"envoy_lb_wrap_total",
		"Count of envoy load-balancer allocation scans that used the wrap path.",
		&["namespace_id", "pool_name", "strategy"],
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_LB_SAMPLES_EFFECTIVE: HistogramVec = register_histogram_vec_with_registry!(
		"envoy_lb_samples_effective",
		"Count of unique hash load-balancer candidates considered per allocation attempt.",
		&["namespace_id", "pool_name"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_LB_SAMPLE_DEDUPE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"envoy_lb_sample_dedupe_total",
		"Count of hash load-balancer samples deduped because they resolved to an existing candidate.",
		&["namespace_id", "pool_name"],
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_LB_TIED_MIN_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"envoy_lb_tied_min_total",
		"Count of hash load-balancer allocations that needed a random tiebreak among minimum-slot candidates.",
		&["namespace_id", "pool_name"],
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_LB_SCAN_CIRCUIT_BREAKER_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"envoy_lb_scan_circuit_breaker_total",
		"Count of hash load-balancer scans aborted after hitting the stale-entry circuit breaker.",
		&["namespace_id", "pool_name", "strategy"],
		*REGISTRY
	).unwrap();
}
