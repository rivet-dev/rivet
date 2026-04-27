use rivet_metrics::{MICRO_BUCKETS, REGISTRY, prometheus::*};

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

	pub static ref SQLITE_MIGRATION_REJECTED_JOURNAL_TOTAL: IntCounter = register_int_counter_with_registry!(
		"pegboard_sqlite_migration_rejected_journal_total",
		"Total number of v1 actors rejected from migration because a rollback journal sidecar was present (actor crashed during a write transaction).",
		*REGISTRY
	).unwrap();
}
