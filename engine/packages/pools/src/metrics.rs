use rivet_metrics::{
	BUCKETS,
	otel::{global::*, metrics::*},
};

lazy_static::lazy_static! {
	static ref METER: Meter = meter("rivet-pools");

	// MARK: SQL
	/// Expected attributes: "action", "context_name", "location"
	pub static ref SQL_QUERY_TOTAL: Counter<u64> = METER.u64_counter("rivet_sql_query_total")
		.with_description("Total number of queries.")
		.build();
	/// Expected attributes: "action", "context_name", "location"
	pub static ref SQL_QUERY_DURATION: Histogram<f64> = METER.f64_histogram("rivet_sql_query_duration")
		.with_description("Total duration of sql query.")
		.with_boundaries(BUCKETS.to_vec())
		.build();
	/// Expected attributes: "action", "context_name", "location"
	pub static ref SQL_ACQUIRE_DURATION: Histogram<f64> = METER.f64_histogram("rivet_sql_acquire_duration")
		.with_description("Total duration to acquire an sql connection.")
		.with_boundaries(BUCKETS.to_vec())
		.build();
	/// Expected attributes: "action", "context_name", "location", "acquire_result"
	pub static ref SQL_ACQUIRE_TOTAL: Counter<u64> = METER.u64_counter("rivet_sql_acquire_total")
		.with_description("Amount times a pool connection was acquired.")
		.build();
	/// Expected attributes: "action", "context_name", "location", "acquire_result"
	pub static ref SQL_ACQUIRE_TRIES: Counter<u64> = METER.u64_counter("rivet_sql_acquire_tries")
		.with_description("Amount of tries required to get a pool connection.")
		.build();
}
