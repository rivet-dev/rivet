//! TODO: replace with `pegboard_envoy_stale_sweep` gasoline workflow.
//!
//! This per-process scheduler is a stepping stone. The spawn pattern multiplies
//! expire op-invocation traffic by cluster size because every engine process can
//! independently observe the same stale envoy. A cluster-singleton workflow would
//! scale O(1) instead. See `.agent/todo/envoy-stale-sweep-workflow.md`.

use std::{
	future::Future,
	panic::AssertUnwindSafe,
	pin::Pin,
	sync::{
		Arc, OnceLock,
		atomic::{AtomicBool, Ordering},
	},
	time::Instant,
};

use futures_util::FutureExt;
use gas::prelude::Id;
use tokio::sync::{Notify, Semaphore};

use crate::{metrics, ops};

pub struct EnvoyExpireScheduler {
	pending: scc::HashSet<String>,
	pending_empty: Notify,
	semaphore: Arc<Semaphore>,
	max_pending: usize,
	expire: ExpireFn,
}

static SCHEDULER: OnceLock<Arc<EnvoyExpireScheduler>> = OnceLock::new();

type ExpireFn = Arc<
	dyn Fn(
			Id,
			String,
		) -> Pin<Box<dyn Future<Output = anyhow::Result<ops::envoy::expire::Output>> + Send>>
		+ Send
		+ Sync,
>;

pub fn get(pools: &rivet_pools::PoolsHandle) -> &'static Arc<EnvoyExpireScheduler> {
	SCHEDULER.get_or_init(|| {
		let pegboard_config = pools.config().pegboard();
		let pools = pools.clone();
		Arc::new(EnvoyExpireScheduler {
			pending: scc::HashSet::default(),
			pending_empty: Notify::new(),
			semaphore: Arc::new(Semaphore::new(
				pegboard_config.envoy_expire_scheduler_max_concurrent_expires(),
			)),
			max_pending: pegboard_config.envoy_expire_scheduler_max_pending(),
			expire: Arc::new(move |ns, envoy_key| {
				let pools = pools.clone();
				Box::pin(async move {
					let input = ops::envoy::expire::Input {
						namespace_id: ns,
						envoy_key,
						skip_if_fresh: true,
					};

					ops::envoy::expire::expire_with_pools(pools.config(), &pools, &input).await
				})
			}),
		})
	})
}

impl EnvoyExpireScheduler {
	#[doc(hidden)]
	pub fn new_for_tests<F, Fut>(
		max_pending: usize,
		max_concurrent_expires: usize,
		expire: F,
	) -> Arc<Self>
	where
		F: Fn(Id, String) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = anyhow::Result<ops::envoy::expire::Output>> + Send + 'static,
	{
		Arc::new(EnvoyExpireScheduler {
			pending: scc::HashSet::default(),
			pending_empty: Notify::new(),
			semaphore: Arc::new(Semaphore::new(max_concurrent_expires)),
			max_pending,
			expire: Arc::new(move |ns, envoy_key| Box::pin(expire(ns, envoy_key))),
		})
	}

	#[doc(hidden)]
	pub fn pending_len(&self) -> usize {
		self.pending.len()
	}

	#[doc(hidden)]
	pub async fn wait_pending_empty(&self) {
		loop {
			let notified = self.pending_empty.notified();
			if self.pending.len() == 0 {
				return;
			}
			notified.await;
		}
	}

	pub fn try_enqueue(self: &Arc<Self>, ns: Id, envoy_key: String) {
		let namespace_id = ns.to_string();

		if self.pending.len() >= self.max_pending {
			metrics::ENVOY_EXPIRE_SCHEDULER_ENQUEUED_TOTAL
				.with_label_values(&[namespace_id.as_str(), "rejected_capacity"])
				.inc();
			return;
		}

		if self.pending.insert_sync(envoy_key.clone()).is_err() {
			metrics::ENVOY_EXPIRE_SCHEDULER_ENQUEUED_TOTAL
				.with_label_values(&[namespace_id.as_str(), "deduped"])
				.inc();
			return;
		}

		metrics::ENVOY_EXPIRE_SCHEDULER_ENQUEUED_TOTAL
			.with_label_values(&[namespace_id.as_str(), "scheduled"])
			.inc();
		metrics::ENVOY_EXPIRE_SCHEDULER_PENDING.set(self.pending.len() as i64);

		let scheduler = Arc::clone(self);
		tokio::spawn(expire_worker(scheduler, ns, envoy_key));
	}
}

async fn expire_worker(scheduler: Arc<EnvoyExpireScheduler>, ns: Id, envoy_key: String) {
	let start = Instant::now();
	let namespace_id = ns.to_string();
	let pending_key = envoy_key.clone();
	let in_flight = Arc::new(AtomicBool::new(false));
	let in_flight_guard = Arc::clone(&in_flight);

	scopeguard::defer! {
		scheduler.pending.remove_sync(&pending_key);
		metrics::ENVOY_EXPIRE_SCHEDULER_PENDING.set(scheduler.pending.len() as i64);
		if scheduler.pending.len() == 0 {
			scheduler.pending_empty.notify_waiters();
		}
		if in_flight_guard.load(Ordering::Relaxed) {
			metrics::ENVOY_EXPIRE_SCHEDULER_IN_FLIGHT.dec();
		}
		metrics::ENVOY_EXPIRE_SCHEDULER_DURATION
			.with_label_values(&[namespace_id.as_str()])
			.observe(start.elapsed().as_secs_f64());
	}

	let permit = match scheduler.semaphore.clone().acquire_owned().await {
		Ok(permit) => permit,
		Err(err) => {
			metrics::ENVOY_EXPIRE_SCHEDULER_COMPLETED_TOTAL
				.with_label_values(&[namespace_id.as_str(), "error"])
				.inc();
			tracing::warn!(?err, ?ns, %envoy_key, "envoy expire scheduler semaphore closed");
			return;
		}
	};

	metrics::ENVOY_EXPIRE_SCHEDULER_IN_FLIGHT.inc();
	in_flight.store(true, Ordering::Relaxed);

	let result = AssertUnwindSafe((scheduler.expire)(ns, envoy_key.clone()))
		.catch_unwind()
		.await;
	drop(permit);

	match result {
		Ok(Ok(out)) if out.did_expire => {
			metrics::ENVOY_EXPIRE_SCHEDULER_COMPLETED_TOTAL
				.with_label_values(&[namespace_id.as_str(), "expired"])
				.inc();
		}
		Ok(Ok(_)) => {
			metrics::ENVOY_EXPIRE_SCHEDULER_COMPLETED_TOTAL
				.with_label_values(&[namespace_id.as_str(), "skipped_fresh_or_already_expired"])
				.inc();
		}
		Ok(Err(err)) => {
			metrics::ENVOY_EXPIRE_SCHEDULER_COMPLETED_TOTAL
				.with_label_values(&[namespace_id.as_str(), "error"])
				.inc();
			tracing::warn!(?err, ?ns, %envoy_key, "read-path envoy expire failed");
		}
		Err(_) => {
			metrics::ENVOY_EXPIRE_SCHEDULER_COMPLETED_TOTAL
				.with_label_values(&[namespace_id.as_str(), "error"])
				.inc();
			tracing::warn!(?ns, %envoy_key, "read-path envoy expire panicked");
		}
	}
}
