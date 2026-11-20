//! Process-wide envoy-client metrics.
//!
//! These metrics live in the actor pod and are intended to make the
//! single-WebSocket transport between the actor and the engine debuggable
//! during incidents (e.g. pod-local handshake latency spikes where sent
//! requests get abandoned on disconnect).
//!
//! All labels here are bounded by code-defined enums. No `actor_id`,
//! `actor_key`, `runner_id`, `request_id`, or other unbounded values may be
//! added here.

use rivet_metrics::{BUCKETS, MICRO_BUCKETS, REGISTRY, START_BUCKETS, prometheus::*};

lazy_static::lazy_static! {
	pub static ref SQLITE_REQUEST_EXPIRED: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("rivetkit_envoy_client_sqlite_request_expired_total",
		"total VFS sqlite requests expired by cleanup (smoking-gun signal: was_sent=true means a request was abandoned without resolution)"),
		&["kind", "was_sent"],
		*REGISTRY
	)
	.unwrap();

	pub static ref SQLITE_REQUESTS_INFLIGHT: IntGauge = register_int_gauge_with_registry!(
		Opts::new("rivetkit_envoy_client_sqlite_requests_inflight",
		"current in-flight VFS sqlite requests tracked in envoy ctx"),
		*REGISTRY
	)
	.unwrap();

	pub static ref REMOTE_SQLITE_REQUESTS_INFLIGHT: IntGauge = register_int_gauge_with_registry!(
		Opts::new("rivetkit_envoy_client_remote_sqlite_requests_inflight",
		"current in-flight remote sqlite (exec/execute) requests tracked in envoy ctx"),
		*REGISTRY
	)
	.unwrap();

	pub static ref KV_REQUESTS_INFLIGHT: IntGauge = register_int_gauge_with_registry!(
		Opts::new("rivetkit_envoy_client_kv_requests_inflight",
		"current in-flight KV requests tracked in envoy ctx"),
		*REGISTRY
	)
	.unwrap();

	pub static ref ENVOY_LOOP_ITERATION_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_envoy_client_envoy_loop_iteration_duration_seconds",
			"duration of one envoy_loop select branch in seconds; long tails indicate the single fan-in loop stalled",
		)
		.buckets(MICRO_BUCKETS.to_vec()),
		&["branch"],
		*REGISTRY
	)
	.unwrap();

	pub static ref WS_TX_LOCK_WAIT_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_envoy_client_ws_tx_lock_wait_duration_seconds",
			"time spent waiting to acquire the ws_tx mutex in seconds",
		)
		.buckets(MICRO_BUCKETS.to_vec()),
		&["message_kind"],
		*REGISTRY
	)
	.unwrap();

	pub static ref WS_TX_LOCK_HOLD_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_envoy_client_ws_tx_lock_hold_duration_seconds",
			"time the ws_tx mutex guard was held in seconds (covers encode + send)",
		)
		.buckets(MICRO_BUCKETS.to_vec()),
		&["message_kind"],
		*REGISTRY
	)
	.unwrap();

	pub static ref SQLITE_REQUEST_TOTAL_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_envoy_client_sqlite_request_total_duration_seconds",
			"end-to-end duration from send_sqlite_request call to oneshot resolution in seconds",
		)
		.buckets(BUCKETS.to_vec()),
		&["kind"],
		*REGISTRY
	)
	.unwrap();

	pub static ref SQLITE_REQUEST_SUBMIT_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_envoy_client_sqlite_request_submit_duration_seconds",
			"time from send_sqlite_request entry to envoy_tx.send return in seconds",
		)
		.buckets(MICRO_BUCKETS.to_vec()),
		&["kind"],
		*REGISTRY
	)
	.unwrap();

	pub static ref SQLITE_REQUEST_WAIT_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"rivetkit_envoy_client_sqlite_request_wait_duration_seconds",
			"time from envoy_tx.send return to oneshot resolution in seconds",
		)
		.buckets(BUCKETS.to_vec()),
		&["kind"],
		*REGISTRY
	)
	.unwrap();

	pub static ref WS_RECONNECT: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("rivetkit_envoy_client_ws_reconnect_total",
		"total websocket reconnect events by reason"),
		&["reason"],
		*REGISTRY
	)
	.unwrap();

	pub static ref WS_PRE_CONNECT: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("rivetkit_envoy_client_ws_pre_connect_total",
		"total engine choices at connect by how the engine was chosen; retried_* results count retries after both connections reached the same engine"),
		&["result"],
		*REGISTRY
	)
	.unwrap();

	pub static ref WS_SESSION_DURATION_SECONDS: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"rivetkit_envoy_client_ws_session_duration_seconds",
			"duration of a single websocket session from connect to disconnect in seconds",
		)
		.buckets(BUCKETS.to_vec()),
		*REGISTRY
	)
	.unwrap();

	pub static ref ENVOY_TX_DEPTH: IntGauge = register_int_gauge_with_registry!(
		Opts::new("rivetkit_envoy_client_envoy_tx_depth",
		"current depth of the unbounded envoy_tx mpsc between WS read task and envoy_loop"),
		*REGISTRY
	)
	.unwrap();

	pub static ref HTTP_DETACHED_SENDS_INFLIGHT: IntGauge = register_int_gauge_with_registry!(
		Opts::new("rivetkit_envoy_client_http_detached_sends_inflight",
		"current detached tasks sending terminal HTTP frames while the HTTP stream is saturated"),
		*REGISTRY
	)
	.unwrap();

	pub static ref HTTP_DETACHED_SENDS_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new("rivetkit_envoy_client_http_detached_sends_capacity",
		"maximum detached terminal HTTP frame sends per WebSocket session"),
		*REGISTRY
	)
	.unwrap();

	pub static ref HTTP_DETACHED_SEND_DROPPED: IntCounter = register_int_counter_with_registry!(
		Opts::new("rivetkit_envoy_client_http_detached_send_dropped_total",
		"total terminal HTTP frames dropped because the detached send budget was exhausted"),
		*REGISTRY
	)
	.unwrap();

	pub static ref WS_READ_HANDLER_DURATION: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"rivetkit_envoy_client_ws_read_handler_duration",
			"time the websocket read loop spends decoding and dispatching one frame, during which the socket is not being drained",
		)
		.buckets(MICRO_BUCKETS.to_vec()),
		*REGISTRY
	)
	.unwrap();

	pub static ref WS_DECODE_DURATION: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"rivetkit_envoy_client_ws_decode_duration",
			"time to deserialize one inbound websocket frame",
		)
		.buckets(MICRO_BUCKETS.to_vec()),
		*REGISTRY
	)
	.unwrap();

	pub static ref WS_RX_BYTES: IntCounter = register_int_counter_with_registry!(
		Opts::new("rivetkit_envoy_client_ws_rx_bytes_total",
		"total bytes read from the engine websocket; the rate of this is how fast this pod consumes websocket data"),
		*REGISTRY
	)
	.unwrap();

	pub static ref WS_RX_MESSAGES: IntCounter = register_int_counter_with_registry!(
		Opts::new("rivetkit_envoy_client_ws_rx_messages_total",
		"total inbound websocket frames read from the engine"),
		*REGISTRY
	)
	.unwrap();

	pub static ref STOPPED_FLOORS: IntGauge = register_int_gauge_with_registry!(
		Opts::new("rivetkit_envoy_client_stopped_floors",
		"actors whose highest stopped generation this Envoy remembers"),
		*REGISTRY
	)
	.unwrap();

	pub static ref STOPPED_FLOORS_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new("rivetkit_envoy_client_stopped_floors_capacity",
		"configured cap on remembered stopped generation floors"),
		*REGISTRY
	)
	.unwrap();

	pub static ref ACTOR_START_SECONDS: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"rivetkit_envoy_client_actor_start_seconds",
			"time from receiving CommandStartActor to sending the actor's Running event, including the runtime's on_actor_start",
		)
		.buckets(START_BUCKETS.to_vec()),
		*REGISTRY
	)
	.unwrap();

	pub static ref ACTOR_QUEUED_REQUESTS: IntGauge = register_int_gauge_with_registry!(
		Opts::new("rivetkit_envoy_client_actor_queued_requests",
		"HTTP requests admitted to actors on this process and not yet taken by their actor tasks, including requests waiting for an actor to start"),
		*REGISTRY
	)
	.unwrap();

	pub static ref ACTOR_QUEUED_REQUESTS_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		Opts::new("rivetkit_envoy_client_actor_queued_requests_capacity",
		"configured cap on queued HTTP requests per actor"),
		*REGISTRY
	)
	.unwrap();

	pub static ref ACTOR_QUEUED_REQUESTS_OVERLOADED: IntCounter = register_int_counter_with_registry!(
		Opts::new("rivetkit_envoy_client_actor_queued_requests_overloaded_total",
		"HTTP requests refused because their actor already had the configured number queued"),
		*REGISTRY
	)
	.unwrap();
}
