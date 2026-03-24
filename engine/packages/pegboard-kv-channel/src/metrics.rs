use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref KV_CHANNEL_REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_kv_channel_request_duration_seconds",
		"Duration of KV channel handler requests.",
		&["op"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref KV_CHANNEL_REQUEST_KEYS: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_kv_channel_request_keys",
		"Number of keys per KV channel request.",
		&["op"],
		vec![1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 128.0],
		*REGISTRY
	).unwrap();

	pub static ref KV_CHANNEL_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_kv_channel_requests_total",
		"Total KV channel requests handled.",
		&["op"],
		*REGISTRY
	).unwrap();
}
