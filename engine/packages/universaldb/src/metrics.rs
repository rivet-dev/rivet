use rivet_metrics::{BUCKETS, REGISTRY, prometheus::*};

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
}
