use rivet_metrics::{prometheus::*, REGISTRY};

lazy_static::lazy_static! {
	pub static ref CACHE_REQUEST_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"cache_request_total",
		"Total number of cache requests.",
		&["key"],
		*REGISTRY,
	).unwrap();
	pub static ref CACHE_REQUEST_ERRORS: IntCounterVec = register_int_counter_vec_with_registry!(
		"cache_request_errors",
		"Total number of cache request errors.",
		&["key"],
		*REGISTRY,
	).unwrap();
	pub static ref CACHE_PURGE_REQUEST_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"cache_purge_request_total",
		"Total number of cache purge requests.",
		&["key"],
		*REGISTRY,
	).unwrap();
	pub static ref CACHE_VALUE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"cache_value_total",
		"Total number of cache values requested.",
		&["key"],
		*REGISTRY,
	).unwrap();
	pub static ref CACHE_VALUE_MISS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"cache_value_miss_total",
		"Total number of cache value misses.",
		&["key"],
		*REGISTRY,
	).unwrap();
	pub static ref CACHE_VALUE_EMPTY_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"cache_value_empty_total",
		"Total number of cache values that were requested but not resolved by cache nor getter.",
		&["key"],
		*REGISTRY,
	).unwrap();
	pub static ref CACHE_PURGE_VALUE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"cache_purge_value_total",
		"Total number of cache values purged.",
		&["key"],
		*REGISTRY,
	).unwrap();
}
