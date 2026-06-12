use universalpubsub::metrics;

#[test]
fn publish_attempt_metrics_accept_subject_root_labels() {
	let roots = [
		"pegboard.runner",
		"pegboard.runner.eviction-by-id",
		"pegboard.runner.eviction-by-name",
		"pegboard.gateway",
		"pegboard.envoy",
		"pegboard.envoy.eviction",
		"pegboard.serverless.outbound",
		"gasoline.worker.bump",
		"gasoline.workflow.created",
		"gasoline.workflow.complete",
		"gasoline.signal.for-workflow",
		"_inbox",
		"unknown",
	];

	for root in roots {
		metrics::PUBLISH_ATTEMPT_DURATION
			.with_label_values(&[root])
			.observe(0.0);
		metrics::PUBLISH_RETRY_TOTAL
			.with_label_values(&[root])
			.inc();
		metrics::NATS_SLOW_CONSUMER_TOTAL
			.with_label_values(&[root])
			.inc();
	}
}
