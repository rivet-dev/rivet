//! Gateway metrics of a migration from V2. Labels are bounded: the namespace name and the
//! route.
use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref REQUESTS: IntCounterVec = register_int_counter_vec_with_registry!(
		"gateway_migrate_from_v2_requests_total",
		"Gateway actor requests while a V2 namespace migrates in, by where they were served: v3 (including actors moved in from V2), v2_fallback, v2_error (routed to V2 but V2 was unreachable or refused), retry_later (retryable refusals sent to the client) or not_found.",
		&["namespace", "route"],
		*REGISTRY
	).unwrap();

	pub static ref CREATE_RATIO: GaugeVec = register_gauge_vec_with_registry!(
		"gateway_migrate_from_v2_create_ratio",
		"The configured share of new creates that land in this cluster (migrate_from_v2.create_ratio), as this process last read it.",
		&["namespace"],
		*REGISTRY
	).unwrap();

	pub static ref FALLBACK_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"gateway_migrate_from_v2_fallback_duration_seconds",
		"Time until V2 answered a proxied request's headers or WebSocket handshake, in seconds.",
		&["namespace"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
}
