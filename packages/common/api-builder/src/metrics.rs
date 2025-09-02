use rivet_metrics::{
	BUCKETS,
	otel::{global::*, metrics::*},
};

lazy_static::lazy_static! {
	static ref METER: Meter = meter("rivet-api-builder");

	// TODO: Request body size
	// TODO: Response body size
	// Keys "method", "path"
	pub static ref API_REQUEST_PENDING: UpDownCounter<i64> = METER.i64_up_down_counter("rivet_api_request_pending")
			.with_description("Total number of requests in progress.")
			.build();

	/// Expected attributes: "method", "path"
	pub static ref API_REQUEST_TOTAL: Counter<u64> = METER.u64_counter("rivet_api_request_total")
		.with_description("Total number of requests.")
		.build();
	/// Expected attributes: "method", "path", "status", "error_code"
	pub static ref API_REQUEST_DURATION: Histogram<f64> = METER.f64_histogram("rivet_api_request_duration")
		.with_description("Duration of API requests.")
		.with_boundaries(BUCKETS.to_vec())
		.build();
	/// Expected attributes: "method", "path", "status", "error_code"
	pub static ref API_REQUEST_ERRORS: Counter<u64> = METER.u64_counter("rivet_api_request_errors")
		.with_description("All errors made to this request.")
		.build();
}
