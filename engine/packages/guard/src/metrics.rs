//! Guard request telemetry. `kind` is `http` or `websocket_upgrade`.

use rivet_metrics::{REGISTRY, START_BUCKETS, prometheus::*};

lazy_static::lazy_static! {
	/// Time from accepting a request to its response headers, or to the upgrade response for a
	/// WebSocket. A WebSocket's lifetime after the upgrade is not included.
	pub static ref REQUEST_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		"guard_request_seconds",
		"Time from guard accepting a request to its response, or to the upgrade response for a WebSocket, by kind: http or websocket_upgrade.",
		&["kind"],
		START_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
}
