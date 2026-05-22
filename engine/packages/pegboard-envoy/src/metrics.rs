use rivet_metrics::{
	BUCKETS, BYTES_BUCKETS, LIFETIME_BUCKETS, MESSAGE_COUNT_BUCKETS, REGISTRY, prometheus::*,
};

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
		"Duration from sqlite commit frame arrival until depot dispatch.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref SQLITE_COMMIT_ENVOY_RESPONSE_DURATION: Histogram = register_histogram_with_registry!(
		"sqlite_commit_envoy_response_duration_seconds",
		"Duration from depot commit return until the websocket response frame is sent.",
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

	// MARK: Stop / eviction causality
	pub static ref ACTOR_STOP_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_actor_stop_total",
		"Count of actor stops observed by pegboard-envoy.",
		&["namespace_id", "pool_name", "reason", "code"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_LIFETIME_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_envoy_actor_lifetime_seconds",
		"Lifetime of actors from start to stop in seconds.",
		&["namespace_id", "pool_name", "reason"],
		LIFETIME_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref WS_CLOSE_INITIATOR_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_ws_close_initiator_total",
		"Count of websocket closes by the initiating party.",
		&["namespace_id", "pool_name", "initiator", "cause"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_LOST_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_lost_total",
		"Count of actors marked lost by origin.",
		&["namespace_id", "pool_name", "origin"],
		*REGISTRY
	).unwrap();

	pub static ref CONNECTION_CLOSE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_connection_close_total",
		"Count of envoy connection closes by ws close code and reason group.",
		&["namespace_id", "pool_name", "close_code", "reason_group"],
		*REGISTRY
	).unwrap();

	pub static ref EVICTION_WITH_REASON_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_eviction_with_reason_total",
		"Count of envoy connections evicted by reason.",
		&["namespace_id", "pool_name", "protocol_version", "reason"],
		*REGISTRY
	).unwrap();

	// MARK: WS traffic shape
	pub static ref WS_MESSAGES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_ws_messages_total",
		"Count of websocket messages by direction and kind.",
		&["namespace_id", "pool_name", "direction", "message_kind"],
		*REGISTRY
	).unwrap();

	pub static ref WS_BYTES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_ws_bytes_total",
		"Encoded websocket message byte volume by direction and kind.",
		&["namespace_id", "pool_name", "direction", "message_kind"],
		*REGISTRY
	).unwrap();

	pub static ref TUNNEL_MESSAGE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_tunnel_message_total",
		"Count of tunnel messages dispatched by direction and tunnel kind.",
		&["namespace_id", "pool_name", "direction", "tunnel_kind"],
		*REGISTRY
	).unwrap();

	pub static ref WS_FRAME_SIZE_BYTES: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_envoy_ws_frame_size_bytes",
		"Size of websocket frames in bytes.",
		&["namespace_id", "pool_name", "direction"],
		BYTES_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref ACK_LAG_MESSAGES: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_envoy_ack_lag_messages",
		"Lag in messages between sent and acked sequence numbers.",
		&["namespace_id", "pool_name", "direction"],
		MESSAGE_COUNT_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	// MARK: Nice-to-haves
	pub static ref ACTOR_STOP_TO_CLOSE_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_envoy_actor_stop_to_close_seconds",
		"Time from sending Stop to receiving Stopping/close in seconds.",
		&["namespace_id", "pool_name"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref PONG_MISSED_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_pong_missed_total",
		"Count of pongs that arrived after the slow threshold but before timeout.",
		&["namespace_id", "pool_name"],
		*REGISTRY
	).unwrap();
}

/// Bounded enum-style label values used across the metrics in this module. Listed here so
/// `prepopulate` can zero-initialize the canonical label combinations and expose them on a
/// cold `/metrics` scrape.
const STOP_REASONS: &[&str] = &[
	"sleep_intent",
	"stop_intent",
	"destroy",
	"going_away",
	"lost",
];

const STOP_CODES: &[&str] = &["ok", "error"];

const WS_CLOSE_INITIATORS: &[&str] = &[
	"engine_ping_timeout",
	"engine_eviction",
	"engine_shutdown",
	"client_close",
	"network",
];

const ACTOR_LOST_ORIGINS: &[&str] = &[
	"engine_workflow_liveness",
	"client_self_evict",
	"engine_command",
];

const CONNECTION_CLOSE_CODES: &[&str] = &["1000", "1001", "1006", "1008", "1011", "other"];

const EVICTION_REASONS: &[&str] = &[
	"duplicate_key",
	"protocol_mismatch",
	"version_drain",
	"shutdown",
];

const DIRECTIONS: &[&str] = &["inbound", "outbound"];

const TUNNEL_KINDS: &[&str] = &[
	"request_start",
	"request_chunk",
	"request_abort",
	"response_start",
	"response_chunk",
	"response_abort",
	"ws_open",
	"ws_message",
	"ws_message_ack",
	"ws_close",
];

/// Zero-initialize the canonical label combinations for the metrics defined in this module so
/// they are present on the first `/metrics` scrape. Called once via a `Once` from the
/// `PegboardEnvoyWs::new` constructor.
pub fn prepopulate() {
	// Existing metrics that already use label vectors are fine to leave to their first
	// observation; zero-initialize the new ones with empty-string values for unbounded labels
	// and one-of-each for bounded enums.
	for reason in STOP_REASONS {
		for code in STOP_CODES {
			ACTOR_STOP_TOTAL.with_label_values(&["", "", reason, code]);
		}
		ACTOR_LIFETIME_SECONDS.with_label_values(&["", "", reason]);
	}

	for initiator in WS_CLOSE_INITIATORS {
		WS_CLOSE_INITIATOR_TOTAL.with_label_values(&["", "", initiator, ""]);
	}

	for origin in ACTOR_LOST_ORIGINS {
		ACTOR_LOST_TOTAL.with_label_values(&["", "", origin]);
	}

	for code in CONNECTION_CLOSE_CODES {
		CONNECTION_CLOSE_TOTAL.with_label_values(&["", "", code, ""]);
	}

	for reason in EVICTION_REASONS {
		EVICTION_WITH_REASON_TOTAL.with_label_values(&["", "", "", reason]);
	}

	for direction in DIRECTIONS {
		WS_MESSAGES_TOTAL.with_label_values(&["", "", direction, ""]);
		WS_BYTES_TOTAL.with_label_values(&["", "", direction, ""]);
		WS_FRAME_SIZE_BYTES.with_label_values(&["", "", direction]);
		ACK_LAG_MESSAGES.with_label_values(&["", "", direction]);
		for tunnel_kind in TUNNEL_KINDS {
			TUNNEL_MESSAGE_TOTAL.with_label_values(&["", "", direction, tunnel_kind]);
		}
	}

	ACTOR_STOP_TO_CLOSE_SECONDS.with_label_values(&["", ""]);
	PONG_MISSED_TOTAL.with_label_values(&["", ""]);
}
