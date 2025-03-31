use lazy_static::lazy_static;
use rivet_metrics::prometheus::*;

lazy_static! {
	pub static ref ACTOR_REQUEST_TOTAL: IntCounterVec = register_int_counter_vec!(
		"actor_request_total",
		"Total number of requests to actor",
		&["actor_id", "server_id", "method", "path"]
	)
	.unwrap();
	pub static ref ACTOR_REQUEST_PENDING: IntGaugeVec = register_int_gauge_vec!(
		"actor_request_pending",
		"Number of pending requests to actor",
		&["actor_id", "server_id", "method", "path"]
	)
	.unwrap();
	pub static ref ACTOR_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
		"actor_request_duration_seconds",
		"Request duration in seconds",
		&["actor_id", "server_id", "status"]
	)
	.unwrap();
	pub static ref ACTOR_REQUEST_ERRORS: IntCounterVec = register_int_counter_vec!(
		"actor_request_errors_total",
		"Total number of errors when proxying requests to actor",
		&["actor_id", "server_id", "error_type"]
	)
	.unwrap();
}
