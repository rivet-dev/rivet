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

#[test]
fn envoy_state_metric_transitions_one_hot() {
	let namespace_id = "state-test-namespace";
	let pool_name = "state-test-pool";
	let protocol_version = "state-test-protocol";

	metrics::set_envoy_connection_state(
		namespace_id,
		pool_name,
		protocol_version,
		None,
		Some(metrics::EnvoyState::Starting),
		"websocket_accepted",
	);
	assert_eq!(
		1,
		metrics::ENVOY_CONNECTIONS_BY_STATE
			.with_label_values(&[
				namespace_id,
				pool_name,
				protocol_version,
				metrics::EnvoyState::Starting.as_str(),
			])
			.get()
	);

	metrics::transition_envoy_connection_state(
		namespace_id,
		pool_name,
		protocol_version,
		metrics::EnvoyState::Starting,
		metrics::EnvoyState::Connected,
		"init_complete",
	);
	assert_eq!(
		0,
		metrics::ENVOY_CONNECTIONS_BY_STATE
			.with_label_values(&[
				namespace_id,
				pool_name,
				protocol_version,
				metrics::EnvoyState::Starting.as_str(),
			])
			.get()
	);
	assert_eq!(
		1,
		metrics::ENVOY_CONNECTIONS_BY_STATE
			.with_label_values(&[
				namespace_id,
				pool_name,
				protocol_version,
				metrics::EnvoyState::Connected.as_str(),
			])
			.get()
	);

	metrics::set_envoy_connection_state(
		namespace_id,
		pool_name,
		protocol_version,
		Some(metrics::EnvoyState::Connected),
		None,
		"websocket_closed",
	);
}
