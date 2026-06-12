use pegboard_outbound::metrics;

#[test]
fn outbound_generation_metric_accepts_all_bucket_labels() {
	let buckets = ["0", "1", "2", "3-5", "6-10", "11+"];

	for bucket in buckets {
		metrics::REQ_BY_GENERATION_TOTAL
			.with_label_values(&["namespace", "runner", bucket])
			.inc();
	}
}

#[test]
fn outbound_error_and_duration_metrics_accept_all_labels() {
	let errors = [
		"http_error",
		"connection_error",
		"stream_ended_early",
		"invalid_payload",
		"downgrade",
		"internal",
	];
	let statuses = ["429", "503", "5xx", "4xx", "2xx", "other", ""];
	let results = [
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
	let drain_reasons = [
		"lifespan_reached",
		"going_away",
		"actor_lost",
		"connection_lost",
		"term_signal",
		"",
	];

	for error in errors {
		for status in statuses {
			metrics::REQ_ERROR_TOTAL
				.with_label_values(&["namespace", "pool", error, status])
				.inc();
		}
	}

	for result in results {
		for drain_reason in drain_reasons {
			metrics::REQ_DURATION_SECONDS
				.with_label_values(&["namespace", "pool", result, drain_reason])
				.observe(0.0);
		}
	}
}
