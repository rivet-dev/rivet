use rivet_metrics::{prometheus::*, BUCKETS, REGISTRY};

lazy_static::lazy_static! {
	// MARK: CRDB
	pub static ref CRDB_POOL_SIZE: IntGauge = register_int_gauge_with_registry!(
		"crdb_pool_conn_size",
		"Number of SQL connections in the pool.",
		*REGISTRY,
	)
	.unwrap();
	pub static ref CRDB_POOL_NUM_IDLE: IntGauge = register_int_gauge_with_registry!(
		"crdb_pool_num_idle",
		"Number of idle SQL connections in the pool.",
		*REGISTRY,
	)
	.unwrap();

	// MARK: SQL
	pub static ref SQL_QUERY_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sql_query_total",
		"Total number of queries.",
		&["action", "context_name", "location"],
		*REGISTRY,
	).unwrap();
	pub static ref SQL_QUERY_PREPARE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sql_query_prepare_duration",
		"Total number of queries.",
		&["action", "context_name", "location"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
	pub static ref SQL_QUERY_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sql_query_duration",
		"Total number of queries.",
		&["action", "context_name", "location"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
}
