use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	// MARK: SQL
	pub static ref SQL_QUERY_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sql_query_total",
		"Total number of queries.",
		&["action", "context_name", "location"],
		*REGISTRY
	).unwrap();
	pub static ref SQL_QUERY_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sql_query_duration",
		"Total duration of sql query.",
		&["action", "context_name", "location"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref SQL_ACQUIRE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sql_acquire_duration",
		"Total duration to acquire an sql connection.",
		&["action", "context_name", "location"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref SQL_ACQUIRE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sql_acquire_total",
		"Amount times a pool connection was acquired.",
		&["action", "context_name", "location", "acquire_result"],
		*REGISTRY
	).unwrap();
	pub static ref SQL_ACQUIRE_TRIES: IntCounterVec = register_int_counter_vec_with_registry!(
		"sql_acquire_tries",
		"Amount of tries required to get a pool connection.",
		&["action", "context_name", "location", "acquire_result"],
		*REGISTRY
	).unwrap();
}
