use lazy_static::lazy_static;
use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static! {
	// MARK: Internal
	pub static ref ROUTE_CACHE_COUNT: IntGauge = register_int_gauge_with_registry!(
		"guard_route_cache_count",
		"Number of entries in the route cache",
		*REGISTRY
	).unwrap();
	pub static ref RATE_LIMITER_COUNT: IntGauge = register_int_gauge_with_registry!(
		"guard_rate_limiter_count",
		"Number of active rate limiters",
		*REGISTRY
	).unwrap();
	pub static ref IN_FLIGHT_COUNTER_COUNT: IntGauge = register_int_gauge_with_registry!(
		"guard_in_flight_counter_count",
		"Number of active in-flight counters",
		*REGISTRY
	).unwrap();
	pub static ref IN_FLIGHT_REQUEST_COUNT: IntGauge = register_int_gauge_with_registry!(
		"guard_in_flight_request_count",
		"Number of active in-flight requests",
		*REGISTRY
	).unwrap();

	// MARK: TCP
	pub static ref TCP_CONNECTION_TOTAL: IntCounter = register_int_counter_with_registry!(
		"guard_tcp_connection_total",
		"Total number of TCP connections ever",
		*REGISTRY
	).unwrap();
	pub static ref TCP_CONNECTION_PENDING: IntGauge = register_int_gauge_with_registry!(
		"guard_tcp_connection_pending",
		"Total number of open TCP connections",
		*REGISTRY
	).unwrap();
	pub static ref TCP_CONNECTION_DURATION: Histogram = register_histogram_with_registry!(
		"guard_tcp_connection_duration",
		"TCP connection duration in seconds",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	// MARK: Pre-proxy
	pub static ref RESOLVE_ROUTE_DURATION: Histogram = register_histogram_with_registry!(
		"guard_resolve_route_duration",
		"Time to resolve request route in seconds",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	// MARK: Proxy requests
	pub static ref PROXY_REQUEST_TOTAL: IntCounter = register_int_counter_with_registry!(
		"guard_proxy_request_total",
		"Total number of requests to actor",
		*REGISTRY
	).unwrap();
	pub static ref PROXY_REQUEST_PENDING: IntGauge = register_int_gauge_with_registry!(
		"guard_proxy_request_pending",
		"Number of pending requests to actor",
		*REGISTRY
	).unwrap();
	pub static ref PROXY_REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"guard_proxy_request_duration",
		"Request duration in seconds",
		&["status"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref PROXY_REQUEST_ERROR_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"guard_proxy_request_errors_total",
		"Total number of errors when proxying requests to actor",
		&["error"],
		*REGISTRY
	).unwrap();

	// MARK: WebSockets
	pub static ref WEBSOCKET_SEND_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"guard_websocket_send_duration",
		"Time to send a WebSocket message through a shared WebSocketHandle in seconds.",
		&["message_kind"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref WEBSOCKET_SEND_LOCK_WAIT_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"guard_core_websocket_send_lock_wait_duration_seconds",
		"Time spent awaiting the per-connection ws_tx mutex inside WebSocketHandle::send. High tails indicate contention from other senders on the same connection.",
		&["message_kind"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref WEBSOCKET_SEND_WRITE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"guard_core_websocket_send_write_duration_seconds",
		"Time spent inside the network write (lock held) of WebSocketHandle::send.",
		&["message_kind"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref WEBSOCKET_WRITE_WOULD_BLOCK_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"guard_websocket_write_would_block_total",
		"Total number of WebSocket write or flush attempts that hit WouldBlock.",
		&["message_kind"],
		*REGISTRY
	).unwrap();
	pub static ref WEBSOCKET_WRITE_BUFFER_FULL_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"guard_websocket_write_buffer_full_total",
		"Total number of WebSocket messages rejected because the tungstenite write buffer was full.",
		&["message_kind"],
		*REGISTRY
	).unwrap();
	pub static ref WEBSOCKET_WRITE_BACKPRESSURE_EVENTS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"guard_websocket_write_backpressure_events_total",
		"Total number of transitions from write-ready to write-backpressured.",
		&["message_kind"],
		*REGISTRY
	).unwrap();
}
