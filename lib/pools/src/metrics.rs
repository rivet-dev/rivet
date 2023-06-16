use rivet_metrics::{prometheus::*, REGISTRY};

lazy_static::lazy_static! {
	// MARK: CRDB
	pub static ref CRDB_POOL_SIZE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"crdb_pool_conn_size",
		"Number of SQL connections in the pool.",
		&["db_name"],
		*REGISTRY,
	)
	.unwrap();
	pub static ref CRDB_POOL_NUM_IDLE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"crdb_pool_num_idle",
		"Number of idle SQL connections in the pool.",
		&["db_name"],
		*REGISTRY,
	)
	.unwrap();
}
