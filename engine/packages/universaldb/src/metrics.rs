use rivet_metrics::{BUCKETS, MICRO_BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref KEY_PACK_COUNT: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_key_pack_count",
		"How many times a key has been packed.",
		&["type"],
		*REGISTRY
	).unwrap();
	pub static ref KEY_UNPACK_COUNT: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_key_unpack_count",
		"How many times a key has been unpacked.",
		&["type"],
		*REGISTRY
	).unwrap();

	pub static ref TRANSACTION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_transaction_total",
		"How many transactions have been started.",
		&["name"],
		*REGISTRY
	).unwrap();
	pub static ref TRANSACTION_PENDING: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"udb_transaction_pending",
		"How many transactions have been started.",
		&["name"],
		*REGISTRY
	).unwrap();
	pub static ref TRANSACTION_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"udb_transaction_duration",
		"Duration of a transaction.",
		&["name"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref TRANSACTION_ATTEMPTS: HistogramVec = register_histogram_vec_with_registry!(
		"udb_transaction_attempts",
		"Amount of attempts (1 + retries) taken for a transaction.",
		&["name"],
		vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 10.0, 12.0, 14.0, 16.0],
		*REGISTRY
	).unwrap();

	pub static ref OPERATION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_operation_total",
		"How many UniversalDB operations have completed.",
		&["op", "isolation", "result"],
		*REGISTRY
	).unwrap();
	pub static ref OPERATION_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"udb_operation_duration_seconds",
		"Duration of UniversalDB operations.",
		&["op", "isolation", "result"],
		MICRO_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref OPERATION_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_operation_bytes",
		"Bytes read or written by UniversalDB operations.",
		&["op", "direction"],
		*REGISTRY
	).unwrap();
	pub static ref OPERATION_KEYS: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_operation_keys",
		"Keys read or written by UniversalDB operations.",
		&["op"],
		*REGISTRY
	).unwrap();
	pub static ref TRANSACTION_READ_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_transaction_read_bytes",
		"Bytes read by UniversalDB transactions.",
		&["name"],
		*REGISTRY
	).unwrap();
	pub static ref TRANSACTION_MUTATION_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		"udb_transaction_mutation_bytes",
		"Bytes written or cleared by UniversalDB transactions.",
		&["name"],
		*REGISTRY
	).unwrap();
}
