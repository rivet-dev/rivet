use prometheus::*;

// mod buckets;
mod registry;
mod server;

// pub use buckets::BUCKETS;
pub use registry::REGISTRY;
pub use server::run_standalone;

lazy_static::lazy_static! {
	pub static ref PACKET_RECV_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"packet_recv_total",
		"Total number of packets received.",
		&[],
		*REGISTRY,
	).unwrap();

	pub static ref PACKET_SEND_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"packet_send_total",
		"Total number of packets sent.",
		&[],
		*REGISTRY,
	).unwrap();

	pub static ref UNKNOWN_ISOLATE_RUNNER: IntCounterVec = register_int_counter_vec_with_registry!(
		"unknown_isolate_runner",
		"Total number of unknown isolate runners that were found and killed.",
		&[],
		*REGISTRY,
	).unwrap();

	pub static ref DUPLICATE_ISOLATE_RUNNER: IntCounterVec = register_int_counter_vec_with_registry!(
		"duplicate_isolate_runner",
		"Total number of duplicate isolate runners that were found and killed.",
		&[],
		*REGISTRY,
	).unwrap();

	pub static ref SQL_ERROR: IntCounterVec = register_int_counter_vec_with_registry!(
		"sql_error",
		"An SQL error occurred.",
		&["error"],
		*REGISTRY,
	).unwrap();
}
