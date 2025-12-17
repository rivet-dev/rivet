use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	// TODO: Request body size
	// TODO: Response body size
	pub static ref API_REQUEST_PENDING: IntGaugeVec =
		register_int_gauge_vec_with_registry!(
			"api_request_pending",
			"Total number of requests in progress.",
			&["method", "path"],
			*REGISTRY
		).unwrap();
	pub static ref API_REQUEST_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"api_request_total",
		"Total number of requests.",
		&["method", "path"],
		*REGISTRY
	).unwrap();
	pub static ref API_REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"api_request_duration",
		"Duration of API requests.",
		&["method", "path", "status", "error_code"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref API_REQUEST_ERRORS: IntCounterVec = register_int_counter_vec_with_registry!(
		"api_request_errors_total",
		"All errors made to this request.",
		&["method", "path", "status", "error_code"],
		*REGISTRY,
	)
	.unwrap();
}
