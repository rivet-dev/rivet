use pegboard::metrics;

#[test]
fn envoy_expire_scheduler_metrics_accept_all_result_labels() {
	for result in [
		"scheduled",
		"deduped",
		"rejected_capacity",
		"expired",
		"skipped_fresh_or_already_expired",
		"error",
	] {
		metrics::ENVOY_EXPIRE_SCHEDULER_ENQUEUED_TOTAL
			.with_label_values(&["namespace", result])
			.inc();
		metrics::ENVOY_EXPIRE_SCHEDULER_COMPLETED_TOTAL
			.with_label_values(&["namespace", result])
			.inc();
	}

	metrics::ENVOY_EXPIRE_SCHEDULER_IN_FLIGHT.set(0);
	metrics::ENVOY_EXPIRE_SCHEDULER_PENDING.set(0);
	metrics::ENVOY_EXPIRE_SCHEDULER_DURATION
		.with_label_values(&["namespace"])
		.observe(0.0);
}

#[test]
fn envoy_load_balancer_metrics_accept_bounded_strategy_labels() {
	for strategy in ["random_ping_timestamp", "random_full_range", "hash"] {
		metrics::ENVOY_LB_ALLOCATION_TOTAL
			.with_label_values(&["namespace", "pool", strategy])
			.inc();
		metrics::ENVOY_LB_NO_ENVOY_AVAILABLE_TOTAL
			.with_label_values(&["namespace", "pool", strategy])
			.inc();
		metrics::ENVOY_LB_ALLOC_DURATION
			.with_label_values(&["namespace", "pool", strategy])
			.observe(0.0);
		metrics::ENVOY_LB_SCAN_DEPTH
			.with_label_values(&["namespace", "pool", strategy])
			.observe(0.0);
		metrics::ENVOY_LB_WRAP_TOTAL
			.with_label_values(&["namespace", "pool", strategy])
			.inc();
		metrics::ENVOY_LB_SCAN_CIRCUIT_BREAKER_TOTAL
			.with_label_values(&["namespace", "pool", strategy])
			.inc();
	}

	metrics::ENVOY_LB_SAMPLES_EFFECTIVE
		.with_label_values(&["namespace", "pool"])
		.observe(1.0);
	metrics::ENVOY_LB_SAMPLE_DEDUPE_TOTAL
		.with_label_values(&["namespace", "pool"])
		.inc();
	metrics::ENVOY_LB_TIED_MIN_TOTAL
		.with_label_values(&["namespace", "pool"])
		.inc();
}
