use std::{future::Future, time::Duration};

use anyhow::Result;
use futures_util::FutureExt;
use rivet_env::capabilities::shutdown::Shutdown;

use crate::PubSub;

/// Owns UPS tasks around application services, including services that do not run Depot.
/// On termination, services keep request resources alive until `requests_drained()`
/// completes, then finish their cleanup before UPS shuts down.
pub async fn run(
	ups: PubSub,
	shutdown: Shutdown,
	shutdown_timeout: Duration,
	services: impl Future<Output = Result<()>>,
) -> Result<()> {
	run_with_before_drain(ups, shutdown, shutdown_timeout, services, async { Ok(()) }).await
}

/// Allows application owners to send their final requests before UPS closes admission.
/// Preparation shares the original shutdown deadline and keeps service handlers polling.
pub async fn run_with_before_drain(
	ups: PubSub,
	mut shutdown: Shutdown,
	shutdown_timeout: Duration,
	services: impl Future<Output = Result<()>>,
	before_drain: impl Future<Output = Result<()>>,
) -> Result<()> {
	let _owner = Owner(&ups);
	let clock = ups.env().clock();
	let mut services = Box::pin(services);

	let mut completed = rivet_env::select! { ups.env().rng("ups.lifecycle.select");
		result = services.as_mut() => Some(result),
		signal = shutdown.recv() => { signal?; None },
	};
	// A service can observe termination and return in the same poll. Preserve the
	// original shutdown budget even if its completion wins the selection above.
	let signalled = completed.is_none() || shutdown.recv().now_or_never().transpose()?.is_some();
	let deadline = if signalled {
		shutdown.deadline(shutdown_timeout)?
	} else {
		rivet_env::deadline_after(clock.as_ref(), shutdown_timeout)?
	};

	// Client cleanup can call a service in this very process, so do not clear subscriptions yet.
	let prepared = rivet_env::timeout_at(clock.as_ref(), deadline, async {
		let mut before_drain = Box::pin(before_drain);
		if completed.is_some() {
			return before_drain.await;
		}
		rivet_env::select_biased!("finish application cleanup before stopping request handlers",
			result = before_drain.as_mut() => result,
			result = services.as_mut() => {
				completed = Some(result);
				before_drain.await
			},
		)
	})
	.await;

	// Keep polling services during the drain: their handlers may need to run before
	// requests can complete, and their cleanup may publish final notifications.
	let drained = rivet_env::timeout_at(clock.as_ref(), deadline, async {
		if let Some(result) = completed {
			let requests = ups.drain_requests(deadline).await;
			result.and(requests)
		} else {
			let (requests, services) =
				futures_util::join!(ups.drain_requests(deadline), services.as_mut());

			requests.and(services)
		}
	})
	.await;
	drop(services);

	let stopped = ups.shutdown(deadline).await;
	prepared?.and(drained?).and(stopped)
}

struct Owner<'a>(&'a PubSub);

impl Drop for Owner<'_> {
	fn drop(&mut self) {
		self.0.cancel_services();
	}
}
