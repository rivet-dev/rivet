//! Ordered ingress admission: pending storage may be refused to retain lifecycle
//! reports, whose adjacent batches are combined without discarding checkpoints.
use super::{SocketInbound, p};
use anyhow::{Result, bail};
use parking_lot::Mutex;
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::{Semaphore, mpsc};

struct State {
	items: VecDeque<SocketInbound>,
	closed: bool,
}

struct Shared {
	state: Mutex<State>,
	capacity: usize,
	pressure: Arc<crate::queue::QueuePressure>,
	control_bytes: Arc<Semaphore>,
}

pub(super) struct Sender {
	shared: Arc<Shared>,
	// Wakeups only. Keeping this sender out of Shared preserves last-sender EOF.
	wake: mpsc::Sender<()>,
}

pub(super) struct Receiver {
	shared: Arc<Shared>,
	wake: mpsc::Receiver<()>,
}

pub(super) fn channel(
	capacity: usize,
	pressure: Arc<crate::queue::QueuePressure>,
) -> (Sender, Receiver) {
	let shared = Arc::new(Shared {
		state: Mutex::new(State {
			items: VecDeque::new(),
			closed: false,
		}),
		capacity,
		pressure,
		// A maximum-size storage frame already in dispatch must not consume the
		// memory reserved for lifecycle progress. Each class remains byte bounded.
		control_bytes: Arc::new(Semaphore::new(super::SOCKET_INBOUND_BYTES)),
	});
	let (tx, rx) = mpsc::channel(1);
	(
		Sender {
			shared: shared.clone(),
			wake: tx,
		},
		Receiver { shared, wake: rx },
	)
}

pub(super) fn is_storage(message: &p::ToRivet) -> bool {
	matches!(
		message,
		p::ToRivet::ToRivetKvRequest(_)
			| p::ToRivet::ToRivetSqliteGetPagesRequest(_)
			| p::ToRivet::ToRivetSqliteCommitRequest(_)
			| p::ToRivet::ToRivetSqliteCommitStageBeginRequest(_)
			| p::ToRivet::ToRivetSqliteCommitStageSegmentRequest(_)
			| p::ToRivet::ToRivetSqliteCommitFinalizeRequest(_)
			| p::ToRivet::ToRivetSqliteExecRequest(_)
			| p::ToRivet::ToRivetSqliteExecuteRequest(_)
			| p::ToRivet::ToRivetSqliteExecuteBatchRequest(_)
	)
}

/// Append only when no retained message separates these events. Permit merging
/// keeps accounting constant-sized even for thousands of tiny event batches.
fn push(items: &mut VecDeque<SocketInbound>, mut item: SocketInbound) {
	if let Some(tail) = items.back_mut()
		&& let (p::ToRivet::ToRivetEvents(earlier), p::ToRivet::ToRivetEvents(later)) =
			(&mut tail.message, &mut item.message)
	{
		earlier.append(later);
		tail._bytes.merge(item._bytes);
	} else {
		items.push_back(item);
	}
}

impl Sender {
	pub(super) fn ensure_open(&self) -> Result<()> {
		if self.shared.state.lock().closed {
			bail!("envoy inbound dispatcher closed");
		}
		Ok(())
	}
	pub(super) fn control_bytes(&self) -> &Arc<Semaphore> {
		&self.shared.control_bytes
	}

	/// Returns only storage requests removed before dispatch. Replies happen after
	/// the lock is released; pop and eviction cannot race an executing request.
	pub(super) fn try_send(&self, item: SocketInbound) -> Result<Vec<p::ToRivet>> {
		let storage = is_storage(&item.message);
		let mut refused = Vec::new();
		let depth = {
			let mut state = self.shared.state.lock();
			if state.closed {
				bail!("envoy inbound dispatcher closed");
			}
			let merges_tail = matches!(item.message, p::ToRivet::ToRivetEvents(_))
				&& state
					.items
					.back()
					.is_some_and(|tail| matches!(tail.message, p::ToRivet::ToRivetEvents(_)));
			if state.items.len() == self.shared.capacity && !merges_tail {
				if storage {
					return Ok(vec![item.message]);
				}
				let Some(index) = state
					.items
					.iter()
					.rposition(|pending| is_storage(&pending.message))
				else {
					bail!("envoy lifecycle queue full; reconnect to replay unacknowledged work");
				};
				let SocketInbound { message, _bytes } =
					state.items.remove(index).expect("index found while locked");
				drop(_bytes);
				refused.push(message);
				// Removing storage can make two event batches adjacent. Preserve all
				// checkpoints and their order while reclaiming the redundant frame slot.
				let old = std::mem::take(&mut state.items);
				for pending in old {
					push(&mut state.items, pending);
				}
			}
			push(&mut state.items, item);
			state.items.len()
		};
		// No callbacks or logging under the admission lock.
		self.shared.pressure.observe(
			crate::queue::QueueKind::SocketInbound,
			depth,
			self.shared.capacity,
		);
		match self.wake.try_send(()) {
			Ok(()) | Err(mpsc::error::TrySendError::Full(())) => Ok(refused),
			Err(mpsc::error::TrySendError::Closed(())) => bail!("envoy inbound dispatcher closed"),
		}
	}
}

impl Receiver {
	pub(super) async fn recv(&mut self) -> Option<SocketInbound> {
		loop {
			if let Some(item) = self.shared.state.lock().items.pop_front() {
				return Some(item);
			}
			// Check the deque before EOF: the sender may have queued a final item
			// and dropped while the receiver was draining a previous wakeup.
			self.wake.recv().await?;
		}
	}
}

impl Drop for Receiver {
	fn drop(&mut self) {
		let mut state = self.shared.state.lock();
		state.closed = true;
		state.items.clear();
	}
}
