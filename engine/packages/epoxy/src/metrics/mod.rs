use rivet_metrics::{BUCKETS, prometheus::*};

// Fixed operation names only: never use keys, values, URLs, or error text as labels.

/// Observe one call outside retryable mutations and pure planners. Dropped futures
/// count as cancelled (including an enclosing timeout); no completion is implied.
pub(crate) fn observe<'a, T: 'a>(
	kind: &'static str,
	operation: &'static str,
	work: impl std::future::Future<Output = anyhow::Result<T>> + 'a,
) -> impl std::future::Future<Output = anyhow::Result<T>> + 'a {
	// Consensus composes reads, recovery, and instrumented mutations. Box each
	// observed call's future here so that callers do not stack every nested frame.
	let work = Box::pin(work);
	async move {
		let mut attempt = WorkAttempt {
			kind,
			operation,
			result: "cancelled",
			_timer: crate::metrics::WORK_DURATION
				.with_label_values(&[kind, operation])
				.start_timer(),
		};
		crate::metrics::WORK_IN_FLIGHT
			.with_label_values(&[kind, operation])
			.inc();
		let result = work.await;
		attempt.result = if result.is_ok() { "ok" } else { "error" };
		if let Err(error) = &result {
			tracing::warn!(kind, operation, %error, "Epoxy work failed");
		}
		result
	}
}

struct WorkAttempt {
	kind: &'static str,
	operation: &'static str,
	result: &'static str,
	_timer: rivet_metrics::prometheus::HistogramTimer,
}

impl Drop for WorkAttempt {
	fn drop(&mut self) {
		crate::metrics::WORK_TOTAL
			.with_label_values(&[self.kind, self.operation, self.result])
			.inc();
		crate::metrics::WORK_IN_FLIGHT
			.with_label_values(&[self.kind, self.operation])
			.dec();
		tracing::debug!(
			kind = self.kind,
			operation = self.operation,
			result = self.result,
			"Epoxy work finished"
		);
	}
}



lazy_static::lazy_static! {
	pub static ref VERSION_PROBES: IntCounter = register_int_counter_with_registry!(
		Opts::new(
			"epoxy_version_probes_total",
			"Peer protocol version probes that returned a version.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref PROPOSAL_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"epoxy_proposal_total",
			"Answered proposals by path and result.",
		),
		&["path", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref KEY_TRANSITION_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"epoxy_key_transition_total",
			"Key state machine transitions by event and reply.",
		),
		&["event", "reply"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REQUEST_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"epoxy_requests_total",
			"Total number of replica HTTP requests.",
		),
		&["request_type", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"epoxy_request_duration_seconds",
			"Duration of replica HTTP request handling in seconds.",
		)
		.buckets(BUCKETS.to_vec()),
		&["request_type"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref WORK_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"epoxy_work_total",
			"Completed Epoxy work attempts; ok means the measured call returned successfully, not necessarily convergence.",
		),
		&["kind", "operation", "result"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref WORK_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"epoxy_work_duration_seconds",
			"Epoxy work attempt duration, including interrupted calls.",
		)
		.buckets(BUCKETS.to_vec()),
		&["kind", "operation"],
		*rivet_metrics::REGISTRY
	).unwrap();
	pub static ref WORK_IN_FLIGHT: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new(
			"epoxy_work_in_flight",
			"Currently executing Epoxy work attempts.",
		),
		&["kind", "operation"],
		*rivet_metrics::REGISTRY
	).unwrap();
}
