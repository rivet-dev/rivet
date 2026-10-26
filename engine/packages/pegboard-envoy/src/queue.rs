//! Bounded queues on one Envoy connection: the socket writer, each gateway tunnel and each
//! actor's three lanes. Every queue has a configured capacity, a process-wide depth gauge per kind
//! and a rate-limited warning when one queue nears its capacity.
use rivet_env::Clock;
use std::sync::{
	Arc,
	atomic::{AtomicU64, Ordering},
};
use tokio::sync::mpsc;

/// A queue is near its capacity once this share of its slots is in use, in percent.
const PRESSURE_PERCENT: usize = 90;

/// One warning per connection per window, so a full queue under sustained load does not flood
/// the log.
const PRESSURE_WARN_INTERVAL_MS: u64 = 30_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QueueKind {
	Socket,
	Tunnel,
	ActorLane,
	ActorInbound,
	/// An actor's KV and SQLite requests, apart from its events.
	ActorStorage,
}

impl QueueKind {
	pub(crate) fn label(self) -> &'static str {
		match self {
			QueueKind::Socket => "socket",
			QueueKind::Tunnel => "tunnel",
			QueueKind::ActorLane => "actor_lane",
			QueueKind::ActorInbound => "actor_inbound",
			QueueKind::ActorStorage => "actor_storage",
		}
	}

	/// The config key that sets this queue's capacity, for operator-facing warnings.
	fn config_key(self) -> &'static str {
		match self {
			QueueKind::Socket => "pegboard.envoy_socket_queue_capacity",
			QueueKind::Tunnel => "pegboard.envoy_tunnel_queue_capacity",
			QueueKind::ActorLane => "pegboard.envoy_actor_lane_capacity",
			QueueKind::ActorInbound | QueueKind::ActorStorage => {
				"pegboard.envoy_actor_inbound_capacity"
			}
		}
	}
}

/// Rate-limits the near-capacity warning across every queue of one connection.
pub(crate) struct QueuePressure {
	clock: Arc<dyn Clock>,
	/// Boot time of the last warning plus one, or 0 before the first.
	last_warn_boot_ms: AtomicU64,
}

impl QueuePressure {
	pub(crate) fn new(clock: Arc<dyn Clock>) -> Self {
		QueuePressure {
			clock,
			last_warn_boot_ms: AtomicU64::new(0),
		}
	}

	/// Logs a warning when the queue is near its capacity and this connection has not warned
	/// within the window.
	fn observe(&self, kind: QueueKind, in_use: usize, capacity: usize) {
		if !near_capacity(in_use, capacity) {
			return;
		}
		let Ok(now) = self.clock.observe() else {
			return;
		};
		if self.claim_warning(now.boot_ms) {
			tracing::warn!(
				queue = kind.label(),
				in_use,
				capacity,
				config_key = kind.config_key(),
				"envoy connection queue is near its capacity; the envoy or a downstream store is slower than its traffic, or the capacity is too small"
			);
		}
	}

	/// Whether a warning at `now_boot_ms` is outside the window of the last one. Claims the
	/// window when it is.
	pub(crate) fn claim_warning(&self, now_boot_ms: u64) -> bool {
		let last = self.last_warn_boot_ms.load(Ordering::Relaxed);
		let due = last == 0 || now_boot_ms.saturating_add(1) >= last + PRESSURE_WARN_INTERVAL_MS;
		if !due {
			return false;
		}
		self.last_warn_boot_ms
			.compare_exchange(
				last,
				now_boot_ms.saturating_add(1),
				Ordering::Relaxed,
				Ordering::Relaxed,
			)
			.is_ok()
	}
}

pub(crate) fn near_capacity(in_use: usize, capacity: usize) -> bool {
	in_use.saturating_mul(100) >= capacity.saturating_mul(PRESSURE_PERCENT)
}

/// Creates a bounded queue of `kind` with `capacity` slots.
pub(crate) fn channel<T>(
	kind: QueueKind,
	capacity: usize,
	pressure: Arc<QueuePressure>,
) -> (Sender<T>, Receiver<T>) {
	let (tx, rx) = mpsc::channel(capacity);
	(Sender { tx, kind, pressure }, Receiver { rx })
}

struct Queued<T> {
	value: T,
}

pub(crate) struct Sender<T> {
	tx: mpsc::Sender<Queued<T>>,
	kind: QueueKind,
	pressure: Arc<QueuePressure>,
}

impl<T> Clone for Sender<T> {
	fn clone(&self) -> Self {
		Sender {
			tx: self.tx.clone(),
			kind: self.kind,
			pressure: self.pressure.clone(),
		}
	}
}

impl<T> Sender<T> {
	fn queued(&self, value: T) -> Queued<T> {
		Queued { value }
	}

	/// Waits for a free slot, then queues `value`.
	pub(crate) async fn send(&self, value: impl Into<T>) -> Result<(), mpsc::error::SendError<T>> {
		self.tx
			.send(self.queued(value.into()))
			.await
			.map_err(|mpsc::error::SendError(queued)| mpsc::error::SendError(queued.value))?;

		let capacity = self.tx.max_capacity();
		let in_use = capacity.saturating_sub(self.tx.capacity());
		self.pressure.observe(self.kind, in_use, capacity);
		Ok(())
	}

	/// Queues `value` only if a slot is free now. A full queue hands the value back, so a caller
	/// that must not wait (the socket reader) can refuse it instead.
	pub(crate) fn try_send(&self, value: T) -> Result<(), mpsc::error::TrySendError<T>> {
		self.tx
			.try_send(self.queued(value))
			.map_err(|error| match error {
				mpsc::error::TrySendError::Full(queued) => {
					mpsc::error::TrySendError::Full(queued.value)
				}
				mpsc::error::TrySendError::Closed(queued) => {
					mpsc::error::TrySendError::Closed(queued.value)
				}
			})?;

		let capacity = self.tx.max_capacity();
		let in_use = capacity.saturating_sub(self.tx.capacity());
		self.pressure.observe(self.kind, in_use, capacity);
		Ok(())
	}
}

pub(crate) struct Receiver<T> {
	rx: mpsc::Receiver<Queued<T>>,
}

impl<T> Receiver<T> {
	pub(crate) async fn recv(&mut self) -> Option<T> {
		let queued = self.rx.recv().await?;
		Some(queued.value)
	}
}


