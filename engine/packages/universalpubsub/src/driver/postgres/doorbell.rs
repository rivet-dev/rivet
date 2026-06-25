use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use deadpool_postgres::Pool;
use tokio::sync::Notify;
use tokio::time::Instant;

/// Number of doorbell shards. A subject maps to a shard via `hash(subject_hash) % K`.
/// Subscribers LISTEN their subject's shard channel; publishers wake the local
/// doorbell task which NOTIFYs the shard.
pub const DOORBELL_SHARD_COUNT: usize = 32;

/// Debounce window. Caps each (process, shard) NOTIFY rate at one per window, which
/// bounds how many backends are woken per shard over time.
const DOORBELL_WINDOW: Duration = Duration::from_millis(5);

/// Returns the NOTIFY channel name for a doorbell shard.
pub fn shard_channel(shard: usize) -> String {
	format!("ups_db_{shard}")
}

/// Returns the doorbell shard for a subject hash.
pub fn shard_for(subject_hash: &str) -> usize {
	use std::hash::{DefaultHasher, Hash, Hasher};
	let mut hasher = DefaultHasher::new();
	subject_hash.hash(&mut hasher);
	(hasher.finish() as usize) % DOORBELL_SHARD_COUNT
}

/// Coalesced, payload-free NOTIFY doorbell.
///
/// Publishers call [`Doorbell::mark_dirty`] after committing a row. A single
/// per-process task drains dirty shards and emits at most one NOTIFY per shard per
/// debounce window using leading-edge fire plus a trailing-edge flush. The doorbell
/// is a latency optimization only. Correctness comes from the table plus the
/// subscriber poll backstop, so a dropped or failed NOTIFY only adds latency.
pub struct Doorbell {
	dirty: [AtomicBool; DOORBELL_SHARD_COUNT],
	notify: Notify,
	pool: Arc<Pool>,
}

impl Doorbell {
	pub fn new(pool: Arc<Pool>) -> Arc<Self> {
		let doorbell = Arc::new(Self {
			dirty: std::array::from_fn(|_| AtomicBool::new(false)),
			notify: Notify::new(),
			pool,
		});

		let task_doorbell = doorbell.clone();
		tokio::spawn(async move { task_doorbell.run().await });

		doorbell
	}

	/// Marks a shard dirty and wakes the doorbell task. Never blocks.
	pub fn mark_dirty(&self, shard: usize) {
		self.dirty[shard].store(true, Ordering::Release);
		self.notify.notify_one();
	}

	async fn run(self: Arc<Self>) {
		// Per-shard timestamp of the last NOTIFY emitted by this process.
		let mut last_notify: [Option<Instant>; DOORBELL_SHARD_COUNT] = [None; DOORBELL_SHARD_COUNT];
		// Per-shard deadline for a pending trailing-edge NOTIFY, if any.
		let mut trailing: [Option<Instant>; DOORBELL_SHARD_COUNT] = [None; DOORBELL_SHARD_COUNT];

		loop {
			// Arm on the next pending trailing deadline so the trailing edge fires
			// even with no further publishes. Wait on the notify permit otherwise.
			let next_deadline = trailing.iter().filter_map(|x| *x).min();
			match next_deadline {
				Some(deadline) => {
					tokio::select! {
						_ = self.notify.notified() => {}
						_ = tokio::time::sleep_until(deadline) => {}
					}
				}
				None => {
					self.notify.notified().await;
				}
			}

			let now = Instant::now();
			for shard in 0..DOORBELL_SHARD_COUNT {
				let is_dirty = self.dirty[shard].swap(false, Ordering::AcqRel);
				if is_dirty {
					match last_notify[shard] {
						Some(last) if now.duration_since(last) < DOORBELL_WINDOW => {
							// Within the window. Defer to a trailing-edge NOTIFY at
							// window end so at most one NOTIFY fires per shard per W.
							if trailing[shard].is_none() {
								trailing[shard] = Some(last + DOORBELL_WINDOW);
							}
						}
						_ => {
							// Leading edge. Fire immediately for low idle latency.
							self.notify_shard(shard).await;
							last_notify[shard] = Some(now);
							trailing[shard] = None;
						}
					}
				}

				// Flush a trailing-edge NOTIFY whose window has elapsed.
				if let Some(deadline) = trailing[shard] {
					if now >= deadline {
						self.notify_shard(shard).await;
						last_notify[shard] = Some(now);
						trailing[shard] = None;
					}
				}
			}
		}
	}

	async fn notify_shard(&self, shard: usize) {
		let channel = shard_channel(shard);
		match self.pool.get().await {
			Ok(conn) => {
				// Payload-free doorbell. The payload lives in the table.
				if let Err(err) = conn.execute("SELECT pg_notify($1, '')", &[&channel]).await {
					tracing::warn!(?err, %channel, "failed to emit doorbell notify");
				}
			}
			Err(err) => {
				tracing::warn!(?err, %channel, "failed to get connection for doorbell notify");
			}
		}
	}
}
