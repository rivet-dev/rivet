use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime};
use futures_util::future::poll_fn;
use rivet_postgres_util::build_tls_config;
use rivet_util::throttle::Backoff;
use scc::HashMap;
use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, broadcast};
use tokio_postgres::AsyncMessage;
use tokio_postgres_rustls::MakeRustlsConnect;
use tracing::Instrument;
use uuid::Uuid;

use crate::driver::{PubSubDriver, SubscriberDriver, SubscriberDriverHandle};
use crate::metrics;
use crate::pubsub::DriverOutput;

mod doorbell;

use doorbell::{Doorbell, shard_channel, shard_for};

/// The transport is the table, not the NOTIFY payload, so there is no per-message
/// size cap from the 8000-byte NOTIFY limit. Match the NATS ceiling so chunking
/// behaves identically across drivers.
pub const POSTGRES_MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// Poll backstop interval. Every subscriber reads its table on this interval
/// regardless of doorbell wakeups. This is the correctness floor that makes delivery
/// independent of any NOTIFY arriving.
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Idle-in-transaction timeout applied to the LISTEN connection. A wedged listener
/// holding a transaction open would otherwise fill the shared notify queue and fail
/// NOTIFY cluster-wide. Bounding it keeps a stuck listener degrading to added latency
/// rather than a cluster outage.
const LISTEN_IDLE_IN_TRANSACTION_TIMEOUT_MS: i64 = 30_000;

const QUEUE_SUB_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
/// How long a queue subscriber's heartbeat must be within to be considered active.
const QUEUE_SUB_TTL_SECS: i64 = 30;

/// How often to GC expired broadcast messages.
const MESSAGE_GC_INTERVAL: Duration = Duration::from_secs(5);
/// Max age before a broadcast message row is garbage collected. Must exceed the poll
/// interval plus the reconnect gap. A subscriber that falls behind this misses
/// messages, matching NATS-core at-most-once semantics for slow consumers.
const MESSAGE_MAX_AGE_SECS: i64 = 10;

/// How often to GC orphaned queue messages.
const QUEUE_MESSAGE_GC_INTERVAL: Duration = Duration::from_secs(300);
/// Max age before an unconsumed queue message is garbage collected.
const QUEUE_MESSAGE_MAX_AGE_SECS: i64 = 3600;

#[derive(Clone)]
pub struct PostgresDriver {
	pool: Arc<Pool>,
	client: Arc<Mutex<Option<tokio_postgres::Client>>>,
	/// Wakeup channels keyed by doorbell shard channel name. Shared by broadcast and
	/// queue subscribers whose subjects map to the same shard. Carries empty wakeups
	/// only; payload lives in the table.
	shard_subscriptions: Arc<HashMap<String, broadcast::Sender<()>>>,
	doorbell: Arc<Doorbell>,
	client_ready: tokio::sync::watch::Receiver<bool>,
}

impl PostgresDriver {
	#[tracing::instrument(skip(conn_str))]
	pub async fn connect(
		conn_str: String,
		ssl_root_cert_path: Option<PathBuf>,
		ssl_client_cert_path: Option<PathBuf>,
		ssl_client_key_path: Option<PathBuf>,
	) -> Result<Self> {
		// Create deadpool config from connection string
		let mut config = Config::new();
		config.url = Some(conn_str.clone());
		config.pool = Some(PoolConfig {
			max_size: 64,
			..Default::default()
		});
		config.manager = Some(ManagerConfig {
			recycling_method: RecyclingMethod::Fast,
		});

		// Create the pool
		tracing::debug!("creating postgres pool");

		// Build TLS configuration with optional custom certificates
		let tls_config = build_tls_config(
			ssl_root_cert_path.as_ref(),
			ssl_client_cert_path.as_ref(),
			ssl_client_key_path.as_ref(),
		)?;

		let tls = MakeRustlsConnect::new(tls_config);

		let pool = config
			.create_pool(Some(Runtime::Tokio1), tls)
			.context("failed to create postgres pool")?;
		tracing::debug!("postgres pool created successfully");

		let pool = Arc::new(pool);
		let shard_subscriptions: Arc<HashMap<String, broadcast::Sender<()>>> =
			Arc::new(HashMap::new());
		let client: Arc<Mutex<Option<tokio_postgres::Client>>> = Arc::new(Mutex::new(None));

		// Create channel for client ready notifications
		let (ready_tx, client_ready) = tokio::sync::watch::channel(false);

		// Spawn connection lifecycle task
		tokio::spawn(Self::spawn_connection_lifecycle(
			conn_str.clone(),
			shard_subscriptions.clone(),
			client.clone(),
			ready_tx,
			ssl_root_cert_path.clone(),
			ssl_client_cert_path.clone(),
			ssl_client_key_path.clone(),
		));

		let doorbell = Doorbell::new(pool.clone());

		let driver = Self {
			pool,
			client,
			shard_subscriptions,
			doorbell,
			client_ready,
		};

		// Wait for initial connection to be established
		driver.wait_for_client().await?;

		// Create tables eagerly so they exist before any publish or subscribe.
		{
			let conn = driver
				.pool
				.get()
				.await
				.context("failed to get connection for table creation")?;
			conn.batch_execute(
				// Broadcast transport table. UNLOGGED gives at-most-once across a
				// crash, matching NATS-core semantics, and avoids WAL fsync on every
				// publish. The real subject is stored so receivers can verify it and
				// reject DefaultHasher subject-hash collisions.
				"CREATE UNLOGGED TABLE IF NOT EXISTS ups_messages ( \
				     id BIGSERIAL PRIMARY KEY, \
				     subject_hash TEXT NOT NULL, \
				     subject TEXT NOT NULL, \
				     payload BYTEA NOT NULL, \
				     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW() \
				 ); \
				 CREATE INDEX IF NOT EXISTS ups_messages_subject_id \
				     ON ups_messages (subject_hash, id); \
				 CREATE TABLE IF NOT EXISTS ups_queue_subs ( \
				     id TEXT PRIMARY KEY, \
				     subject_hash TEXT NOT NULL, \
				     queue_hash TEXT NOT NULL, \
				     heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT NOW() \
				 ); \
				 CREATE INDEX IF NOT EXISTS ups_queue_subs_subject_queue \
				     ON ups_queue_subs (subject_hash, queue_hash); \
				 CREATE UNLOGGED TABLE IF NOT EXISTS ups_queue_messages ( \
				     id BIGSERIAL PRIMARY KEY, \
				     subject_hash TEXT NOT NULL, \
				     queue_hash TEXT NOT NULL, \
				     payload BYTEA NOT NULL, \
				     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW() \
				 ); \
				 CREATE INDEX IF NOT EXISTS ups_queue_messages_idx \
				     ON ups_queue_messages (subject_hash, queue_hash, id);",
			)
			.await
			.context("failed to create tables")?;
			tracing::debug!("tables ready");
		}

		// Spawn GC task for expired broadcast messages
		let message_gc_driver = driver.clone();
		tokio::spawn(async move {
			let mut interval = tokio::time::interval(MESSAGE_GC_INTERVAL);
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			loop {
				interval.tick().await;
				if let Ok(conn) = message_gc_driver.pool.get().await {
					let result = conn
						.execute(
							"DELETE FROM ups_messages \
							 WHERE created_at < NOW() - ($1::bigint * INTERVAL '1 second')",
							&[&MESSAGE_MAX_AGE_SECS],
						)
						.await;
					if let Err(e) = result {
						tracing::warn!(?e, "failed to gc broadcast messages");
					}
				}
			}
		});

		// Spawn GC task for orphaned queue messages
		let gc_driver = driver.clone();
		tokio::spawn(async move {
			let mut interval = tokio::time::interval(QUEUE_MESSAGE_GC_INTERVAL);
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			loop {
				interval.tick().await;
				if let Ok(conn) = gc_driver.pool.get().await {
					let result = conn
						.execute(
							"DELETE FROM ups_queue_messages \
							 WHERE created_at < NOW() - ($1::bigint * INTERVAL '1 second')",
							&[&QUEUE_MESSAGE_MAX_AGE_SECS],
						)
						.await;
					if let Err(e) = result {
						tracing::warn!(?e, "failed to gc queue messages");
					}
				}
			}
		});

		Ok(driver)
	}

	/// Manages the connection lifecycle with automatic reconnection
	async fn spawn_connection_lifecycle(
		conn_str: String,
		shard_subscriptions: Arc<HashMap<String, broadcast::Sender<()>>>,
		client: Arc<Mutex<Option<tokio_postgres::Client>>>,
		ready_tx: tokio::sync::watch::Sender<bool>,
		ssl_root_cert_path: Option<PathBuf>,
		ssl_client_cert_path: Option<PathBuf>,
		ssl_client_key_path: Option<PathBuf>,
	) {
		let mut backoff = Backoff::default();

		// Build TLS configuration with optional custom certificates
		let tls_config = match build_tls_config(
			ssl_root_cert_path.as_ref(),
			ssl_client_cert_path.as_ref(),
			ssl_client_key_path.as_ref(),
		) {
			std::result::Result::Ok(config) => config,
			std::result::Result::Err(e) => {
				tracing::error!(?e, "failed to build TLS config");
				return;
			}
		};

		let tls = MakeRustlsConnect::new(tls_config);

		loop {
			match tokio_postgres::connect(&conn_str, tls.clone()).await {
				Result::Ok((new_client, conn)) => {
					tracing::debug!("postgres listen connection established");
					// Reset backoff on successful connection
					backoff = Backoff::default();

					// Spawn the polling task immediately
					// This must be done before any operations on the client
					let shard_subscriptions_clone = shard_subscriptions.clone();
					let poll_handle = tokio::spawn(async move {
						Self::poll_connection(conn, shard_subscriptions_clone).await;
					});

					// Bound a stuck listener so it cannot wedge the shared notify queue.
					if let Result::Err(e) = new_client
						.execute(
							&format!(
								"SET idle_in_transaction_session_timeout = '{}'",
								LISTEN_IDLE_IN_TRANSACTION_TIMEOUT_MS
							),
							&[],
						)
						.await
					{
						tracing::warn!(?e, "failed to set idle_in_transaction_session_timeout");
					}

					// Get shard channels to re-subscribe to
					let mut channels = Vec::new();
					shard_subscriptions
						.iter_async(|k, _| {
							channels.push(k.clone());
							true
						})
						.await;

					if !channels.is_empty() {
						tracing::debug!(
							channels = channels.len(),
							"re-subscribing to doorbell shards after reconnection"
						);
					}

					for channel in channels.iter() {
						tracing::debug!(?channel, "re-subscribing to channel");
						if let Result::Err(e) = new_client
							.execute(&format!("LISTEN \"{}\"", channel), &[])
							.await
						{
							tracing::error!(?e, %channel, "failed to re-subscribe to channel");
						} else {
							tracing::debug!(%channel, "successfully re-subscribed to channel");
						}
					}

					// Update the client reference and signal ready
					// Do this AFTER re-subscribing to ensure LISTEN is complete
					*client.lock().await = Some(new_client);
					let _ = ready_tx.send(true);

					// Wait for the polling task to complete (when the connection closes)
					let _ = poll_handle.await;

					// Clear the client reference on disconnect
					*client.lock().await = None;

					// Notify that client is disconnected
					let _ = ready_tx.send(false);
				}
				Result::Err(e) => {
					tracing::error!(?e, "failed to connect to postgres, retrying");
					backoff.tick().await;
				}
			}
		}
	}

	/// Polls the connection for notifications until it closes or errors
	async fn poll_connection<T>(
		mut conn: tokio_postgres::Connection<tokio_postgres::Socket, T>,
		shard_subscriptions: Arc<HashMap<String, broadcast::Sender<()>>>,
	) where
		T: tokio_postgres::tls::TlsStream + Unpin,
	{
		loop {
			match poll_fn(|cx| conn.poll_message(cx)).await {
				Some(std::result::Result::Ok(AsyncMessage::Notification(note))) => {
					tracing::trace!(channel = %note.channel(), "received doorbell wakeup");
					// Doorbell notifications are payload-free wakeup signals only.
					// Subscribers read their payload from the table.
					if let Some(sub) = shard_subscriptions.get_async(note.channel()).await {
						let _ = sub.send(());
					} else {
						tracing::trace!(channel = %note.channel(), "wakeup for unknown shard");
					}
				}
				Some(std::result::Result::Ok(_)) => {
					// Ignore other async messages
				}
				Some(std::result::Result::Err(err)) => {
					tracing::error!(?err, "postgres connection error");
					break;
				}
				None => {
					tracing::warn!("postgres connection closed");
					break;
				}
			}
		}
	}

	/// Wait for the client to be connected
	async fn wait_for_client(&self) -> Result<()> {
		let mut ready_rx = self.client_ready.clone();
		tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
			loop {
				// Check if client is already available
				if self.client.lock().await.is_some() {
					return Ok(());
				}

				// Wait for the ready signal to change
				ready_rx
					.changed()
					.await
					.context("connection lifecycle task ended")?;
			}
		})
		.await
		.context("timeout waiting for postgres client connection")?
	}

	fn hash_subject(&self, subject: &str) -> String {
		// Postgres channel names have a 64 character limit, but this hash is also the
		// table index key. Collisions are possible and resolved by verifying the real
		// subject stored alongside each row.
		let mut hasher = DefaultHasher::new();
		subject.hash(&mut hasher);
		format!("ups_{:x}", hasher.finish())
	}

	fn hash_queue(&self, queue: &str) -> String {
		let mut hasher = DefaultHasher::new();
		queue.hash(&mut hasher);
		format!("{:x}", hasher.finish())
	}

	/// Returns the current max broadcast message id, used as a subscriber's starting
	/// cursor so it only sees future messages (NATS at-most-once, no replay).
	async fn current_max_id(&self) -> Result<i64> {
		let conn = self
			.pool
			.get()
			.await
			.context("failed to get connection for cursor init")?;
		let row = conn
			.query_one("SELECT COALESCE(MAX(id), 0) FROM ups_messages", &[])
			.await
			.context("failed to read current max id")?;
		Ok(row.get(0))
	}

	/// Ensures this process is LISTENing on the given doorbell shard and returns a
	/// wakeup receiver plus a drop guard that UNLISTENs once no receivers remain.
	async fn ensure_shard_listen(
		&self,
		shard: usize,
	) -> (broadcast::Receiver<()>, tokio_util::sync::DropGuard) {
		let channel = shard_channel(shard);

		match self.shard_subscriptions.entry_async(channel.clone()).await {
			scc::hash_map::Entry::Occupied(existing) => {
				let rx = existing.subscribe();
				let drop_guard =
					self.spawn_shard_cleanup_task(channel.clone(), existing.get().clone());
				(rx, drop_guard)
			}
			scc::hash_map::Entry::Vacant(e) => {
				let (tx, rx) = broadcast::channel(1024);
				e.insert_entry(tx.clone());
				metrics::POSTGRES_SUBSCRIPTION_COUNT.set(self.shard_subscriptions.len() as i64);

				if let Some(client) = &*self.client.lock().await {
					match client
						.execute(&format!("LISTEN \"{channel}\""), &[])
						.instrument(tracing::trace_span!("pg_listen"))
						.await
					{
						Result::Ok(_) => {
							tracing::debug!(%channel, "successfully subscribed to shard");
						}
						Result::Err(e) => {
							tracing::warn!(?e, %channel, "failed to LISTEN, will retry on reconnection");
						}
					}
				} else {
					tracing::debug!(%channel, "client not connected, will LISTEN on reconnection");
				}

				let drop_guard = self.spawn_shard_cleanup_task(channel.clone(), tx.clone());
				(rx, drop_guard)
			}
		}
	}

	fn spawn_shard_cleanup_task(
		&self,
		channel: String,
		tx: broadcast::Sender<()>,
	) -> tokio_util::sync::DropGuard {
		let driver = self.clone();
		let token = tokio_util::sync::CancellationToken::new();
		let drop_guard = token.clone().drop_guard();

		tokio::spawn(async move {
			token.cancelled().await;
			if tx.receiver_count() == 0 {
				if let Some(client) = &*driver.client.lock().await {
					let sql = format!("UNLISTEN \"{}\"", channel);
					if let Err(err) = client.execute(sql.as_str(), &[]).await {
						tracing::warn!(?err, %channel, "failed to UNLISTEN channel");
					} else {
						tracing::trace!(%channel, "unlistened channel");
					}
				}
				driver.shard_subscriptions.remove_async(&channel).await;
				metrics::POSTGRES_SUBSCRIPTION_COUNT.set(driver.shard_subscriptions.len() as i64);
			}
		});

		drop_guard
	}

	/// Inserts the broadcast row and any active queue-group rows in one transaction.
	async fn try_publish_to_db(
		&self,
		subject: &str,
		subject_hash: &str,
		payload: &[u8],
	) -> Result<()> {
		let mut conn = self
			.pool
			.get()
			.await
			.context("failed to get connection for publish")?;
		let tx = conn
			.transaction()
			.await
			.context("failed to begin publish transaction")?;

		// Broadcast row.
		tx.execute(
			"INSERT INTO ups_messages (subject_hash, subject, payload) VALUES ($1, $2, $3)",
			&[&subject_hash, &subject, &payload],
		)
		.await
		.context("failed to insert broadcast message")?;

		// Queue rows for every active queue group on this subject. Batched into the
		// same transaction so a crash never strands a row mid-publish.
		let rows = tx
			.query(
				"SELECT DISTINCT queue_hash FROM ups_queue_subs \
				 WHERE subject_hash = $1 \
				 AND heartbeat_at > NOW() - ($2::bigint * INTERVAL '1 second')",
				&[&subject_hash, &QUEUE_SUB_TTL_SECS],
			)
			.await
			.context("failed to query active queue subs")?;

		for row in rows {
			let queue_hash: String = row.get(0);
			tx.execute(
				"INSERT INTO ups_queue_messages (subject_hash, queue_hash, payload) \
				 VALUES ($1, $2, $3)",
				&[&subject_hash, &queue_hash, &payload],
			)
			.await
			.context("failed to insert queue message")?;
		}

		tx.commit().await.context("failed to commit publish")?;

		Ok(())
	}
}

#[async_trait]
impl PubSubDriver for PostgresDriver {
	async fn subscribe(
		&self,
		subject: &str,
		_reply_id: Option<Uuid>,
	) -> Result<SubscriberDriverHandle> {
		let subject_hash = self.hash_subject(subject);
		let shard = shard_for(&subject_hash);

		// Capture the cursor before LISTENing. Any message inserted after this point
		// has a higher id and is delivered either by the doorbell wakeup or the poll
		// backstop, so there is no subscribe/publish race.
		let cursor = self.current_max_id().await?;

		let (rx, drop_guard) = self.ensure_shard_listen(shard).await;

		Ok(Box::new(PostgresSubscriber {
			subject: subject.to_string(),
			subject_hash,
			pool: self.pool.clone(),
			cursor,
			buffer: VecDeque::new(),
			rx,
			_drop_guard: drop_guard,
		}))
	}

	async fn queue_subscribe(&self, subject: &str, queue: &str) -> Result<SubscriberDriverHandle> {
		let subject_hash = self.hash_subject(subject);
		let queue_hash = self.hash_queue(queue);
		let shard = shard_for(&subject_hash);

		// Register this subscriber in the database so publishers know the queue exists
		let sub_id = Uuid::new_v4().to_string();
		{
			let conn = self
				.pool
				.get()
				.await
				.context("failed to get connection for queue subscribe")?;
			conn.execute(
				"INSERT INTO ups_queue_subs (id, subject_hash, queue_hash) VALUES ($1, $2, $3)",
				&[&sub_id, &subject_hash, &queue_hash],
			)
			.await
			.context("failed to register queue subscriber")?;
		}

		let (rx, drop_guard) = self.ensure_shard_listen(shard).await;

		// Spawn heartbeat task to keep the registration alive
		let pool = self.pool.clone();
		let sub_id_for_heartbeat = sub_id.clone();
		let heartbeat_token = tokio_util::sync::CancellationToken::new();
		let heartbeat_token_child = heartbeat_token.clone();
		tokio::spawn(async move {
			let mut interval = tokio::time::interval(QUEUE_SUB_HEARTBEAT_INTERVAL);
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			loop {
				tokio::select! {
					_ = heartbeat_token_child.cancelled() => break,
					_ = interval.tick() => {
						if let Ok(conn) = pool.get().await {
							if let Err(e) = conn
								.execute(
									"UPDATE ups_queue_subs SET heartbeat_at = NOW() WHERE id = $1",
									&[&sub_id_for_heartbeat],
								)
								.await
							{
								tracing::warn!(?e, id = %sub_id_for_heartbeat, "failed to heartbeat queue sub");
							}
						}
					}
				}
			}
		});

		Ok(Box::new(PostgresQueueSubscriber {
			subject: subject.to_string(),
			subject_hash,
			queue_hash,
			sub_id,
			pool: self.pool.clone(),
			rx,
			_drop_guard: drop_guard,
			_heartbeat_token: heartbeat_token,
		}))
	}

	async fn publish(
		&self,
		subject: &str,
		payload: &[u8],
		_reply_subject: Option<&str>,
	) -> Result<()> {
		let subject_hash = self.hash_subject(subject);
		let shard = shard_for(&subject_hash);

		// Persist the message, retrying on transient connection errors. The row is
		// committed before the doorbell rings so any wakeup observes it.
		let mut backoff = Backoff::default();
		loop {
			match self
				.try_publish_to_db(subject, &subject_hash, payload)
				.await
			{
				Result::Ok(()) => break,
				Result::Err(e) => {
					if !backoff.tick().await {
						tracing::warn!(?e, %subject, "failed to publish, cannot retry again");
						return Err(e);
					}
					tracing::debug!(?e, "publish failed, retrying");
				}
			}
		}

		// Ring the doorbell. Best-effort: the subscriber poll backstop covers a
		// dropped or coalesced wakeup, so publish never blocks on NOTIFY.
		self.doorbell.mark_dirty(shard);

		Ok(())
	}

	async fn flush(&self) -> Result<()> {
		Ok(())
	}

	fn max_message_size(&self) -> usize {
		POSTGRES_MAX_MESSAGE_SIZE
	}
}

pub struct PostgresSubscriber {
	subject: String,
	subject_hash: String,
	pool: Arc<Pool>,
	cursor: i64,
	buffer: VecDeque<Vec<u8>>,
	rx: broadcast::Receiver<()>,
	_drop_guard: tokio_util::sync::DropGuard,
}

impl PostgresSubscriber {
	/// Reads new rows past the cursor into the buffer, advancing the cursor. Rows
	/// whose stored subject does not match are skipped (DefaultHasher collisions) but
	/// still advance the cursor.
	async fn fetch(&mut self) -> Result<()> {
		let conn = self
			.pool
			.get()
			.await
			.context("failed to get connection for poll")?;
		let rows = conn
			.query(
				"SELECT id, subject, payload FROM ups_messages \
				 WHERE subject_hash = $1 AND id > $2 ORDER BY id",
				&[&self.subject_hash, &self.cursor],
			)
			.await
			.context("failed to poll broadcast messages")?;

		for row in rows {
			let id: i64 = row.get(0);
			let subject: String = row.get(1);
			let payload: Vec<u8> = row.get(2);
			self.cursor = id;
			if subject == self.subject {
				self.buffer.push_back(payload);
			}
		}

		Ok(())
	}
}

#[async_trait]
impl SubscriberDriver for PostgresSubscriber {
	async fn next(&mut self) -> Result<DriverOutput> {
		loop {
			if let Some(payload) = self.buffer.pop_front() {
				return Ok(DriverOutput::Message {
					subject: self.subject.clone(),
					payload,
				});
			}

			if let Err(e) = self.fetch().await {
				// Transient DB errors must not kill the subscriber; the next poll
				// tick retries.
				tracing::warn!(?e, subject = %self.subject, "failed to poll, will retry");
			}

			if !self.buffer.is_empty() {
				continue;
			}

			// Wait for a doorbell wakeup or the poll backstop, whichever is first.
			tokio::select! {
				res = self.rx.recv() => {
					match res {
						std::result::Result::Ok(()) => {}
						Err(broadcast::error::RecvError::Lagged(_)) => {}
						Err(broadcast::error::RecvError::Closed) => {
							return Ok(DriverOutput::Unsubscribed);
						}
					}
				}
				_ = tokio::time::sleep(POLL_INTERVAL) => {}
			}
		}
	}
}

pub struct PostgresQueueSubscriber {
	subject: String,
	subject_hash: String,
	queue_hash: String,
	sub_id: String,
	pool: Arc<Pool>,
	rx: broadcast::Receiver<()>,
	_drop_guard: tokio_util::sync::DropGuard,
	_heartbeat_token: tokio_util::sync::CancellationToken,
}

impl PostgresQueueSubscriber {
	/// Attempts to atomically claim and delete one pending message for this (subject, queue).
	async fn claim_message(&self) -> Result<Option<Vec<u8>>> {
		let conn = self
			.pool
			.get()
			.await
			.context("failed to get connection for queue claim")?;

		let rows = conn
			.query(
				"WITH claimed AS ( \
				     SELECT id, payload FROM ups_queue_messages \
				     WHERE subject_hash = $1 AND queue_hash = $2 \
				     ORDER BY id \
				     LIMIT 1 \
				     FOR UPDATE SKIP LOCKED \
				 ) \
				 DELETE FROM ups_queue_messages \
				 WHERE id IN (SELECT id FROM claimed) \
				 RETURNING payload",
				&[&self.subject_hash, &self.queue_hash],
			)
			.await
			.context("failed to claim queue message")?;

		Ok(rows.into_iter().next().map(|row| row.get::<_, Vec<u8>>(0)))
	}
}

#[async_trait]
impl SubscriberDriver for PostgresQueueSubscriber {
	async fn next(&mut self) -> Result<DriverOutput> {
		loop {
			// Drain any messages that arrived before or between wakeups.
			match self.claim_message().await {
				Result::Ok(Some(payload)) => {
					return Ok(DriverOutput::Message {
						subject: self.subject.clone(),
						payload,
					});
				}
				Result::Ok(None) => {}
				Result::Err(e) => {
					tracing::warn!(?e, subject = %self.subject, "failed to claim, will retry");
				}
			}

			// Wait for a doorbell wakeup or the poll backstop, then loop back to claim.
			tokio::select! {
				res = self.rx.recv() => {
					match res {
						std::result::Result::Ok(()) => {}
						Err(broadcast::error::RecvError::Lagged(_)) => {}
						Err(broadcast::error::RecvError::Closed) => {
							return Ok(DriverOutput::Unsubscribed);
						}
					}
				}
				_ = tokio::time::sleep(POLL_INTERVAL) => {}
			}
		}
	}
}

impl Drop for PostgresQueueSubscriber {
	fn drop(&mut self) {
		let pool = self.pool.clone();
		let sub_id = self.sub_id.clone();
		tokio::spawn(async move {
			if let Ok(conn) = pool.get().await {
				if let Err(e) = conn
					.execute("DELETE FROM ups_queue_subs WHERE id = $1", &[&sub_id])
					.await
				{
					tracing::warn!(?e, %sub_id, "failed to deregister queue subscriber");
				}
			}
		});
	}
}
