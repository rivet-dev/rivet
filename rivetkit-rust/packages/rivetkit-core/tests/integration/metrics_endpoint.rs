#[test]
fn render_prometheus_metrics_includes_rivetkit_info() {
	rivetkit_core::metrics_endpoint::record_rivetkit_info(
		env!("CARGO_PKG_VERSION"),
		42,
		"serverful",
		"metrics-test-pool",
	);

	let metrics =
		rivetkit_core::metrics_endpoint::render_prometheus_metrics().expect("render metrics");
	let body = String::from_utf8(metrics.body).expect("metrics should be utf-8");
	let expected = format!(
		"rivetkit_info{{envoy_kind=\"serverful\",envoy_version=\"42\",pool_name=\"metrics-test-pool\",runtime=\"rivetkit\",type=\"local\",version=\"{}\"}} 1",
		env!("CARGO_PKG_VERSION")
	);

	assert!(
		body.lines().any(|line| line == expected),
		"missing version metric in:\n{body}"
	);
}
