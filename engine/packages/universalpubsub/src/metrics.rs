use rivet_metrics::{REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref SUBSCRIBER_COUNT: IntCounterVec = register_int_counter_vec_with_registry!(
		"ups_subscriber_count",
		"Total number of subscribers ever created by subject.",
		&["subject"],
		*REGISTRY
	).unwrap();
	pub static ref ACTIVE_SUBSCRIBER_COUNT: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"ups_active_subscriber_count",
		"Number of active subscribers by subject.",
		&["subject"],
		*REGISTRY
	).unwrap();
	// PubSub layer metrics
	pub static ref LOCAL_SUBSCRIBER_COUNT: IntGauge = register_int_gauge_with_registry!(
		"ups_local_subscriber_count",
		"Number of local in-memory subscriber entries.",
		*REGISTRY
	).unwrap();
	pub static ref REPLY_SUBSCRIBER_COUNT: IntGauge = register_int_gauge_with_registry!(
		"ups_reply_subscriber_count",
		"Number of pending reply subscriber entries.",
		*REGISTRY
	).unwrap();
	// Memory driver metrics
	pub static ref MEMORY_SUBSCRIBER_COUNT: IntGauge = register_int_gauge_with_registry!(
		"ups_memory_subscriber_count",
		"Number of subject entries in the memory driver.",
		*REGISTRY
	).unwrap();
	// Postgres driver metrics
	pub static ref POSTGRES_SUBSCRIPTION_COUNT: IntGauge = register_int_gauge_with_registry!(
		"ups_postgres_subscription_count",
		"Number of subscription entries in the postgres driver.",
		*REGISTRY
	).unwrap();

	// Message metrics
	pub static ref MESSAGE_RECV_COUNT: IntCounterVec = register_int_counter_vec_with_registry!(
		"ups_message_recv_count",
		"Total number of messages ever received by subject.",
		&["subject"],
		*REGISTRY
	).unwrap();
	pub static ref MESSAGE_SEND_COUNT: IntCounterVec = register_int_counter_vec_with_registry!(
		"ups_message_send_count",
		"Total number of messages ever sent by subject.",
		&["kind", "subject"],
		*REGISTRY
	).unwrap();
	pub static ref BYTES_PER_MESSAGE: HistogramVec = register_histogram_vec_with_registry!(
		"ups_bytes_per_message",
		"Amount of bytes per message received.",
		&["subject"],
		vec![16.0, 32.0, 64.0, 128.0, 256.0, 1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 1048576.0, 4194304.0, 16777216.0],
		*REGISTRY
	).unwrap();
}
