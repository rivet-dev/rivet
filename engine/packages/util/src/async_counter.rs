use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

use tokio::sync::Notify;
use tokio::time::{Instant, timeout_at};

pub struct AsyncCounter {
	value: AtomicUsize,
	zero_notify: Notify,
	zero_observers: Mutex<Vec<Weak<Notify>>>,
}

impl AsyncCounter {
	pub fn new() -> Self {
		Self {
			value: AtomicUsize::new(0),
			zero_notify: Notify::new(),
			zero_observers: Mutex::new(Vec::new()),
		}
	}

	pub fn register_zero_notify(&self, notify: &Arc<Notify>) {
		self
			.zero_observers
			.lock()
			.expect("async counter observer lock poisoned")
			.push(Arc::downgrade(notify));
	}

	pub fn increment(&self) {
		self.value.fetch_add(1, Ordering::Relaxed);
	}

	pub fn decrement(&self) {
		let prev = self.value.fetch_sub(1, Ordering::AcqRel);
		debug_assert!(prev > 0, "AsyncCounter decrement below zero");
		if prev == 1 {
			self.zero_notify.notify_waiters();
			let mut observers = self
				.zero_observers
				.lock()
				.expect("async counter observer lock poisoned");
			observers.retain(|observer| {
				let Some(notify) = observer.upgrade() else {
					return false;
				};
				notify.notify_waiters();
				true
			});
		}
	}

	pub fn load(&self) -> usize {
		self.value.load(Ordering::Acquire)
	}

	pub async fn wait_zero(&self, deadline: Instant) -> bool {
		loop {
			let notified = self.zero_notify.notified();
			tokio::pin!(notified);
			notified.as_mut().enable();

			if self.value.load(Ordering::Acquire) == 0 {
				return true;
			}

			if timeout_at(deadline, notified).await.is_err() {
				return false;
			}
		}
	}
}

impl Default for AsyncCounter {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use std::panic::catch_unwind;
	use std::sync::Arc;
	use std::time::Duration;

	use tokio::sync::Notify;
	use tokio::task::yield_now;
	use tokio::time::{Instant, advance};

	use super::AsyncCounter;

	#[tokio::test(start_paused = true)]
	async fn waiter_wakes_on_decrement_to_zero() {
		let counter = Arc::new(AsyncCounter::new());
		counter.increment();

		let waiter = tokio::spawn({
			let counter = counter.clone();
			async move { counter.wait_zero(Instant::now() + Duration::from_secs(1)).await }
		});

		yield_now().await;
		counter.decrement();
		advance(Duration::from_millis(1)).await;

		assert!(waiter.await.expect("waiter should join"));
	}

	#[tokio::test(start_paused = true)]
	async fn waiter_race_with_immediate_zero_transition_is_safe() {
		let counter = Arc::new(AsyncCounter::new());
		counter.increment();

		let waiter = tokio::spawn({
			let counter = counter.clone();
			async move { counter.wait_zero(Instant::now() + Duration::from_secs(1)).await }
		});

		counter.decrement();
		advance(Duration::from_millis(1)).await;

		assert!(waiter.await.expect("waiter should join"));
	}

	#[tokio::test(start_paused = true)]
	async fn multiple_waiters_all_wake_on_zero_transition() {
		let counter = Arc::new(AsyncCounter::new());
		counter.increment();

		let waiters = (0..4)
			.map(|_| {
				let counter = counter.clone();
				tokio::spawn(async move {
					counter.wait_zero(Instant::now() + Duration::from_secs(1)).await
				})
			})
			.collect::<Vec<_>>();

		yield_now().await;
		counter.decrement();
		advance(Duration::from_millis(1)).await;

		for waiter in waiters {
			assert!(waiter.await.expect("waiter should join"));
		}
	}

	#[tokio::test(start_paused = true)]
	async fn zero_observers_wake_on_zero_transition() {
		let counter = Arc::new(AsyncCounter::new());
		let notify = Arc::new(Notify::new());
		counter.register_zero_notify(&notify);
		counter.increment();

		let waiter = tokio::spawn({
			let notify = notify.clone();
			async move {
				let notified = notify.notified();
				tokio::pin!(notified);
				notified.as_mut().enable();
				notified.await;
			}
		});

		yield_now().await;
		counter.decrement();
		advance(Duration::from_millis(1)).await;

		waiter.await.expect("observer waiter should join");
	}

	#[tokio::test(start_paused = true)]
	async fn non_zero_decrement_does_not_wake_waiter() {
		let counter = Arc::new(AsyncCounter::new());
		counter.increment();
		counter.increment();

		let waiter = tokio::spawn({
			let counter = counter.clone();
			async move { counter.wait_zero(Instant::now() + Duration::from_secs(1)).await }
		});

		yield_now().await;
		counter.decrement();
		advance(Duration::from_millis(1)).await;

		assert!(
			!waiter.is_finished(),
			"waiter should stay blocked until the counter actually reaches zero"
		);

		counter.decrement();
		advance(Duration::from_millis(1)).await;

		assert!(waiter.await.expect("waiter should join"));
	}

	#[tokio::test(start_paused = true)]
	async fn deadline_returns_false_when_counter_stays_non_zero() {
		let counter = Arc::new(AsyncCounter::new());
		counter.increment();

		let waiter = tokio::spawn({
			let counter = counter.clone();
			async move { counter.wait_zero(Instant::now() + Duration::from_millis(5)).await }
		});

		advance(Duration::from_millis(5)).await;

		assert!(!waiter.await.expect("waiter should join"));
	}

	#[cfg(debug_assertions)]
	#[test]
	fn decrement_below_zero_panics_in_debug() {
		let counter = AsyncCounter::new();
		let result = catch_unwind(|| counter.decrement());
		assert!(result.is_err(), "below-zero decrement should panic in debug");
	}
}
