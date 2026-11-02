//! Volume client request latency observed at the production transport adapter.

use crate::volume_client_machine::event::RpcError;
use rivet_data::generated::depot_client_protocol_v1::VolumeRejectResponse;
use rivet_metrics::prometheus::{
	HistogramOpts, HistogramVec, register_histogram_vec_with_registry,
};

lazy_static::lazy_static! {
	static ref REQUEST_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"depot_volume_client_request_duration_seconds",
			"Volume client lifecycle and replica request latency, including requests abandoned at their deadline.",
		)
		.buckets(rivet_metrics::BUCKETS.to_vec()),
		&["request", "result"],
		*rivet_metrics::REGISTRY
	)
	.unwrap();
}

/// Records one request when it completes, or as `cancelled` when its future is dropped
/// first, which is how the executor abandons a request at its deadline.
pub(super) struct RequestTimer {
	request: &'static str,
	started: Option<std::time::Instant>,
}

impl RequestTimer {
	pub(super) fn start(request: &'static str) -> Self {
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let started = std::time::Instant::now();
		Self {
			request,
			started: Some(started),
		}
	}

	pub(super) fn finish<T>(mut self, result: &Result<T, RpcError>) {
		let label = match result {
			Ok(_) => "ok",
			Err(RpcError::Rejected(_)) => "rejected",
			Err(RpcError::LifecycleRejected(reason)) => lifecycle_label(reason),
			Err(RpcError::RoutingUnavailable) => "routing_unavailable",
			Err(RpcError::Overloaded { .. }) => "overloaded",
			Err(RpcError::Unavailable) => "unavailable",
		};
		self.record(label);
	}

	fn record(&mut self, result: &str) {
		if let Some(started) = self.started.take() {
			REQUEST_DURATION
				.with_label_values(&[self.request, result])
				.observe(started.elapsed().as_secs_f64());
		}
	}
}

fn lifecycle_label(reason: &VolumeRejectResponse) -> &'static str {
	match reason {
		VolumeRejectResponse::NotMaintainer(_) => "not_maintainer",
		VolumeRejectResponse::TabletSplitting => "tablet_splitting",
		VolumeRejectResponse::VolumeNotFound => "volume_not_found",
		VolumeRejectResponse::VolumeExists => "volume_exists",
		VolumeRejectResponse::VolumeDeleted => "volume_deleted",
		VolumeRejectResponse::Overloaded(_) => "overloaded",
		VolumeRejectResponse::VolumeRecordMissing(_) => "record_missing",
		VolumeRejectResponse::VolumeUnderRepair(_) => "under_repair",
	}
}

impl Drop for RequestTimer {
	fn drop(&mut self) {
		self.record("cancelled");
	}
}
