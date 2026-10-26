//! Canary request latency and stall counts. `store` and `region` are fixed per canary
//! process and validated at startup; `op`, `result` and `threshold` are code-defined.

use rivet_metrics::prometheus::*;

lazy_static::lazy_static! {
	pub(crate) static ref REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"object_store_canary_request_duration_seconds",
			"Latency of object-store canary requests, by store, region, operation (put, put_conditional, get) and result (ok, precondition, not_found, error, timeout). One attempt per request, allowed 60 s, with no retries.",
		).buckets(rivet_metrics::BUCKETS.to_vec()),
		&["store", "region", "op", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref STALLS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"object_store_canary_stalls_total",
			"Object-store canary requests that stayed outstanding past each threshold (1s, 5s, 10s), counted when the threshold passes, by store, region and operation.",
		),
		&["store", "region", "op", "threshold"],
		*rivet_metrics::REGISTRY
	).unwrap();
}
