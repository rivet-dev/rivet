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

	pub static ref ACTOR_KV_SQLITE_STORAGE_REQUEST_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"actor_kv_sqlite_storage_request_total",
		"Count of actor KV requests that touch SQLite page-store keys.",
		&["path", "op"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_KV_SQLITE_STORAGE_ENTRY_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"actor_kv_sqlite_storage_entry_total",
		"Count of SQLite page-store entries touched by actor KV requests.",
		&["path", "op", "entry_kind"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_KV_SQLITE_STORAGE_BYTES_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"actor_kv_sqlite_storage_bytes_total",
		"Request, response, and payload bytes for SQLite page-store actor KV traffic.",
		&["path", "op", "byte_kind"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_KV_SQLITE_STORAGE_DURATION_SECONDS_TOTAL: CounterVec = register_counter_vec_with_registry!(
		"actor_kv_sqlite_storage_duration_seconds_total",
		"Total wall-clock time spent serving SQLite page-store actor KV requests.",
		&["path", "op"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_KV_SQLITE_STORAGE_PHASE_DURATION_SECONDS_TOTAL: CounterVec = register_counter_vec_with_registry!(
		"actor_kv_sqlite_storage_phase_duration_seconds_total",
		"Total wall-clock time spent in SQLite-specific actor KV phases.",
		&["path", "phase"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_KV_SQLITE_STORAGE_CLEAR_SUBSPACE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"actor_kv_sqlite_storage_clear_subspace_total",
		"Count of generic clear_subspace_range calls needed for SQLite page-store writes.",
		&["path"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_KV_SQLITE_STORAGE_VALIDATION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"actor_kv_sqlite_storage_validation_total",
		"Count of SQLite page-store write validation outcomes.",
		&["path", "result"],
		*REGISTRY
	).unwrap();
}
