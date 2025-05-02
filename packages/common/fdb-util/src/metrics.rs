use rivet_metrics::{prometheus::*, MICRO_BUCKETS, REGISTRY};

lazy_static::lazy_static! {
	pub static ref PING_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"fdb_ping_duration",
		"Total duration to retrieve a single value from FDB.",
		&[],
		MICRO_BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
	pub static ref MISSED_PING: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"fdb_missed_ping",
		"1 if FDB missed the last ping.",
		&[],
		*REGISTRY,
	)
	.unwrap();

	pub static ref KEY_PACK_COUNT: IntCounterVec = register_int_counter_vec_with_registry!(
		"fdb_key_pack_count",
		"How many times a key has been packed.",
		&["type"],
		*REGISTRY,
	)
	.unwrap();
	pub static ref KEY_UNPACK_COUNT: IntCounterVec = register_int_counter_vec_with_registry!(
		"fdb_key_unpack_count",
		"How many times a key has been unpacked.",
		&["type"],
		*REGISTRY,
	)
	.unwrap();
}
