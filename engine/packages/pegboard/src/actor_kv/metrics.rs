use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref ACTOR_KV_OPERATION_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"actor_kv_operation_duration_seconds",
		"Duration of actor KV operations including UDB transaction.",
		&["op"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_KV_KEYS_PER_OP: HistogramVec = register_histogram_vec_with_registry!(
		"actor_kv_keys_per_operation",
		"Number of keys per actor KV operation.",
		&["op"],
		vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0],
		*REGISTRY
	).unwrap();
}
