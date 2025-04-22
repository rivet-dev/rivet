use rivet_metrics::{prometheus::*, BUCKETS, MICRO_BUCKETS, REGISTRY};

lazy_static::lazy_static! {
	// MARK: CRDB
	pub static ref CRDB_POOL_SIZE: IntGauge = register_int_gauge_with_registry!(
		"crdb_pool_conn_size",
		"Number of CRDB connections in the pool.",
		*REGISTRY,
	)
	.unwrap();
	pub static ref CRDB_POOL_NUM_IDLE: IntGauge = register_int_gauge_with_registry!(
		"crdb_pool_num_idle",
		"Number of idle CRDB connections in the pool.",
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
	pub static ref SQL_QUERY_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sql_query_duration",
		"Total duration of sql query.",
		&["action", "context_name", "location"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
	pub static ref SQL_ACQUIRE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sql_acquire_duration",
		"Total duration to acquire an sql connection.",
		&["action", "context_name", "location"],
		BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
	pub static ref SQL_ACQUIRE_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"sql_acquire_total",
		"Amount times a pool connection was acquired.",
		&["action", "context_name", "location", "acquire_result"],
		*REGISTRY,
	).unwrap();
	pub static ref SQL_ACQUIRE_TRIES: IntCounterVec = register_int_counter_vec_with_registry!(
		"sql_acquire_tries",
		"Amount of tries required to get a pool connection.",
		&["action", "context_name", "location", "acquire_result"],
		*REGISTRY,
	).unwrap();

	// MARK: Sqlite
	pub static ref SQLITE_POOL_SIZE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"sqlite_pool_conn_size",
		"Number of Sqlite connections in the pool.",
		&["conn_type"],
		*REGISTRY,
	)
	.unwrap();
	pub static ref SQLITE_GET_POOL_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"sqlite_get_pool_duration",
		"Duration to fully connect to an Sqlite database.",
		&["conn_type", "did_insert"],
		MICRO_BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
	pub static ref SQLITE_LARGE_DB: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"sqlite_large_db",
		"Sqlite databases stored in FDB that are larger than 3 MiB.",
		&["key"],
		*REGISTRY,
	)
	.unwrap();
	pub static ref SQLITE_UPLOAD_DB_RATE: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_upload_db_rate",
		"Rate to read and upload to FDB an sqlite DB in bytes/s.",
		&["key"],
		*REGISTRY,
	)
	.unwrap();
	pub static ref SQLITE_DOWNLOAD_DB_RATE: GaugeVec = register_gauge_vec_with_registry!(
		"sqlite_download_db_rate",
		"Rate to download and write to file an sqlite DB from FDB in bytes/s.",
		&["key"],
		*REGISTRY,
	)
	.unwrap();
}
