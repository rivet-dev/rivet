use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use std::{
	future::Future,
	sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::{Context, Result, anyhow};
use futures_util::FutureExt;
use rivet_metrics::GaugeGuardExt;
use rivet_tracing_utils::CustomInstrumentExt;

use crate::{
	driver::{DatabaseDriverHandle, Erased},
	metrics,
	throttle::{
		ThrottleClass, ThrottleConfig, ThrottleDecision, ThrottleKind, ThrottleState, ThrottleTxn,
	},
	transaction::{RetryableTransaction, Transaction},
};

/// Returns the simulated latency duration read from UDB_SIMULATED_LATENCY_MS at startup.
fn simulated_latency() -> Option<Duration> {
	static LATENCY: OnceLock<Option<Duration>> = OnceLock::new();
	*LATENCY.get_or_init(|| {
		let ms: u64 = std::env::var("UDB_SIMULATED_LATENCY_MS")
			.ok()?
			.parse()
			.ok()?;
		if ms == 0 {
			return None;
		}
		tracing::debug!(latency_ms = ms, "udb simulated latency enabled");
		Some(Duration::from_millis(ms))
	})
}

#[derive(Clone)]
pub struct Database {
	driver: DatabaseDriverHandle,
	throttle: Arc<ThrottleState>,
}

impl Database {
	pub fn new(driver: DatabaseDriverHandle) -> Self {
		tokio::spawn(last_tx_age_heartbeat());

		Database {
			driver,
			throttle: Arc::new(ThrottleState::new(None)),
		}
	}

	/// Enables cluster-wide byte-rate throttling for transactions that opt in with
	/// [`Transaction::charge_throttle`]. Without this every axis is unthrottled and charging is inert.
	///
	/// This starts the background flusher, without which nothing this process charges would ever reach
	/// the shared counters. The flusher holds a clone of this database, so it lives as long as the
	/// process; a caller that wants to drive [`Database::flush_throttle`] itself passes a config built
	/// with `without_flusher`.
	pub fn with_throttle(mut self, config: ThrottleConfig) -> Self {
		self.throttle = Arc::new(ThrottleState::new(Some(config)));

		if let Some(period) = self.throttle.flush_interval() {
			let db = self.clone();
			tokio::spawn(async move {
				let mut interval = tokio::time::interval(period);
				// A flush that runs long must not be followed by a burst of catch-up ticks: the next
				// one would find the same books it just drained.
				interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
				// The first tick resolves immediately, and there is nothing to flush at startup.
				interval.tick().await;

				loop {
					interval.tick().await;

					if let Err(err) = db.flush_throttle().await {
						tracing::warn!(
							?err,
							"failed to flush throttle charges, retrying on the next tick"
						);
					}
				}
			});
		}

		self
	}

	/// Folds this process's accumulated throttle bytes into the shared counters and refreshes its
	/// estimate. Runs on a timer in production; a test drives it directly so a charge reaches the
	/// counters at a known point.
	pub async fn flush_throttle(&self) -> Result<()> {
		self.throttle.flush(self).await
	}

	/// Samples a throttle's admission gate without a transaction, so a caller can back off before
	/// opening one it would only discard.
	pub fn check_throttle(
		&self,
		name: &'static str,
		kind: ThrottleKind,
		class: ThrottleClass,
	) -> ThrottleDecision {
		self.throttle.check(name, kind, class)
	}

	/// Run a closure with automatic retry logic and a name.
	#[tracing::instrument(skip_all)]
	pub async fn txn<'a, F, Fut, T>(&'a self, name: &'static str, closure: F) -> Result<T>
	where
		F: Fn(RetryableTransaction) -> Fut + Send + Sync,
		Fut: Future<Output = Result<T>> + Send,
		T: Send + 'a + 'static,
	{
		if let Some(delay) = simulated_latency() {
			tokio::time::sleep(delay).await;
		}

		let start = Instant::now();
		let attempts = AtomicUsize::new(0);
		metrics::TRANSACTION_TOTAL.with_label_values(&[name]).inc();
		let _pending_guard = metrics::TRANSACTION_PENDING
			.with_label_values(&[name])
			.inc_guard();

		// Shared across every attempt: reads charge per attempt, writes charge once if the transaction
		// commits, and only the loop out here knows whether it did.
		let throttle = Arc::new(ThrottleTxn::new(self.throttle.clone(), true));

		let closure = &closure;
		let res = self
			.driver
			.run(Box::new(|tx| {
				attempts.fetch_add(1, Ordering::AcqRel);

				let tx = tx.with_name(name).with_throttle(throttle.clone());
				let guard = ThrottleAttemptGuard::new(tx.deref().clone());
				async move {
					let _guard = guard;
					closure(tx).await.map(|value| Box::new(value) as Erased)
				}
				.custom_instrument(tracing::debug_span!("txn_attempt"))
				.boxed()
			}))
			.await
			.and_then(|res| {
				res.downcast::<T>()
					.map(|x| *x)
					.map_err(|_| anyhow!("failed to downcast `run` return type"))
			})
			.context("transaction failed");

		let final_attempts = attempts.load(Ordering::Acquire);
		let duration = start.elapsed();
		metrics::TRANSACTION_ATTEMPTS
			.with_label_values(&[name])
			.observe(final_attempts as f64);
		metrics::TRANSACTION_DURATION
			.with_label_values(&[name])
			.observe(duration.as_secs_f64());

		if res.is_ok() {
			// The transaction committed, so what its final attempt wrote is real load. Attempts that
			// were discarded wrote nothing and are not charged.
			throttle.commit();

			// Update the global "last successful tx" timestamp consumed by the heartbeat ticker.
			// Stores epoch ms; ticker observes age as a gauge.
			let epoch_ms = std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.map(|d| d.as_millis() as u64)
				.unwrap_or(0);
			metrics::LAST_TX_COMPLETED_EPOCH_MS.store(epoch_ms, Ordering::Release);
		}

		res
	}

	/// The underlying driver, for building a second [`Database`] over the same storage. Tests use this
	/// to stand up a second throttle bookkeeper against shared counters.
	#[doc(hidden)]
	pub fn driver_handle(&self) -> DatabaseDriverHandle {
		self.driver.clone()
	}

	/// Creates a new txn instance.
	///
	/// The result can check throttles but cannot charge them: charging a write needs a commit outcome,
	/// which only [`Database::txn`]'s retry loop observes.
	pub fn create_txn(&self) -> Result<Transaction> {
		Ok(self
			.driver
			.create_txn()?
			.with_throttle(Arc::new(ThrottleTxn::new(self.throttle.clone(), false))))
	}

	pub fn txn_retry_limit(&self, limit: i32) -> Result<()> {
		self.driver.txn_retry_limit(limit)
	}

	/// Create a consistent point-in-time snapshot of the database at the given path.
	pub fn checkpoint(&self, path: &Path) -> Result<()> {
		self.driver.checkpoint(path)
	}

	/// Gracefully release process-wide driver resources before shutdown.
	pub async fn shutdown(&self) {
		// Flush before the driver goes away, so a graceful shutdown does not drop the charges this
		// process accumulated since the last tick.
		if let Err(err) = self.flush_throttle().await {
			tracing::warn!(?err, "failed to flush throttle charges during shutdown");
		}

		self.driver.shutdown().await;
	}
}

/// Closes out one transaction attempt: reports what it read and remembers what it wrote.
///
/// This is a guard rather than a line after the `await` because an attempt whose future is dropped
/// (an outer timeout, a cancelled task) has still charged everything it read. Reporting from here
/// keeps the metric and the observer agreeing with the counter on every path, including that one.
struct ThrottleAttemptGuard {
	tx: Transaction,
}

impl ThrottleAttemptGuard {
	fn new(tx: Transaction) -> Self {
		ThrottleAttemptGuard { tx }
	}
}

impl Drop for ThrottleAttemptGuard {
	fn drop(&mut self) {
		if let Some(throttle) = self.tx.throttle() {
			throttle.end_attempt(self.tx.read_bytes(), self.tx.throttled_write_bytes());
		}
	}
}

/// Pod-local heartbeat exposing seconds since the most recent successful UDB tx.
/// All actors on this pod slowing down at once shows up here as a rising gauge,
/// even if per-request outliers are too noisy to spot in aggregate.
async fn last_tx_age_heartbeat() {
	loop {
		tokio::time::sleep(Duration::from_secs(1)).await;

		let last_ms = metrics::LAST_TX_COMPLETED_EPOCH_MS.load(Ordering::Acquire);
		if last_ms == 0 {
			// No successful tx recorded yet; leave the gauge at its default.
			continue;
		}
		let now_ms = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|d| d.as_millis() as u64)
			.unwrap_or(last_ms);
		let age_s = now_ms.saturating_sub(last_ms) as f64 / 1000.0;
		metrics::LAST_TX_COMPLETED_AGE_SECONDS.set(age_s);
	}
}
