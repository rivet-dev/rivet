use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

use tokio::sync::Notify;
use tokio::time::{Instant, timeout_at};

pub struct AsyncCounter {
	value: AtomicUsize,
	zero_notify: Notify,
	zero_observers: Mutex<Vec<Weak<Notify>>>,
	change_observers: Mutex<Vec<Weak<Notify>>>,
	change_callbacks: Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>,
}

impl AsyncCounter {
	pub fn new() -> Self {
		Self {
			value: AtomicUsize::new(0),
			zero_notify: Notify::new(),
			zero_observers: Mutex::new(Vec::new()),
			change_observers: Mutex::new(Vec::new()),
			change_callbacks: Mutex::new(Vec::new()),
		}
	}

	pub fn register_zero_notify(&self, notify: &Arc<Notify>) {
		self.zero_observers
			.lock()
			.expect("async counter observer lock poisoned")
			.push(Arc::downgrade(notify));
	}

	/// Register an observer that is woken on every increment and decrement.
	///
	/// Use this for state that needs to re-evaluate on any counter transition,
	/// not just zero. Observers are held as `Weak<Notify>` so they are pruned
	/// automatically when the `Arc` is dropped.
	pub fn register_change_notify(&self, notify: &Arc<Notify>) {
		self.change_observers
			.lock()
			.expect("async counter observer lock poisoned")
			.push(Arc::downgrade(notify));
	}

	pub fn register_change_callback(&self, callback: Arc<dyn Fn() + Send + Sync>) {
		self.change_callbacks
			.lock()
			.expect("async counter observer lock poisoned")
			.push(callback);
	}

	pub fn increment(&self) {
		self.value.fetch_add(1, Ordering::Relaxed);
		self.notify_change();
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
		self.notify_change();
	}

	fn notify_change(&self) {
		let mut observers = self
			.change_observers
			.lock()
			.expect("async counter observer lock poisoned");
		observers.retain(|observer| {
			let Some(notify) = observer.upgrade() else {
				return false;
			};
			notify.notify_waiters();
			true
		});
		drop(observers);

		let callbacks = self
			.change_callbacks
			.lock()
			.expect("async counter observer lock poisoned")
			.clone();
		for callback in callbacks {
			callback();
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


