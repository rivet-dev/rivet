//! Gateway telemetry. The client WebSocket metrics share guard's prefix because they describe the
//! sockets guard accepted, next to `guard_request_seconds`.

use rivet_metrics::{REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	/// Keepalive ping frames sent on client WebSockets, by `result`: `ok`, or `error` when the
	/// send failed and ended the connection.
	pub(crate) static ref WEBSOCKET_KEEPALIVE_PING_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"guard_websocket_keepalive_ping_total",
		"Total number of keepalive ping frames sent on client WebSockets, by result: ok or error.",
		&["result"],
		*REGISTRY
	).unwrap();

	/// Gateway requests that named no namespace and resolved in `default` because their
	/// credential resolves none either, by `credential` kind.
	pub(crate) static ref GATEWAY_NAMESPACELESS_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"gateway_namespaceless_requests_total",
		"Total number of gateway requests that named no namespace and resolved in default, by credential kind.",
		&["credential"],
		*REGISTRY
	).unwrap();
}
