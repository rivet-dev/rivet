use lazy_static::lazy_static;
use rivet_metrics::{
	BUCKETS,
	otel::{global::*, metrics::*},
};

lazy_static! {
	static ref METER: Meter = meter("rivet-guard");

	// MARK: Internal
	/// Has no expected attributes
	pub static ref ROUTE_CACHE_COUNT: Gauge<u64> = METER.u64_gauge("rivet_guard_route_cache_count")
		.with_description("Number of entries in the route cache")
		.build();
	/// Has no expected attributes
	pub static ref RATE_LIMITER_COUNT: Gauge<u64> = METER.u64_gauge("rivet_guard_rate_limiter_count")
		.with_description("Number of active rate limiters")
		.build();
	/// Has no expected attributes
	pub static ref IN_FLIGHT_COUNTER_COUNT: Gauge<u64> = METER.u64_gauge("rivet_guard_in_flight_counter_count")
		.with_description("Number of active in-flight counters")
		.build();

	// MARK: TCP
	/// Has no expected attributes
	pub static ref TCP_CONNECTION_TOTAL: Counter<u64> = METER.u64_counter("rivet_guard_tcp_connection_total")
		.with_description("Total number of TCP connections ever")
		.build();
	/// Has no expected attributes
	pub static ref TCP_CONNECTION_PENDING: UpDownCounter<i64> = METER.i64_up_down_counter("rivet_guard_tcp_connection_pending")
		.with_description("Total number of open TCP connections")
		.build();
	/// Has no expected attributes
	pub static ref TCP_CONNECTION_DURATION: Histogram<f64> = METER.f64_histogram("rivet_guard_tcp_connection_duration")
		.with_description("TCP connection duration in seconds")
		.with_boundaries(BUCKETS.to_vec())
		.build();

	// MARK: Pre-proxy
	/// Has no expected attributes
	pub static ref RESOLVE_ROUTE_DURATION: Histogram<f64> = METER.f64_histogram("rivet_guard_resolve_route_duration")
		.with_description("Time to resolve request route in seconds")
		.with_boundaries(BUCKETS.to_vec())
		.build();

	// MARK: Proxy requests
	/// Has no expected attributes
	pub static ref PROXY_REQUEST_TOTAL: Counter<u64> = METER.u64_counter("rivet_guard_proxy_request_total")
		.with_description("Total number of requests to actor")
		.build();
	/// Has no expected attributes
	pub static ref PROXY_REQUEST_PENDING: UpDownCounter<i64> = METER.i64_up_down_counter("rivet_guard_proxy_request_pending")
		.with_description("Number of pending requests to actor")
		.build();
	/// Expected attributes: "status"
	pub static ref PROXY_REQUEST_DURATION: Histogram<f64> = METER.f64_histogram("rivet_guard_proxy_request_duration")
		.with_description("Request duration in seconds")
		.with_boundaries(BUCKETS.to_vec())
		.build();
	/// Expected attributes: "error_type"
	pub static ref PROXY_REQUEST_ERROR: Counter<u64> = METER.u64_counter("rivet_guard_proxy_request_errors_total")
		.with_description("Total number of errors when proxying requests to actor")
		.build();
}
