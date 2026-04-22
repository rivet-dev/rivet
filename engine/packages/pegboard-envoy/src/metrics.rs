use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref CONNECTION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_connection_total",
		"Count of envoy connections opened.",
		&["namespace_id", "pool_name", "protocol_version"],
		*REGISTRY
	).unwrap();

	pub static ref EVICTION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_eviction_total",
		"Count of envoy connections evicted.",
		&["namespace_id", "pool_name", "protocol_version"],
		*REGISTRY
	).unwrap();

	pub static ref CONNECTION_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_envoy_connection_active",
		"Count of envoy connections currently active.",
		&["namespace_id", "pool_name", "protocol_version"],
		*REGISTRY
	).unwrap();

	pub static ref RECEIVE_INIT_PACKET_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_envoy_receive_init_packet_duration",
		"Duration to receive the init packet for a envoy connection.",
		&["namespace_id", "pool_name"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref EVENT_MULTIPLEXER_COUNT: IntGauge = register_int_gauge_with_registry!(
		"pegboard_envoy_event_multiplexer_count",
		"Number of active actor event multiplexers.",
		*REGISTRY
	).unwrap();

	pub static ref INGESTED_EVENTS_TOTAL: IntCounter = register_int_counter_with_registry!(
		"pegboard_envoy_ingested_events_total",
		"Count of actor events.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMMIT_ENVOY_DISPATCH_DURATION: Histogram = register_histogram_with_registry!(
		"sqlite_commit_envoy_dispatch_duration_seconds",
		"Duration from sqlite commit frame arrival until sqlite-storage dispatch.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMMIT_ENVOY_RESPONSE_DURATION: Histogram = register_histogram_with_registry!(
		"sqlite_commit_envoy_response_duration_seconds",
		"Duration from sqlite-storage commit return until the websocket response frame is sent.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_MIGRATION_ATTEMPTS_TOTAL: IntCounter = register_int_counter_with_registry!(
		"pegboard_envoy_sqlite_migration_attempts_total",
		"Total number of sqlite v1 to v2 migration attempts.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_MIGRATION_SUCCESSES_TOTAL: IntCounter = register_int_counter_with_registry!(
		"pegboard_envoy_sqlite_migration_successes_total",
		"Total number of sqlite v1 to v2 migrations that completed successfully.",
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_MIGRATION_FAILURES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_sqlite_migration_failures_total",
		"Total number of sqlite v1 to v2 migration failures by phase.",
		&["phase"],
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_MIGRATION_DURATION: Histogram = register_histogram_with_registry!(
		"pegboard_envoy_sqlite_migration_duration_seconds",
		"Duration of sqlite v1 to v2 migrations.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_MIGRATION_PAGES: Histogram = register_histogram_with_registry!(
		"pegboard_envoy_sqlite_migration_pages",
		"Number of pages imported during sqlite v1 to v2 migration.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
}
