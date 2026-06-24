use std::{
	sync::{
		Arc,
		atomic::{AtomicI64, Ordering},
	},
	time::Duration,
};

use deadpool_postgres::Pool;
use tokio::sync::{Notify, watch};

use super::listener::PgListener;

/// The singleton row id of `udb_lease`.
pub const LEASE_ID: i32 = 1;

/// How often the follower refreshes its cached lease row (epoch, leader channel, watermark) as a
/// backstop to the `udb_watermark` NOTIFY. A stale-but-older watermark only widens the conflict
/// window, so this can be loose.
const LEASE_REFRESH_INTERVAL: Duration = Duration::from_millis(500);

/// Channel a follower NOTIFYs (and the leader LISTENs) to wake the leader's drain loop.
pub fn commit_channel(node_id: &str) -> String {
	format!("udb_commit_{node_id}")
}

/// Channel the leader NOTIFYs (and a follower LISTENs) to deliver a commit result.
pub fn reply_channel(node_id: &str) -> String {
	format!("udb_reply_{node_id}")
}

/// Channel the leader NOTIFYs on every watermark advance; all nodes LISTEN.
pub const WATERMARK_CHANNEL: &str = "udb_watermark";

/// Cached view of the current leader lease, as seen by a follower.
#[derive(Clone, Debug)]
pub struct LeaseInfo {
	pub epoch: i64,
	/// Node id of the current leader, used to build its commit channel.
	pub leader_addr: String,
}

/// Process-wide state shared by the follower transaction tasks and the leader resolver. Every node
/// is both a follower (it submits its own commits) and a candidate leader.
pub struct PostgresShared {
	pub pool: Pool,
	/// Unique per-process id used to name this node's NOTIFY channels.
	pub node_id: String,
	pub listener: PgListener,
	/// Highest durable commit version (`udb_lease.durable_version`); the follower read version.
	durable_version: AtomicI64,
	/// Pinged whenever `durable_version` advances.
	watermark_notify: Notify,
	lease_tx: watch::Sender<Option<LeaseInfo>>,
	lease_rx: watch::Receiver<Option<LeaseInfo>>,
}

impl PostgresShared {
	pub fn new(pool: Pool, node_id: String, listener: PgListener) -> Arc<Self> {
		let (lease_tx, lease_rx) = watch::channel(None);
		let shared = Arc::new(Self {
			pool,
			node_id,
			listener,
			durable_version: AtomicI64::new(0),
			watermark_notify: Notify::new(),
			lease_tx,
			lease_rx,
		});

		tokio::spawn(Self::cache_refresh_task(shared.clone()));

		shared
	}

	/// The cached follower read version (`durable_version`).
	pub fn read_version(&self) -> i64 {
		self.durable_version.load(Ordering::SeqCst)
	}

	/// Advance the cached watermark monotonically and wake any waiters.
	pub fn advance_durable_version(&self, version: i64) {
		let prev = self.durable_version.fetch_max(version, Ordering::SeqCst);
		if version > prev {
			self.watermark_notify.notify_waiters();
		}
	}

	/// Current cached lease, if known.
	pub fn current_lease(&self) -> Option<LeaseInfo> {
		self.lease_rx.borrow().clone()
	}

	/// Publish a freshly observed/elected lease into the cache.
	pub fn set_lease(&self, lease: LeaseInfo) {
		let _ = self.lease_tx.send(Some(lease));
	}

	/// Background task: keep `durable_version` and the cached lease fresh via the `udb_watermark`
	/// NOTIFY plus a periodic poll of `udb_lease`.
	async fn cache_refresh_task(shared: Arc<Self>) {
		let mut watermark_rx = shared.listener.listen(WATERMARK_CHANNEL).await;
		let mut interval = tokio::time::interval(LEASE_REFRESH_INTERVAL);
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

		loop {
			tokio::select! {
				notify = watermark_rx.recv() => {
					match notify {
						Ok(payload) => {
							if let Ok(version) = payload.parse::<i64>() {
								shared.advance_durable_version(version);
							}
						}
						Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
						Err(tokio::sync::broadcast::error::RecvError::Closed) => {
							// Re-subscribe; the listener recreates the channel on reconnect.
							watermark_rx = shared.listener.listen(WATERMARK_CHANNEL).await;
						}
					}
				}
				_ = interval.tick() => {
					shared.refresh_lease_row().await;
				}
			}
		}
	}

	async fn refresh_lease_row(&self) {
		let conn = match self.pool.get().await {
			Ok(conn) => conn,
			Err(err) => {
				tracing::debug!(?err, "failed to get connection for lease refresh");
				return;
			}
		};

		let row = conn
			.query_opt(
				"SELECT epoch, leader_addr, durable_version FROM udb_lease WHERE id = $1",
				&[&LEASE_ID],
			)
			.await;

		match row {
			Ok(Some(row)) => {
				let epoch: i64 = row.get(0);
				let leader_addr: String = row.get(1);
				let durable_version: i64 = row.get(2);
				self.advance_durable_version(durable_version);
				self.set_lease(LeaseInfo { epoch, leader_addr });
			}
			Ok(None) => {
				// No lease row yet; no leader elected.
			}
			Err(err) => {
				tracing::debug!(?err, "failed to refresh lease row");
			}
		}
	}
}
