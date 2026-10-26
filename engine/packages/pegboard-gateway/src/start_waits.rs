//! Requests on this gateway process waiting for actors to start.
//!
//! The process keeps at most one wait per actor. The first request for a not-running actor
//! creates it, subscribes to the wait's own notice subject and registers one start waiter in the
//! actor state machine; later requests for the same actor join it and register nothing. The
//! actor's host sends start notices to that subject as the actor moves along its start path, and
//! every joined request wakes. A wait id is a per-process counter: the worker id is fresh per
//! process start, so `(worker_id, wait_id)` never repeats.
//!
//! The waiting requests receive the notices themselves: whichever request is waiting holds the
//! subscription and publishes what it reads to the others, and the next one takes over when it
//! leaves. The subscription ends with the last request. A notice is lossy. Each request pairs
//! its wait with its own fallback poll, so a lost notice only delays that start.
use anyhow::{Context, Result, ensure};
use pegboard::{keys::ups::ActorStartNoticeSubject, wire};
use rivet_util::Id;
use std::sync::{
	Arc,
	atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use tokio::sync::{Mutex, watch};
use vbare::Versioned;

/// What a start notice reported for an actor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
	/// `epoch`'s start command was just sent to `allocation`, and the actor's waiter stays
	/// stored, so the running notice follows.
	Starting {
		epoch: u32,
		allocation: wire::ActorAllocation,
	},
	/// The actor runs `epoch` on `allocation`. The actor took its waiters.
	Running {
		epoch: u32,
		allocation: wire::ActorAllocation,
	},
	/// Another datacenter holds the actor's key; `reservation_id` carries its label. The actor
	/// took its waiters and is being destroyed.
	EpoxyAlreadyReserved { reservation_id: Id },
	/// The actor is being destroyed and took its waiters. A getOrCreate creates it again.
	Destroyed,
}

impl Notice {
	pub(crate) fn decode(kind: &wire::ActorStartNoticeKind) -> Result<Self> {
		Ok(match kind {
			wire::ActorStartNoticeKind::ActorStartNoticeStarting(starting) => Notice::Starting {
				epoch: starting.epoch,
				allocation: starting.allocation.clone(),
			},
			wire::ActorStartNoticeKind::ActorStartNoticeRunning(running) => Notice::Running {
				epoch: running.epoch,
				allocation: running.allocation.clone(),
			},
			wire::ActorStartNoticeKind::ActorStartNoticeEpoxyAlreadyReserved(reserved) => {
				Notice::EpoxyAlreadyReserved {
					reservation_id: Id::from_slice(&reserved.reservation_id)?,
				}
			}
			wire::ActorStartNoticeKind::ActorStartNoticeDestroyed => Notice::Destroyed,
		})
	}
}

/// The wait's notice subscription, held by whichever request is receiving. `None` once the
/// subscription ended or was never made; the requests then wait on their polls alone.
type Subscription = Arc<Mutex<Option<universalpubsub::RequestSubscriber>>>;

struct Wait {
	wait_id: u64,
	notices: watch::Sender<Option<Notice>>,
	subscription: Subscription,
	joined: usize,
}

/// The result of completing one wait id named by a notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completion {
	Completed,
	/// The wait already finished or was replaced. Not an error.
	UnknownWait,
}

impl Completion {
	pub fn label(self) -> &'static str {
		match self {
			Completion::Completed => "completed",
			Completion::UnknownWait => "unknown_wait",
		}
	}
}

/// The process's start waits, owned by the gateway router state.
pub struct StartWaits {
	env: Arc<dyn rivet_env::Env>,
	/// Where waits subscribe for their notices. `None` leaves every wait to its fallback poll.
	ups: Option<universalpubsub::PubSub>,
	capacity: usize,
	waits: scc::HashMap<Id, Wait>,
	len: AtomicUsize,
	next_wait_id: AtomicU64,
	/// Whether the near-capacity warning fired since usage was last below 90 %.
	near_full: AtomicBool,
	/// Test hook: when set, notices are acknowledged without completing waits.
	drop_notices: AtomicBool,
}

impl StartWaits {
	pub fn new(
		env: Arc<dyn rivet_env::Env>,
		ups: Option<universalpubsub::PubSub>,
		capacity: usize,
	) -> Arc<Self> {
		Arc::new(Self {
			env,
			ups,
			capacity: capacity.max(1),
			waits: scc::HashMap::new(),
			len: AtomicUsize::new(0),
			next_wait_id: AtomicU64::new(1),
			near_full: AtomicBool::new(false),
			drop_notices: AtomicBool::new(false),
		})
	}

	/// Joins the actor's wait, creating it when none exists. Returns `None` when the process holds
	/// its capacity of waits, so the caller waits on its fallback poll alone.
	pub async fn join(self: &Arc<Self>, actor_id: Id) -> Option<WaitGuard> {
		let entry = self.waits.entry_async(actor_id).await;
		let (wait_id, created, notices, subscription) = match entry {
			scc::hash_map::Entry::Occupied(mut occupied) => {
				let wait = occupied.get_mut();
				wait.joined += 1;
				(
					wait.wait_id,
					false,
					wait.notices.clone(),
					wait.subscription.clone(),
				)
			}
			scc::hash_map::Entry::Vacant(vacant) => {
				if self.len.load(Ordering::Acquire) >= self.capacity {
					drop(vacant);
					self.warn_if_near_full();
					return None;
				}
				let wait_id = self.next_wait_id.fetch_add(1, Ordering::Relaxed);
				let (sender, _receiver) = watch::channel(None);
				let subscription = Arc::new(Mutex::new(self.subscribe(actor_id, wait_id)));
				vacant.insert_entry(Wait {
					wait_id,
					notices: sender.clone(),
					subscription: subscription.clone(),
					joined: 1,
				});
				self.len.fetch_add(1, Ordering::AcqRel);
				self.observe_usage();
				(wait_id, true, sender, subscription)
			}
		};

		let updates = notices.subscribe();
		Some(WaitGuard {
			waits: self.clone(),
			actor_id,
			wait_id,
			created,
			notices,
			updates,
			subscription,
			released: false,
		})
	}

	/// Subscribes a new wait to its notice subject. A wait that cannot subscribe is left to its
	/// fallback poll; the requests in it still finish.
	fn subscribe(&self, actor_id: Id, wait_id: u64) -> Option<universalpubsub::RequestSubscriber> {
		let ups = self.ups.as_ref()?;
		match ups.subscribe_request(ActorStartNoticeSubject::new(wait_id)) {
			Ok(subscriber) => Some(subscriber),
			Err(error) => {
				tracing::debug!(%actor_id, wait_id, ?error, "start notice subscription failed; the wait polls instead");
				None
			}
		}
	}

	/// Completes the actor's wait if it is the one `wait_id` names, for an answer that settled
	/// the wait locally without a notice.
	pub async fn complete(&self, actor_id: Id, wait_id: u64, notice: Notice) -> Completion {
		let Some(wait) = self.waits.get_async(&actor_id).await else {
			return Completion::UnknownWait;
		};
		if wait.wait_id != wait_id {
			return Completion::UnknownWait;
		}
		wait.notices.send_replace(Some(notice));
		Completion::Completed
	}

	/// Number of actors with a wait in this process.
	pub fn len(&self) -> usize {
		self.len.load(Ordering::Acquire)
	}

	/// Test hook: acknowledges every notice without completing its wait.
	#[doc(hidden)]
	pub fn set_drop_notices(&self, drop_notices: bool) {
		self.drop_notices.store(drop_notices, Ordering::Release);
	}

	pub(crate) fn drops_notices(&self) -> bool {
		self.drop_notices.load(Ordering::Acquire)
	}

	/// Leaves the wait; the last request to leave removes it, and with it its subscription.
	async fn release(&self, actor_id: Id, wait_id: u64) {
		let entry = self.waits.entry_async(actor_id).await;
		let scc::hash_map::Entry::Occupied(mut occupied) = entry else {
			return;
		};
		let wait = occupied.get_mut();
		if wait.wait_id != wait_id {
			return;
		}
		wait.joined = wait.joined.saturating_sub(1);
		if wait.joined == 0 {
			let _removed = occupied.remove();
			self.len.fetch_sub(1, Ordering::AcqRel);
			self.observe_usage();
		}
	}

	fn observe_usage(&self) {
		let len = self.len();
		if len.saturating_mul(10) >= self.capacity.saturating_mul(9) {
			self.warn_if_near_full();
		} else {
			self.near_full.store(false, Ordering::Release);
		}
	}

	/// Warns once each time usage reaches 90 % of the capacity.
	fn warn_if_near_full(&self) {
		if !self.near_full.swap(true, Ordering::AcqRel) {
			tracing::warn!(
				len = self.len(),
				capacity = self.capacity,
				"gateway start waits are near capacity; raise pegboard.gateway_start_waits_capacity if many actors start at once"
			);
		}
	}
}

/// One request's membership in an actor's wait. Call [`WaitGuard::finish`] when the request is
/// done; dropping it without that releases it on a background task.
pub struct WaitGuard {
	waits: Arc<StartWaits>,
	actor_id: Id,
	wait_id: u64,
	created: bool,
	notices: watch::Sender<Option<Notice>>,
	updates: watch::Receiver<Option<Notice>>,
	subscription: Subscription,
	released: bool,
}

impl WaitGuard {
	pub fn wait_id(&self) -> u64 {
		self.wait_id
	}

	/// Whether this request created the wait, so it registers the start waiter.
	pub fn created(&self) -> bool {
		self.created
	}

	/// Waits for the next notice this request has not seen. The request that gets the wait's
	/// subscription receives for every request in the wait, publishing each notice to the rest;
	/// a request that leaves mid-wait hands the subscription to the next one. Pends forever if
	/// the wait is gone, which only happens after every guard finished.
	pub async fn notified(&mut self) -> Notice {
		let subscription = self.subscription.clone();
		loop {
			let took = rivet_env::select_biased!("a notice another request already received is taken before the subscription",
				changed = self.updates.changed() => Took::Published(changed),
				subscription = subscription.lock() => Took::Subscription(subscription),
			);
			let mut subscription = match took {
				Took::Published(changed) => match self.unseen(changed) {
					Some(notice) => return notice,
					None => continue,
				},
				Took::Subscription(subscription) => subscription,
			};
			let Some(subscriber) = subscription.as_mut() else {
				// Without a subscription only an answer settled locally can complete the wait.
				drop(subscription);
				return self.published().await;
			};

			loop {
				let read = rivet_env::select_biased!("a notice published by an earlier receiver is taken before reading more",
					changed = self.updates.changed() => Read::Published(changed),
					request = subscriber.next() => Read::Request(request),
				);
				let request = match read {
					Read::Published(changed) => match self.unseen(changed) {
						Some(notice) => return notice,
						None => continue,
					},
					Read::Request(Ok(Some(request))) => request,
					Read::Request(Ok(None)) => break,
					Read::Request(Err(error)) => {
						tracing::debug!(actor_id = %self.actor_id, wait_id = self.wait_id, ?error, "start notice subscription ended; the wait polls instead");
						break;
					}
				};
				match decode(&request, self.actor_id) {
					Ok(notice) => {
						let _ = request.reply(&[]);
						if self.waits.drops_notices() {
							continue;
						}
						self.notices.send_replace(Some(notice.clone()));
						self.updates.mark_unchanged();
						return notice;
					}
					Err(error) => {
						tracing::warn!(
							actor_id = %self.actor_id,
							wait_id = self.wait_id,
							?error,
							"rejecting an invalid actor start notice; the sender is out of date or misrouted"
						);
						let _ = request.fail(&error).await;
					}
				}
			}

			// The subscription ended for good; the remaining requests poll.
			*subscription = None;
			drop(subscription);
			return self.published().await;
		}
	}

	/// The notice `changed` announced, if it is one this request has not returned yet.
	fn unseen(&mut self, changed: Result<(), watch::error::RecvError>) -> Option<Notice> {
		if changed.is_err() {
			return None;
		}
		self.updates.borrow_and_update().clone()
	}

	/// Waits for a notice published by another request or a locally settled answer.
	async fn published(&mut self) -> Notice {
		loop {
			if self.updates.changed().await.is_err() {
				return std::future::pending().await;
			}
			if let Some(notice) = self.updates.borrow_and_update().clone() {
				return notice;
			}
		}
	}

	pub async fn finish(mut self) {
		self.released = true;
		self.waits.release(self.actor_id, self.wait_id).await;
	}
}

impl Drop for WaitGuard {
	fn drop(&mut self) {
		if self.released {
			return;
		}
		let waits = self.waits.clone();
		let actor_id = self.actor_id;
		let wait_id = self.wait_id;
		self.waits.env.spawn(Box::pin(async move {
			waits.release(actor_id, wait_id).await;
		}));
	}
}

/// What `notified` got first: a notice another request published, or the wait's subscription.
enum Took<'a> {
	Published(Result<(), watch::error::RecvError>),
	Subscription(tokio::sync::MutexGuard<'a, Option<universalpubsub::RequestSubscriber>>),
}

/// What the receiving request got first: a published notice, or the next request on the
/// subscription.
enum Read {
	Published(Result<(), watch::error::RecvError>),
	Request(Result<Option<universalpubsub::RequestMessage>>),
}

/// Decodes a notice for the wait of `actor_id`. A notice naming another actor was misrouted.
fn decode(request: &universalpubsub::RequestMessage, actor_id: Id) -> Result<Notice> {
	ensure!(
		request.class() == universalpubsub::RequestClass::ActorStartNotice,
		"start notice sent under the {} class",
		request.class().as_str()
	);
	let notice: wire::ActorStartNotice =
		rivet_data::versioned::ActorStartNotice::deserialize_with_embedded_version(
			&request.payload,
		)
		.context("invalid actor start notice")?;
	let noticed_actor_id = Id::from_slice(&notice.actor_id).context("invalid actor id")?;
	ensure!(
		noticed_actor_id == actor_id,
		"start notice for another actor on this wait's subject"
	);
	Notice::decode(&notice.kind)
}


