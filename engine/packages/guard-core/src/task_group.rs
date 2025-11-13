use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::Future;
use tokio::sync::Notify;
use tracing::Instrument;

pub struct TaskGroup {
	running_count: AtomicUsize,
	notify: Notify,
}

impl TaskGroup {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			running_count: AtomicUsize::new(0),
			notify: Notify::new(),
		})
	}

	pub fn spawn<F, O>(self: &Arc<Self>, fut: F)
	where
		F: Future<Output = O> + Send + 'static,
	{
		self.running_count.fetch_add(1, Ordering::Relaxed);

		// TODO: Handle panics
		let self2 = self.clone();
		tokio::spawn(
			async move {
				fut.await;

				// Decrement and notify any waiters if the count hits zero
				if self2.running_count.fetch_sub(1, Ordering::AcqRel) == 1 {
					self2.notify.notify_waiters();
				}
			}
			.in_current_span(),
		);
	}

	#[tracing::instrument(skip_all)]
	pub async fn wait_idle(&self) {
		// Fast path
		if self.running_count.load(Ordering::Acquire) == 0 {
			return;
		}

		// Wait for notifications until the count reaches zero
		loop {
			self.notify.notified().await;
			if self.running_count.load(Ordering::Acquire) == 0 {
				break;
			}
		}
	}
}
