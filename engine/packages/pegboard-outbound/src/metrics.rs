use rivet_metrics::{BUCKETS, MICRO_BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref REQ_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_outbound_req_total",
		"Count of serverless outbound requests made.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();
	pub static ref REQ_BY_GENERATION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_outbound_req_by_generation_total",
		"Count of serverless outbound requests by actor generation bucket.",
		&["namespace_id", "runner_name", "generation_bucket"],
		*REGISTRY
	).unwrap();
	pub static ref REQ_ERROR_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_outbound_req_error_total",
		"Count of serverless outbound request failures by reason.",
		&["namespace_id", "pool_name", "error", "status"],
		*REGISTRY
	).unwrap();
	pub static ref REQ_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_outbound_req_duration_seconds",
		"Full serverless outbound request lifetime by terminal result.",
		&["namespace_id", "pool_name", "result", "drain_reason"],
		MICRO_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref REQ_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_outbound_req_active",
		"Count of serverless outbound requests currently active.",
		&["namespace_id", "runner_name"],
		*REGISTRY
	).unwrap();

	pub static ref REQ_SSE_OPEN_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_outbound_req_sse_open_duration",
		"Time from starting a serverless outbound request to the SSE stream opening in seconds.",
		&["namespace_id", "runner_name"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
}

pub fn prepopulate() {
	const ERRORS: &[&str] = &[
		"http_error",
		"connection_error",
		"stream_ended_early",
		"invalid_payload",
		"downgrade",
		"internal",
	];
	const STATUSES: &[&str] = &["429", "503", "5xx", "4xx", "2xx", "other", ""];
	const RESULTS: &[&str] = &[
		"success",
		"error_http_429",
		"error_http_503",
		"error_http_5xx",
		"error_http_4xx",
		"error_http_other",
		"error_connection",
		"error_stream_ended",
		"error_invalid_payload",
		"error_downgrade",
		"error_internal",
	];
	const DRAIN_REASONS: &[&str] = &[
		"lifespan_reached",
		"going_away",
		"actor_lost",
		"connection_lost",
		"term_signal",
		"",
	];

	for error in ERRORS {
		for status in STATUSES {
			REQ_ERROR_TOTAL
				.with_label_values(&["", "", error, status])
				.inc_by(0);
		}
	}

	for result in RESULTS {
		for drain_reason in DRAIN_REASONS {
			REQ_DURATION_SECONDS.with_label_values(&["", "", result, drain_reason]);
		}
	}
}
