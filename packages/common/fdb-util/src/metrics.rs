use rivet_metrics::{prometheus::*, MICRO_BUCKETS, REGISTRY};

lazy_static::lazy_static! {
	pub static ref FDB_PING_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		"fdb_ping_duration",
		"Total duration to retrieve a single value from FDB.",
		&[],
		MICRO_BUCKETS.to_vec(),
		*REGISTRY,
	).unwrap();
	pub static ref FDB_MISSED_PING: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"fdb_missed_ping",
		"1 if FDB missed the last ping.",
		&[],
		*REGISTRY,
	)
	.unwrap();
}
