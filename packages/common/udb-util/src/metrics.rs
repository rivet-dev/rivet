use rivet_metrics::{
	MICRO_BUCKETS,
	otel::{global::*, metrics::*},
};

lazy_static::lazy_static! {
	static ref METER: Meter = meter("rivet-udb-util");

	/// Has no expected attributes
	pub static ref PING_DURATION: Histogram<f64> = METER.f64_histogram("rivet_udb_ping_duration")
		.with_description("Total duration to retrieve a single value from udb.")
		.with_boundaries(MICRO_BUCKETS.to_vec())
		.build();
	/// Has no expected attributes
	pub static ref MISSED_PING: Gauge<u64> = METER.u64_gauge("rivet_udb_missed_ping")
		.with_description("1 if udb missed the last ping.")
		.build();

	/// Expected attributes: "type"
	pub static ref KEY_PACK_COUNT: Counter<u64> = METER.u64_counter("rivet_udb_key_pack_count")
		.with_description("How many times a key has been packed.")
		.build();
	/// Expected attributes: "type"
	pub static ref KEY_UNPACK_COUNT: Counter<u64> = METER.u64_counter("rivet_udb_key_unpack_count")
		.with_description("How many times a key has been unpacked.")
		.build();
}
