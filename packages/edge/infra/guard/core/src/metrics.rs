use lazy_static::lazy_static;
use rivet_metrics::{prometheus::*, BUCKETS, REGISTRY};

lazy_static! {
	// MARK: Internal
	pub static ref ROUTE_CACHE_COUNT: IntGauge = register_int_gauge_with_registry!(
		"guard_route_cache_count",
		"Number of entries in the route cache",
		*REGISTRY,
	).unwrap();
	pub static ref RATE_LIMITER_COUNT: IntGauge = register_int_gauge_with_registry!(
		"guard_rate_limiter_count",
		"Number of active rate limiters",
		*REGISTRY,
	).unwrap();
	pub static ref IN_FLIGHT_COUNTER_COUNT: IntGauge = register_int_gauge_with_registry!(
		"guard_in_flight_counter_count",
		"Number of active in-flight counters",
		*REGISTRY,
	)
	.unwrap();

	// MARK: TCP
	pub static ref TCP_CONNECTION_TOTAL: IntCounter = register_int_counter_with_registry!(
		"guard_tcp_connection_total",
		"Total number of TCP connections ever",
		*REGISTRY,
	)
	.unwrap();
	pub static ref TCP_CONNECTION_PENDING: IntGauge = register_int_gauge_with_registry!(
		"guard_tcp_connection_pending",
		"Total number of open TCP connections",
		*REGISTRY,
	)
	.unwrap();
	pub static ref TCP_CONNECTION_DURATION: Histogram = register_histogram_with_registry!(
		"guard_tcp_connection_duration",
		"TCP connection duration in seconds",
		BUCKETS.to_vec(),
		*REGISTRY,
	)
	.unwrap();

	// MARK: Pre-proxy
	pub static ref RESOLVE_ROUTE_DURATION: Histogram = register_histogram_with_registry!(
		"guard_resolve_route_duration",
		"Time to resolve request route in seconds",
		BUCKETS.to_vec(),
		*REGISTRY,
	)
	.unwrap();

	// MARK: Proxy requests
	pub static ref PROXY_REQUEST_TOTAL: IntCounter = register_int_counter_with_registry!(
		"guard_proxy_request_total",
		"Total number of requests to actor",
		*REGISTRY,
	)
	.unwrap();
	pub static ref PROXY_REQUEST_PENDING: IntGauge = register_int_gauge_with_registry!(
		"guard_proxy_request_pending",
		"Number of pending requests to actor",
		*REGISTRY,
	)
	.unwrap();
	pub static ref PROXY_REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"guard_proxy_request_duration",
		"Request duration in seconds",
		&["status"],
		BUCKETS.to_vec(),
		*REGISTRY,
	)
	.unwrap();
	pub static ref PROXY_REQUEST_ERROR: IntCounterVec = register_int_counter_vec_with_registry!(
		"guard_proxy_request_errors_total",
		"Total number of errors when proxying requests to actor",
		&["error_type"],
		*REGISTRY,
	)
	.unwrap();
}
