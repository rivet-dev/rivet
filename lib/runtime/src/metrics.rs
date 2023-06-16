use rivet_metrics::{prometheus::*, REGISTRY};

lazy_static::lazy_static! {
	// MARK: Tokio
	pub static ref TOKIO_THREAD_COUNT: IntGauge =
		register_int_gauge_with_registry!(
			"tokio_thread_count",
			"Total number of Tokio threads.",
			*REGISTRY
		).unwrap();
}
