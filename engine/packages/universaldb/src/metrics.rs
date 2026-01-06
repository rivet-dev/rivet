use rivet_metrics::{MICRO_BUCKETS, REGISTRY, prometheus::*};

lazy_static::lazy_static! {
	pub static ref FDB_PING_DURATION: Histogram = register_histogram_with_registry!(
		"udb_fdb_ping_duration",
		"Total duration to retrieve a single value from fdb.",
		MICRO_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref FDB_MISSED_PING: IntGauge = register_int_gauge_with_registry!(
		"udb_fdb_missed_ping",
		"1 if fdb missed the last ping.",
		*REGISTRY
	).unwrap();

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
}
