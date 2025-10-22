use rivet_metrics::otel::{global::*, metrics::*};

lazy_static::lazy_static! {
	static ref METER: Meter = meter("rivet-cache");

	/// Expected attributes: "key"
	pub static ref CACHE_REQUEST_TOTAL: Counter<u64> = METER.u64_counter("rivet_cache_request_total")
		.with_description("Total number of cache requests.")
		.build();
	/// Expected attributes: "key"
	pub static ref CACHE_REQUEST_ERRORS: Counter<u64> = METER.u64_counter("rivet_cache_request_errors")
		.with_description("Total number of cache request errors.")
		.build();
	/// Expected attributes: "key"
	pub static ref CACHE_PURGE_REQUEST_TOTAL: Counter<u64> = METER.u64_counter("rivet_cache_purge_request_total")
		.with_description("Total number of cache purge requests.")
		.build();
	/// Expected attributes: "key"
	pub static ref CACHE_VALUE_TOTAL: Counter<u64> = METER.u64_counter("rivet_cache_value_total")
		.with_description("Total number of cache values requested.")
		.build();
	/// Expected attributes: "key"
	pub static ref CACHE_VALUE_MISS_TOTAL: Counter<u64> = METER.u64_counter("rivet_cache_value_miss_total")
		.with_description("Total number of cache value misses.")
		.build();
	/// Expected attributes: "key"
	pub static ref CACHE_VALUE_EMPTY_TOTAL: Counter<u64> = METER.u64_counter("rivet_cache_value_empty_total")
		.with_description("Total number of cache values that were requested but not resolved by cache nor getter.")
		.build();
	/// Expected attributes: "key"
	pub static ref CACHE_PURGE_VALUE_TOTAL: Counter<u64> = METER.u64_counter("rivet_cache_purge_value_total")
		.with_description("Total number of cache values purged.")
		.build();
}
