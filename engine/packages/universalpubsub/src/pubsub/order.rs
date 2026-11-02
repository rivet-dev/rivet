//! Ordered lanes: requests to one subject on one worker that carry order tickets reach the
//! subscriber in the order their tickets were taken.
//!
//! A caller takes a ticket synchronously at its own ordering point, for example while it
//! processes effects in order, and sends the request with it later from any task. The request
//! waits for its turn before it enters a peer channel's write queue, or before it is dispatched
//! in process, and passes the turn once it is queued or once it gives up. Every request on a lane
//! rides the same channel, which writes frames in queue order on one HTTP/2 stream; the serving
//! side dispatches a channel's requests in the order it reads them, and the subscriber delivers
//! its requests in dispatch order.
//!
//! Order covers delivery, not outcomes. Each request still gets exactly one reply or a timeout
//! and is never replayed. A request that fails before it is sent, or whose caller leaves, passes
//! its turn unsent, so the next request on the lane goes on and the receiver sees a gap it must
//! already handle for any lost request. A channel that ends fails every request still pending on
//! it, so requests sent on its replacement can overtake only requests whose callers have already
//! been told their outcome is unknown.

use std::{
	collections::BTreeSet,
	hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hash, Hasher},
	sync::Arc,
};

use anyhow::Result;
use tokio::sync::Notify;

use super::WorkerId;

/// Lanes are pruned only when this many exist, so a steady set of destinations never churns.
const PRUNE_LANES_AT: usize = 1024;

/// Every ordered lane this process sends on, by destination worker and subject.
pub(crate) struct OrderLanes {
	lanes: scc::HashMap<(WorkerId, Vec<u8>), Arc<Lane>>,
}

/// A borrowed lane key. It hashes exactly as the owned `(WorkerId, Vec<u8>)` key does, so an
/// existing lane is found without copying the subject.
struct LaneKeyRef<'a>(&'a WorkerId, &'a [u8]);

impl Hash for LaneKeyRef<'_> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.0.hash(state);
		self.1.hash(state);
	}
}

impl scc::Equivalent<(WorkerId, Vec<u8>)> for LaneKeyRef<'_> {
	fn equivalent(&self, key: &(WorkerId, Vec<u8>)) -> bool {
		*self.0 == key.0 && self.1 == key.1.as_slice()
	}
}

/// One lane: its destination, its turns and the channel every request on it rides.
struct Lane {
	worker: WorkerId,
	subject: Box<[u8]>,
	turns: Turns,
	/// Picks the lane's channel, so every request on it shares one write queue.
	route: u64,
}

/// One request's place on an ordered lane. Dropping it passes the turn, so a caller that stops
/// waiting never holds up the requests behind it.
pub struct OrderTicket {
	lane: Arc<Lane>,
	turn: Turn,
}

impl OrderLanes {
	pub(crate) fn new() -> Self {
		Self {
			lanes: scc::HashMap::new(),
		}
	}

	/// Takes the next ticket on the lane to `subject` at `worker`. Every access to this map is
	/// synchronous, because a ticket is taken at the caller's ordering point and must not yield;
	/// no async code waits on it.
	pub(crate) fn ticket(&self, worker: &WorkerId, subject: &[u8]) -> OrderTicket {
		if self.lanes.len() >= PRUNE_LANES_AT {
			// A lane only the map holds has no ticket outstanding, so dropping it loses no turn.
			self.lanes
				.retain_sync(|_, lane| Arc::strong_count(lane) > 1);
		}
		let existing = self
			.lanes
			.read_sync(&LaneKeyRef(worker, subject), |_, lane| lane.clone());
		let lane = match existing {
			Some(lane) => lane,
			None => self
				.lanes
				.entry_sync((worker.clone(), subject.to_vec()))
				.or_insert_with(|| {
					Arc::new(Lane {
						worker: worker.clone(),
						subject: subject.into(),
						turns: Turns::new(),
						route: BuildHasherDefault::<DefaultHasher>::default().hash_one(subject),
					})
				})
				.get()
				.clone(),
		};
		let turn = lane.turns.take();
		OrderTicket { lane, turn }
	}
}

impl OrderTicket {
	/// Whether this ticket belongs to the lane a request to `subject` at `worker` rides.
	pub(crate) fn is_for(&self, worker: &WorkerId, subject: &[u8]) -> bool {
		self.lane.worker == *worker && *self.lane.subject == *subject
	}

	/// The channel route shared by every request on this lane.
	pub(crate) fn route(&self) -> u64 {
		self.lane.route
	}

	/// Waits until every earlier ticket on the lane has passed, or the deadline passes.
	pub(crate) async fn turn(
		&self,
		clock: &dyn rivet_env::Clock,
		deadline_boot_ms: u64,
	) -> Result<()> {
		self.turn.wait(clock, deadline_boot_ms).await
	}

	/// Passes the turn to the next ticket. A request calls this once it is queued or has given
	/// up; dropping the ticket does the same.
	pub(crate) fn pass(&mut self) {
		self.turn.pass();
	}
}

/// A sequence of turns taken in one order and run in that order, from any task. An ordered lane
/// keeps one per destination; a receiver that must handle messages in arrival order can keep one
/// per key, for example per actor, so unrelated keys never wait for each other.
///
/// Take a turn synchronously at the ordering point with [`Turns::take`], wait for it later with
/// [`Turn::wait`], and pass it once the ordered step is done. A turn dropped before it comes up
/// is skipped, so a caller that gives up never holds up the turns behind it.
pub struct Turns {
	inner: Arc<TurnsInner>,
}

struct TurnsInner {
	state: parking_lot::Mutex<TurnsState>,
	turn: Notify,
}

/// Turns are numbered in the order they were taken; `next_turn` is the one that may run now.
struct TurnsState {
	next_ticket: u64,
	next_turn: u64,
	/// Turns that passed before they came up, because their holder gave up while waiting.
	passed_early: BTreeSet<u64>,
}

/// One place in a [`Turns`] sequence. Dropping it passes the turn.
pub struct Turn {
	turns: Arc<TurnsInner>,
	seq: u64,
	passed: bool,
}

impl Turns {
	pub fn new() -> Self {
		Self {
			inner: Arc::new(TurnsInner {
				state: parking_lot::Mutex::new(TurnsState {
					next_ticket: 0,
					next_turn: 0,
					passed_early: BTreeSet::new(),
				}),
				turn: Notify::new(),
			}),
		}
	}

	/// Takes the next turn. Never yields, so it can be called at a synchronous ordering point.
	pub fn take(&self) -> Turn {
		let seq = {
			let mut state = self.inner.state.lock();
			let seq = state.next_ticket;
			state.next_ticket += 1;
			seq
		};
		Turn {
			turns: self.inner.clone(),
			seq,
			passed: false,
		}
	}
}

impl Turns {
	/// Whether every turn taken so far has passed, so a fresh sequence would order the same.
	pub fn is_idle(&self) -> bool {
		let state = self.inner.state.lock();
		state.next_turn == state.next_ticket
	}
}

impl Default for Turns {
	fn default() -> Self {
		Self::new()
	}
}

impl Turn {
	/// Waits until every earlier turn has passed, or the deadline passes.
	pub async fn wait(&self, clock: &dyn rivet_env::Clock, deadline_boot_ms: u64) -> Result<()> {
		loop {
			// Arm the notification before checking, so a pass between the check and the wait
			// still wakes this turn.
			let notified = self.turns.turn.notified();
			tokio::pin!(notified);
			notified.as_mut().enable();
			if self.turns.state.lock().next_turn == self.seq {
				return Ok(());
			}
			rivet_env::timeout_at(clock, deadline_boot_ms, notified)
				.await
				.map_err(super::request::timeout_error)?;
		}
	}

	/// Passes the turn to the next one. Dropping the turn does the same.
	pub fn pass(&mut self) {
		if self.passed {
			return;
		}
		self.passed = true;
		{
			let mut state = self.turns.state.lock();
			if state.next_turn == self.seq {
				state.next_turn += 1;
				loop {
					let next = state.next_turn;
					if !state.passed_early.remove(&next) {
						break;
					}
					state.next_turn += 1;
				}
			} else {
				state.passed_early.insert(self.seq);
			}
		}
		self.turns.turn.notify_waiters();
	}
}

impl Drop for Turn {
	fn drop(&mut self) {
		self.pass();
	}
}
