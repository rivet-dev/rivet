//! Writes the heartbeats and actor counts of every envoy this process holds to their pools' ring
//! index, one [`pegboard::envoy_ring::PoolHeartbeats`] command per pool per flush.
//!
//! Connections register with their pool's writer once the fleet reads the ring index. A pool's
//! flush task runs only while it has registered connections. Each flush pulls every connection's
//! latest pong and actor count and sends those that changed since the last acknowledged write, so a
//! window with many pongs or actor starts costs one write per envoy. The written heartbeat is also
//! the node's pool admission, and an acknowledged write renews it. Every heartbeat carries the
//! envoy's whole state, so a lost or refused flush is repaired by the next one. A connect flushes
//! its pool before it returns, so a new envoy is allocatable without waiting for the interval.
//!
//! A connection leaves its writer before its disconnect removes its rows, and leaving waits for
//! a flush in flight, so no flush can write a connection back after its removal.

use super::{Connection, live_pool_heartbeat};
use anyhow::Result;
use pegboard::pool_wire;
use rivet_engine_context::EngineContext;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use uuid::Uuid;

/// Process-wide registry of pool writers. Building it starts nothing; a pool's task starts with
/// its first connection.
pub(crate) struct IndexWriter {
	pools: scc::HashMap<(String, String), Arc<PoolWriter>>,
	tasks: rivet_env::TaskScope,
}

struct PoolWriter {
	namespace: String,
	pool_name: String,
	connections: scc::HashMap<Uuid, Arc<Connection>>,
	/// Held while a flush collects and writes, so leaving waits for a flush in flight.
	flushing: tokio::sync::Mutex<()>,
	/// Wakes the flush task before its next tick.
	flush_now: tokio::sync::Notify,
}

/// A connection's membership in its pool's writer.
pub(super) struct RingMembership {
	/// Whether the connection registered with its pool's writer.
	joined: std::sync::atomic::AtomicBool,
	/// The actor count of the last acknowledged write, or `u32::MAX` before the first.
	written_count: std::sync::atomic::AtomicU32,
	/// Notified when a write found that a newer connection of the envoy owns its state.
	pub(super) superseded: tokio::sync::Notify,
	/// Notified when a write found the pool deleted.
	pub(super) admission_lost: tokio::sync::Notify,
}

impl Default for RingMembership {
	fn default() -> Self {
		Self {
			joined: std::sync::atomic::AtomicBool::new(false),
			written_count: std::sync::atomic::AtomicU32::new(u32::MAX),
			superseded: tokio::sync::Notify::new(),
			admission_lost: tokio::sync::Notify::new(),
		}
	}
}

impl IndexWriter {
	pub(crate) fn new(ctx: &EngineContext) -> Self {
		Self {
			pools: scc::HashMap::new(),
			tasks: ctx.env().task_scope(),
		}
	}

	/// Registers `conn` with its pool's writer, starting the pool's flush task when it has none.
	/// Idempotent. The first registration of a connection wakes its pool's flush.
	pub(super) async fn join(self: &Arc<Self>, ctx: &EngineContext, conn: &Arc<Connection>) {
		if conn.ring.joined.swap(true, Ordering::AcqRel) {
			return;
		}
		let key = (conn.scope.namespace.clone(), conn.scope.pool_name.clone());
		// The connection is added while the registry entry is held, so a pool that is retiring
		// either sees it or is replaced by a new one.
		let pool = match self.pools.entry_async(key).await {
			scc::hash_map::Entry::Occupied(entry) => {
				let pool = entry.get().clone();
				let _ = pool
					.connections
					.insert_async(conn.connection_id, conn.clone())
					.await;
				pool
			}
			scc::hash_map::Entry::Vacant(entry) => {
				let pool = Arc::new(PoolWriter {
					namespace: conn.scope.namespace.clone(),
					pool_name: conn.scope.pool_name.clone(),
					connections: scc::HashMap::new(),
					flushing: tokio::sync::Mutex::new(()),
					flush_now: tokio::sync::Notify::new(),
				});
				let _ = pool
					.connections
					.insert_async(conn.connection_id, conn.clone())
					.await;
				entry.insert_entry(pool.clone());
				let ctx = ctx.clone();
				let writer = self.clone();
				let task_pool = pool.clone();
				self.tasks.spawn(async move {
					if let Err(error) = run_pool(&ctx, &writer, &task_pool).await {
						tracing::warn!(
							?error,
							namespace = %task_pool.namespace,
							pool_name = %task_pool.pool_name,
							"node index writer stopped; its nodes age out of allocation until they reconnect"
						);
					}
				});
				pool
			}
		};
		pool.flush_now.notify_one();
	}

	/// Unregisters `conn`, after any flush in flight that may have collected it finishes.
	pub(super) async fn leave(&self, conn: &Connection) {
		if !conn.ring.joined.load(Ordering::Acquire) {
			return;
		}
		let key = (conn.scope.namespace.clone(), conn.scope.pool_name.clone());
		let Some(pool) = self.pools.read_async(&key, |_, pool| pool.clone()).await else {
			return;
		};
		let flushing = pool.flushing.lock().await;
		pool.connections.remove_async(&conn.connection_id).await;
		drop(flushing);
		// Wake the task so it can end once its pool has no connections.
		pool.flush_now.notify_one();
	}

	/// Flushes `conn`'s pool now and returns once the write landed, so a connect can return an
	/// allocatable envoy.
	pub(super) async fn flush_for(&self, ctx: &EngineContext, conn: &Connection) -> Result<()> {
		let key = (conn.scope.namespace.clone(), conn.scope.pool_name.clone());
		let Some(pool) = self.pools.read_async(&key, |_, pool| pool.clone()).await else {
			return Ok(());
		};
		flush(ctx, &pool).await
	}

	/// Stops every pool's flush task and waits for them. Called after every connection drained.
	pub(crate) async fn shutdown(&self) {
		self.tasks.shutdown().await;
	}

	/// Removes `pool` from the registry when it has no connections, under the registry entry's
	/// lock so a concurrent join either lands first and keeps the pool or creates a new one.
	/// Returns whether it was removed.
	async fn retire(&self, pool: &Arc<PoolWriter>) -> bool {
		let key = (pool.namespace.clone(), pool.pool_name.clone());
		self.pools
			.remove_if_async(&key, |current| {
				Arc::ptr_eq(current, pool) && current.connections.is_empty()
			})
			.await
			.is_some()
	}
}

/// Flushes one pool on every tick of the flush interval and whenever woken, until the pool has
/// no connections.
async fn run_pool(ctx: &EngineContext, writer: &IndexWriter, pool: &Arc<PoolWriter>) -> Result<()> {
	let period_ms = ctx.config().pegboard().envoy_index_flush_interval();
	let mut interval = rivet_env::interval(ctx.env().clock(), period_ms);
	loop {
		rivet_env::select!( ctx.env().rng("envoy.index_writer");
			tick = interval.tick() => { tick?; },
			_ = pool.flush_now.notified() => {},
		);
		if pool.connections.is_empty() {
			if writer.retire(pool).await {
				return Ok(());
			}
			continue;
		}
		if let Err(error) = flush(ctx, pool).await {
			tracing::debug!(
				?error,
				namespace = %pool.namespace,
				pool_name = %pool.pool_name,
				"node index flush failed; the next flush writes the latest state again"
			);
		}
	}
}

/// Writes the heartbeat of every registered connection whose heartbeat or actor count changed
/// since its last acknowledged write, and hands each its reply.
async fn flush(ctx: &EngineContext, pool: &PoolWriter) -> Result<()> {
	let _flushing = pool.flushing.lock().await;
	let mut members = Vec::new();
	pool.connections
		.iter_async(|_, conn| {
			members.push(conn.clone());
			true
		})
		.await;

	let mut sent = Vec::new();
	let mut heartbeats = Vec::new();
	for conn in members {
		if let Some(heartbeat) = conn.ring_heartbeat(ctx).await? {
			sent.push((conn, heartbeat.clone()));
			heartbeats.push(heartbeat);
		}
	}
	if heartbeats.is_empty() {
		return Ok(());
	}

	let replies =
		pegboard::envoy_ring::send_heartbeats(ctx, &pool.namespace, &pool.pool_name, heartbeats)
			.await?;
	for ((conn, heartbeat), reply) in sent.into_iter().zip(replies) {
		conn.apply_ring_reply(ctx, &heartbeat, reply).await?;
	}
	Ok(())
}

impl Connection {
	/// This connection's heartbeat for its pool's ring index, or `None` when it has nothing new to
	/// write: it is not connected, its last pong is outside the eligibility window, or neither the
	/// pong nor its actor count changed since the last acknowledged write. The heartbeat is the
	/// connection's admission, so a stopping envoy keeps writing it, marked stopping so allocation
	/// skips it.
	async fn ring_heartbeat(
		&self,
		ctx: &EngineContext,
	) -> Result<Option<pool_wire::PoolHeartbeat>> {
		let record = self.record.lock().await;
		if !record.connected {
			return Ok(None);
		}
		let heartbeat_unix_ms = self.last_pong_unix_ms.load(Ordering::Acquire);
		if live_pool_heartbeat(heartbeat_unix_ms, ctx.env().clock().observe()?.unix_ms).is_none() {
			return Ok(None);
		}
		let actor_count = u32::try_from(self.actor_handle_budget.in_use()).unwrap_or(u32::MAX);
		if heartbeat_unix_ms <= self.last_heartbeat_unix_ms.load(Ordering::Acquire)
			&& actor_count == self.ring.written_count.load(Ordering::Acquire)
		{
			return Ok(None);
		}
		Ok(Some(pool_wire::PoolHeartbeat {
			envoy_key: self.envoy_key.clone().into(),
			connection_epoch: record.entry.connection_epoch,
			worker_id: ctx.ups()?.worker_id().as_bytes().to_vec().into(),
			connection_id: self.connection_id.as_bytes().to_vec().into(),
			version: record.entry.version,
			names: record.entry.names.clone(),
			heartbeat_unix_ms,
			actor_count,
			stopping: record.entry.stopping,
		}))
	}

	/// Applies the reply to a written heartbeat: a superseded connection is told to close, and
	/// otherwise the write is recorded and the drain observes the pool's newest live version.
	async fn apply_ring_reply(
		&self,
		ctx: &EngineContext,
		heartbeat: &pool_wire::PoolHeartbeat,
		reply: pool_wire::PoolHeartbeatReply,
	) -> Result<()> {
		if reply.superseded {
			self.ring.superseded.notify_one();
			return Ok(());
		}
		if reply.admission_lost {
			self.ring.admission_lost.notify_one();
			return Ok(());
		}
		// The acknowledged heartbeat renewed the pool admission. A late reply carries an older
		// pong, so it confirms no more than that pong and expires with it.
		self.pool_heartbeat_unix_ms
			.fetch_max(heartbeat.heartbeat_unix_ms, Ordering::AcqRel);
		if live_pool_heartbeat(
			self.pool_heartbeat_unix_ms.load(Ordering::Acquire),
			ctx.env().clock().observe()?.unix_ms,
		)
		.is_none()
		{
			return Ok(());
		}
		self.last_heartbeat_unix_ms
			.fetch_max(heartbeat.heartbeat_unix_ms, Ordering::AcqRel);
		self.ring
			.written_count
			.store(heartbeat.actor_count, Ordering::Release);
		if let Some(config) = self.pool_config.get() {
			self.drain
				.observe(
					&self.scope,
					&self.envoy_key,
					heartbeat.version,
					reply.newest_live_version,
					config.drain_on_version_upgrade,
					ctx.env().clock().observe()?.boot_ms,
				)
				.await;
		}
		Ok(())
	}
}
