use pegboard_envoy::metrics;

#[test]
fn envoy_connection_metrics_accept_labels() {
	metrics::ENVOY_CONNECTED
		.with_label_values(&["namespace", "pool"])
		.set(1);
	metrics::ENVOY_LIFETIME_SECONDS
		.with_label_values(&["namespace", "pool"])
		.observe(1.0);
	metrics::ENVOY_PING_LAG_SECONDS
		.with_label_values(&["namespace", "pool"])
		.observe(0.1);
}
