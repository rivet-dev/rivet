//! Subscriptions to `Topology::watch`, coalesced to one state machine watch per process.
//!
//! Every subscriber of one process in this worker shares one subscription, whatever service
//! or tablet asked: the first `watch` creates it and sends `WatchRequest`, later ones clone
//! its handle, and the last handle to drop removes it and sends `UnwatchRequest`. So a
//! process held by a thousand tablets' volumes still costs one watch and, for a stateless
//! worker, one heartbeat read per poll interval.
use crate::topology_machine::{event::Event, watch::WatchedProcess};
use crate::worker::{WorkerId, WorkerInstanceId};
use anyhow::{Context, Result};
use rivet_env::TaskScope;
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use std::sync::{
	Arc, OnceLock, Weak,
	atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use tokio::sync::{mpsc, watch};

/// Resolves once its worker process is gone: the lease expired, the process released it, or
/// a newer process of the worker claimed it. Clones share one subscription. A watch exists
/// for low-urgency cleanup, such as unloading the volumes a crashed holder left open. It is
/// detected minutes late on purpose and is never the evidence that makes a fence safe or a
/// decision correct; acting on it must stay correct even if the process were alive.
#[derive(Clone)]
pub struct WatchHandle {
	subscription: Arc<Subscription>,
}

impl WatchHandle {
	/// Waits until the process is gone. Never resolves after this worker's topology stops,
	/// since a stopped topology no longer observes anything.
	pub async fn gone(&self) {
		let mut gone = self.subscription.gone.clone();
		if gone.wait_for(|gone| *gone).await.is_err() {
			std::future::pending::<()>().await;
		}
	}

	pub fn is_gone(&self) -> bool {
		*self.subscription.gone.borrow()
	}

	pub fn process(&self) -> (&WorkerId, WorkerInstanceId) {
		let process = &self.subscription.process;
		(&process.worker_id, process.instance_id)
	}
}

/// One subscription shared by every handle of a process.
struct Subscription {
	process: WatchedProcess,
	id: u64,
	gone: watch::Receiver<bool>,
	registry: Weak<WatchRegistry>,
	/// Counts this process on `topology_watches` while any handle holds it.
	_watched: IntGaugeGuard,
}

impl Drop for Subscription {
	fn drop(&mut self) {
		let Some(registry) = self.registry.upgrade() else {
			return;
		};
		registry.release(self.process.clone(), self.id);
	}
}

struct Slot {
	id: u64,
	gone: watch::Sender<bool>,
	subscription: Weak<Subscription>,
}

/// Owned by `Topology`; shared with its executor, which resolves fired watches.
pub(crate) struct WatchRegistry {
	slots: scc::HashMap<WatchedProcess, Slot>,
	next_id: AtomicU64,
	/// Slots in `slots`, counted on insert and removal so the cap check never scans the map.
	watched: AtomicUsize,
	/// Set while new watches are refused at the cap, so the warning logs once per episode.
	refusing: AtomicBool,
	inbox: OnceLock<mpsc::WeakSender<Event>>,
	tasks: Weak<TaskScope>,
	capacity: usize,
	_capacity_metric: IntGaugeGuard,
}

impl WatchRegistry {
	pub(crate) fn new(capacity: usize, tasks: Weak<TaskScope>) -> Self {
		Self {
			slots: scc::HashMap::new(),
			next_id: AtomicU64::new(0),
			watched: AtomicUsize::new(0),
			refusing: AtomicBool::new(false),
			inbox: OnceLock::new(),
			tasks,
			capacity,
			_capacity_metric: crate::metrics::WATCH_CAPACITY.add_guard(capacity as i64),
		}
	}

	pub(crate) fn bind(&self, inbox: mpsc::WeakSender<Event>) -> Result<()> {
		self.inbox
			.set(inbox)
			.map_err(|_| anyhow::anyhow!("duplicate topology startup"))
	}

	/// Returns the shared subscription of `process`, creating it when no handle holds one.
	///
	/// A new subscription is published only together with its `WatchRequest`: both happen
	/// under the map entry with no await between them, so no caller can share a subscription
	/// the state machine never heard of, and a cancelled caller leaves nothing behind.
	pub(crate) async fn watch(self: &Arc<Self>, process: WatchedProcess) -> Result<WatchHandle> {
		let inbox = self
			.inbox
			.get()
			.and_then(mpsc::WeakSender::upgrade)
			.context("topology not running")?;
		let entry = self.slots.entry_async(process.clone()).await;
		let existing = match &entry {
			scc::hash_map::Entry::Occupied(slot) => slot.get().subscription.upgrade(),
			scc::hash_map::Entry::Vacant(_) => None,
		};
		if let Some(subscription) = existing {
			return Ok(WatchHandle { subscription });
		}

		// A slot whose last handle dropped is replaced in place and keeps its count.
		let new_process = matches!(entry, scc::hash_map::Entry::Vacant(_));
		let watched = self.watched.load(Ordering::Acquire);
		if new_process && watched >= self.capacity {
			crate::metrics::WATCH_REJECTED.inc();
			if !self.refusing.swap(true, Ordering::AcqRel) {
				tracing::warn!(
					capacity = self.capacity,
					"topology watch capacity reached; cleanup after the processes refused waits for a slot"
				);
			}
			anyhow::bail!("topology watch capacity reached");
		}

		let id = self.next_id.fetch_add(1, Ordering::Relaxed);
		inbox
			.try_send(Event::WatchRequest {
				process: process.clone(),
				subscription: id,
			})
			.context("topology inbox full or stopped")?;

		let (sender, receiver) = watch::channel(false);
		let subscription = Arc::new(Subscription {
			process,
			id,
			gone: receiver,
			registry: Arc::downgrade(self),
			_watched: crate::metrics::WATCHES.inc_guard(),
		});
		let slot = Slot {
			id,
			gone: sender,
			subscription: Arc::downgrade(&subscription),
		};
		match entry {
			scc::hash_map::Entry::Occupied(mut occupied) => {
				*occupied.get_mut() = slot;
			}
			scc::hash_map::Entry::Vacant(vacant) => {
				vacant.insert_entry(slot);
				self.refusing.store(false, Ordering::Release);
				let watched = self.watched.fetch_add(1, Ordering::AcqRel) + 1;
				if watched == self.capacity - self.capacity / 5 {
					tracing::warn!(
						watched,
						capacity = self.capacity,
						"topology watches near capacity"
					);
				}
			}
		}
		Ok(WatchHandle { subscription })
	}

	/// Resolves the handles of subscription `id` of `process`, if it still exists.
	pub(crate) async fn fire(&self, process: &WatchedProcess, id: u64) {
		self.slots
			.read_async(process, |_, slot| {
				if slot.id == id {
					slot.gone.send_replace(true);
				}
			})
			.await;
	}

	/// Removes subscription `id` after its last handle dropped and tells the state machine.
	/// Runs from `Drop`, so it spawns: the map is shared with async callers.
	fn release(self: Arc<Self>, process: WatchedProcess, id: u64) {
		let Some(tasks) = self.tasks.upgrade() else {
			return;
		};
		tasks.spawn(async move {
			let removed = self
				.slots
				.remove_if_async(&process, |slot| slot.id == id)
				.await;
			if removed.is_some() {
				self.watched.fetch_sub(1, Ordering::AcqRel);
			}
			let Some(inbox) = self.inbox.get().and_then(mpsc::WeakSender::upgrade) else {
				return;
			};
			let _ = inbox
				.send(Event::UnwatchRequest {
					process,
					subscription: id,
				})
				.await;
		});
	}
}
