use rivet_metrics::{REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	// PubSub layer metrics
	pub static ref LOCAL_SUBSCRIBERS_COUNT: IntGauge = register_int_gauge_with_registry!(
		"ups_local_subscribers_count",
		"Number of local in-memory subscriber entries in the pubsub layer.",
		*REGISTRY
	).unwrap();
	pub static ref REPLY_SUBSCRIBERS_COUNT: IntGauge = register_int_gauge_with_registry!(
		"ups_reply_subscribers_count",
		"Number of pending reply subscriber entries in the pubsub layer.",
		*REGISTRY
	).unwrap();

	// Memory driver metrics
	pub static ref MEMORY_SUBSCRIBERS_COUNT: IntGauge = register_int_gauge_with_registry!(
		"ups_memory_subscribers_count",
		"Number of subject entries in the memory driver.",
		*REGISTRY
	).unwrap();

	// Postgres driver metrics
	pub static ref POSTGRES_SUBSCRIPTIONS_COUNT: IntGauge = register_int_gauge_with_registry!(
		"ups_postgres_subscriptions_count",
		"Number of subscription entries in the postgres driver.",
		*REGISTRY
	).unwrap();
}
