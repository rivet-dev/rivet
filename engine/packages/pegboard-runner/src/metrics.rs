use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref CONNECTION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_runner_connection_total",
		"Count of runner connections opened.",
		&["namespace_id", "runner_name", "protocol_version"],
		*REGISTRY
	).unwrap();

	pub static ref EVICTION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_runner_eviction_total",
		"Count of runner connections evicted.",
		&["namespace_id", "runner_name", "protocol_version"],
		*REGISTRY
	).unwrap();

	pub static ref CONNECTION_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_runner_connection_active",
		"Count of runner connections currently active.",
		&["namespace_id", "runner_name", "protocol_version"],
		*REGISTRY
	).unwrap();

	pub static ref RECEIVE_INIT_PACKET_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_runner_receive_init_packet_duration",
		"Duration to receive the init packet for a runner connection.",
		&["namespace_id", "runner_name"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref EVENT_MULTIPLEXER_COUNT: IntGauge = register_int_gauge_with_registry!(
		"pegboard_event_multiplexer_count",
		"Number of active actor event multiplexers.",
		*REGISTRY
	).unwrap();

	pub static ref INGESTED_EVENTS_TOTAL: IntCounter = register_int_counter_with_registry!(
		"pegboard_ingested_events_total",
		"Count of actor events.",
		*REGISTRY
	).unwrap();
}
