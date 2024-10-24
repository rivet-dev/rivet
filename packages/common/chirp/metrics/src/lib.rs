use rivet_metrics::{prometheus::*, BUCKETS, REGISTRY};

lazy_static::lazy_static! {
	// MARK: Chirp
	pub static ref CHIRP_REQUEST_PENDING: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"chirp_request_pending",
		"Total number of requests in progress.",
		&["context_name"],
		*REGISTRY,
	).unwrap();
	pub static ref CHIRP_REQUEST_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"chirp_request_total",
		"Total number of requests.",
		&["context_name"],
		*REGISTRY,
	).unwrap();
	pub static ref CHIRP_REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_request_duration",
		"Total duration of a request.",
		&["context_name", "error_code"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
	pub static ref CHIRP_MESSAGE_RECV_LAG: HistogramVec = register_histogram_vec_with_registry!(
		"chirp_message_recv_lag",
		"Time between the publish timestamp and the timestamp the message was received.",
		&["context_name"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
	pub static ref CHIRP_REQUEST_ERRORS: IntCounterVec = register_int_counter_vec_with_registry!(
		"chirp_request_errors",
		"All errors made to this request.",
		&["context_name", "error_code", "error_type"],
		*REGISTRY,
	).unwrap();
}
