//! One deadline queue shared by many runners, built on sharded `rivet_state_machine::IndexedDeadlineQueue`s.
//!
//! A runner normally keeps its own sleep for its earliest alarm. A process that holds many
//! mostly quiet state machines, such as one volume client per held actor, instead registers each
//! runner here: the queue holds each runner's earliest alarm, and one driver task sleeps until
//! the earliest of them and wakes exactly the runners that are due. The runner still owns its
//! alarms and their order; the shared queue only says when to look at them, so the alarm
//! contract is unchanged: a runner delivers its due alarms before any input.

use rivet_env::{ClockError, Env};
use rivet_state_machine::IndexedDeadlineQueue;
use std::sync::{
	Arc,
	atomic::{AtomicU64, Ordering},
};
use tokio::sync::{Notify, watch};

/// A cloneable handle to one shared deadline queue. Construction starts nothing; the owner runs
/// [`SharedTimers::run`] for as long as any registered runner may hold an alarm, and stops it
/// only after those runners stopped.
#[derive(Clone)]
pub struct SharedTimers {
	inner: Arc<Inner>,
}

impl Default for SharedTimers {
	fn default() -> Self {
		Self {
			inner: Arc::new(Inner {
				shards: (0..SHARDS).map(|_| Default::default()).collect(),
				changed: Notify::new(),
				next_id: AtomicU64::new(0),
			}),
		}
	}
}

/// Independently locked parts of the queue. Every runner that re-arms takes its shard's lock,
/// and a process holds one runner per open volume, so one lock for all of them is contended
/// on every worker thread.
const SHARDS: usize = 16;

struct Inner {
	// Forced-sync: a slot cancels its entry from `Drop`. Every critical section is a few map
	// operations and never awaits.
	shards: Box<[parking_lot::Mutex<IndexedDeadlineQueue<Entry>>]>,
	// Some shard's earliest deadline moved earlier, so the driver must sleep less.
	changed: Notify,
	next_id: AtomicU64,
}

/// One runner's queue key. It carries the runner's wake, so the driver needs no second map to
/// find it; order and equality use only the ID.
#[derive(Clone)]
struct Entry {
	id: u64,
	notify: Arc<Notify>,
}

impl PartialEq for Entry {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}

impl Eq for Entry {}

impl PartialOrd for Entry {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Entry {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id.cmp(&other.id)
	}
}

impl SharedTimers {
	pub fn new() -> Self {
		Self::default()
	}

	/// Wakes each registered runner when its earliest alarm is due, until `stop` is set or its
	/// sender is dropped. A clock failure ends the driver with that error, and the owner treats
	/// it like any other clock failure.
	pub async fn run(
		&self,
		env: Arc<dyn Env>,
		mut stop: watch::Receiver<bool>,
	) -> Result<(), ClockError> {
		let clock = env.clock();
		loop {
			if *stop.borrow() {
				return Ok(());
			}
			// Arm the change notification before reading the heads, so a registration that
			// moves a head earlier after this read still wakes the driver.
			let changed = self.inner.changed.notified();
			tokio::pin!(changed);
			changed.as_mut().enable();
			let next_at = self
				.inner
				.shards
				.iter()
				.filter_map(|shard| shard.lock().next_at())
				.min();
			let due = async {
				match next_at {
					Some(at) => clock.sleep_until(at).await,
					None => std::future::pending().await,
				}
			};
			rivet_env::select_biased!("stop, then a moved head, then the earliest deadline",
				result = stop.changed() => {
					if result.is_err() {
						return Ok(());
					}
					continue;
				},
				_ = &mut changed => continue,
				result = due => result?,
			);

			let now = clock.observe()?.boot_ms;
			for shard in &self.inner.shards {
				let due = shard.lock().pop_due(now, usize::MAX);
				for (_, entry) in due {
					entry.notify.notify_one();
				}
			}
		}
	}

	/// Registers one runner. The slot is the runner's only handle to the queue, and dropping it
	/// removes the runner's entry.
	pub(crate) fn register(&self) -> TimerSlot {
		let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
		TimerSlot {
			entry: Entry {
				id,
				notify: Arc::new(Notify::new()),
			},
			inner: self.inner.clone(),
			scheduled: None,
		}
	}

	/// The number of runners with a queued alarm.
	pub fn queued(&self) -> usize {
		self.inner
			.shards
			.iter()
			.map(|shard| shard.lock().len())
			.sum()
	}
}

/// One runner's entry in a [`SharedTimers`] queue: the deadline of its earliest alarm.
pub(crate) struct TimerSlot {
	entry: Entry,
	inner: Arc<Inner>,
	/// The deadline this slot last queued. The driver removes an entry when it fires, so after a
	/// wake this may name an entry that is gone; [`TimerSlot::fired`] forgets it.
	scheduled: Option<u64>,
}

impl TimerSlot {
	fn shard(&self) -> &parking_lot::Mutex<IndexedDeadlineQueue<Entry>> {
		&self.inner.shards[(self.entry.id % SHARDS as u64) as usize]
	}

	/// Queues the runner's earliest alarm at `at`, or removes its entry for `None`.
	pub(crate) fn set(&mut self, at: Option<u64>) {
		if at == self.scheduled {
			return;
		}
		let mut queue = self.shard().lock();
		let head = queue.next_at();
		match at {
			Some(at) => queue.schedule(self.entry.clone(), at),
			None => {
				queue.cancel(&self.entry);
			}
		}
		drop(queue);
		self.scheduled = at;
		// The driver sleeps until the earliest head of all shards, which is no later than this
		// shard's head, so only a deadline before this shard's head can move it earlier.
		if let Some(at) = at
			&& head.is_none_or(|head| at < head)
		{
			self.inner.changed.notify_one();
		}
	}

	/// The driver woke this runner and removed its entry.
	pub(crate) fn fired(&mut self) {
		self.scheduled = None;
	}

	/// Resolves when the driver wakes this runner. A wake may be stale; the runner rechecks its
	/// alarms against the clock.
	pub(crate) fn wake(&self) -> impl std::future::Future<Output = ()> + Send + 'static {
		self.entry.notify.clone().notified_owned()
	}
}

impl Drop for TimerSlot {
	fn drop(&mut self) {
		self.shard().lock().cancel(&self.entry);
	}
}
