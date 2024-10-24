use rivet_metrics::{prometheus::*, BUCKETS, REGISTRY};

lazy_static::lazy_static! {
	pub static ref CHIRP_PERF_DURATION: HistogramVec =
		register_histogram_vec_with_registry!("chirp_perf_duration", "Duration of Chirp perf spans.", &["label"], BUCKETS.to_vec(), *REGISTRY).unwrap();
}
