use std::sync::{
	Arc,
	atomic::{AtomicBool, AtomicUsize, Ordering},
};

use anyhow::Result;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use super::PubSub;

pub(super) struct Lifecycle {
	pub draining: AtomicBool,
	/// Cancelled when draining begins, so served peer channels stop reading and end.
	pub drain_started: CancellationToken,
	active: AtomicUsize,
	changed: Notify,
	pub stopped: CancellationToken,
	pub listening: CancellationToken,
	requests_drained: CancellationToken,
}

impl Lifecycle {
	pub fn new() -> Self {
		Self {
			draining: Default::default(),
			drain_started: Default::default(),
			active: Default::default(),
			changed: Default::default(),
			stopped: Default::default(),
			listening: Default::default(),
			requests_drained: Default::default(),
		}
	}

	pub fn accept(self: &Arc<Self>, permit: super::request::RequestPermit) -> ActiveRequest {
		self.active.fetch_add(1, Ordering::AcqRel);

		ActiveRequest {
			lifecycle: self.clone(),
			_permit: permit,
			_metric: rivet_metrics::GaugeGuardExt::inc_guard(&*crate::metrics::REQUEST_ACTIVE),
		}
	}
}

pub(super) struct ActiveRequest {
	lifecycle: Arc<Lifecycle>,
	/// Counts this request on `REQUEST_ACTIVE` until it ends.
	_metric: rivet_metrics::IntGaugeGuard,
	_permit: super::request::RequestPermit,
}

impl ActiveRequest {
	/// Transfer capacity after decoding a subject without dropping the drain reservation.
	pub(super) fn replace_permit(&mut self, permit: super::request::RequestPermit) {
		self._permit = permit;
	}
}

impl Drop for ActiveRequest {
	fn drop(&mut self) {
		self.lifecycle.active.fetch_sub(1, Ordering::AcqRel);
		self.lifecycle.changed.notify_waiters();
	}
}

impl PubSub {
	pub(crate) fn cancel_services(&self) {
		self.stop_accepting_requests();
		self.inner.lifecycle.requests_drained.cancel();
		self.inner.lifecycle.stopped.cancel();
	}

	/// Called by the internal API server once its listener is bound and routes installed.
	pub fn mark_listening(&self) {
		self.inner.lifecycle.listening.cancel();
	}

	pub async fn wait_listening(&self) {
		self.inner.lifecycle.listening.cancelled().await;
	}

	/// A closed request inbox is expected once process draining has begun.
	pub fn is_draining(&self) -> bool {
		self.inner.lifecycle.draining.load(Ordering::Acquire)
	}

	/// Wait before closing resources needed by accepted requests. The drain deadline
	/// also releases this wait so a stuck handler cannot prevent forced cleanup.
	pub async fn requests_drained(&self) {
		self.inner.lifecycle.requests_drained.cancelled().await;
	}

	/// Stop accepting requests across all clones, then wait for accepted messages to be
	/// replied to or dropped, including admitted HTTP body readers.
	/// The deadline bounds draining, but does not abort application handler tasks.
	#[tracing::instrument(skip_all)]
	pub async fn drain_requests(&self, deadline: u64) -> Result<()> {
		self.stop_accepting_requests();

		tracing::info!("draining UPS requests");
		let result = rivet_env::timeout_at(self.env().clock().as_ref(), deadline, async {
			loop {
				let changed = self.inner.lifecycle.changed.notified();
				tokio::pin!(changed);
				changed.as_mut().enable();

				if self.inner.lifecycle.active.load(Ordering::Acquire) == 0 {
					break;
				}

				changed.await;
			}
		})
		.await;

		if result.is_err() {
			crate::metrics::SHUTDOWN
				.with_label_values(&["requests_timeout"])
				.inc();
			tracing::warn!(
				pending = self.inner.lifecycle.active.load(Ordering::Acquire),
				"UPS request drain timed out"
			);
			self.inner.lifecycle.stopped.cancel();
		}

		self.inner.lifecycle.requests_drained.cancel();

		result.map_err(super::request::timeout_error)
	}

	fn stop_accepting_requests(&self) {
		let mut subscriptions = self.inner.subscriptions.lock();
		self.inner.lifecycle.draining.store(true, Ordering::Release);
		self.inner.lifecycle.drain_started.cancel();
		subscriptions.clear();
	}

	/// Drain requests, then mark UPS stopped.
	#[tracing::instrument(skip_all)]
	pub async fn shutdown(&self, deadline: u64) -> Result<()> {
		let requests = self.drain_requests(deadline).await;
		self.inner.channels.shutdown().await;
		self.inner.lifecycle.stopped.cancel();

		requests
	}

	/// Wait for UPS shutdown.
	pub async fn stopped(&self) {
		self.inner.lifecycle.stopped.cancelled().await;
	}
}
