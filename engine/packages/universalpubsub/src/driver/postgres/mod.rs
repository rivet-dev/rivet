use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD as BASE64;
use deadpool_postgres::{Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime};
use futures_util::future::poll_fn;
use rivet_postgres_util::build_tls_config;
use rivet_util::backoff::Backoff;
use scc::HashMap;
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

#[derive(Clone)]
struct Subscription {
	// Channel to send messages to this subscription
	tx: broadcast::Sender<Vec<u8>>,
}

impl Subscription {
	fn new(tx: broadcast::Sender<Vec<u8>>) -> Self {
		Self { tx }
	}
}

/// > In the default configuration it must be shorter than 8000 bytes
///
/// https://www.postgresql.org/docs/17/sql-notify.html
const MAX_NOTIFY_LENGTH: usize = 8000;

/// Base64 encoding ratio
const BYTES_PER_BLOCK: usize = 3;
const CHARS_PER_BLOCK: usize = 4;

/// Calculate max message size if encoded as base64
///
/// We need to remove BYTES_PER_BLOCK since there might be a tail on the base64-encoded data that
/// would bump it over the limit.
pub const POSTGRES_MAX_MESSAGE_SIZE: usize =
	(MAX_NOTIFY_LENGTH * BYTES_PER_BLOCK) / CHARS_PER_BLOCK - BYTES_PER_BLOCK;

const QUEUE_SUB_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
/// How long a queue subscriber's heartbeat must be within to be considered active.
const QUEUE_SUB_TTL_SECS: i64 = 30;
/// How often to GC orphaned queue messages.
const QUEUE_MESSAGE_GC_INTERVAL: Duration = Duration::from_secs(300);
/// Max age before an unconsumed queue message is garbage collected.
const QUEUE_MESSAGE_MAX_AGE_SECS: i64 = 3600;

#[derive(Clone)]
pub struct PostgresDriver {
	pool: Arc<Pool>,
	client: Arc<Mutex<Option<tokio_postgres::Client>>>,
	subscriptions: Arc<HashMap<String, Subscription>>,
	/// Wakeup channels for queue subscriptions, keyed by queue channel name.
	queue_subscriptions: Arc<HashMap<String, Subscription>>,
	client_ready: tokio::sync::watch::Receiver<bool>,
}

impl PostgresDriver {
	#[tracing::instrument(skip(conn_str), fields(memory_optimization))]
	pub async fn connect(
		conn_str: String,
		memory_optimization: bool,
		ssl_root_cert_path: Option<PathBuf>,
		ssl_client_cert_path: Option<PathBuf>,
		ssl_client_key_path: Option<PathBuf>,
	) -> Result<Self> {
		tracing::debug!(?memory_optimization, "connecting to postgres");
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

		let subscriptions: Arc<HashMap<String, Subscription>> = Arc::new(HashMap::new());
		let queue_subscriptions: Arc<HashMap<String, Subscription>> = Arc::new(HashMap::new());
		let client: Arc<Mutex<Option<tokio_postgres::Client>>> = Arc::new(Mutex::new(None));

		// Create channel for client ready notifications
		let (ready_tx, client_ready) = tokio::sync::watch::channel(false);

		// Spawn connection lifecycle task
		tokio::spawn(Self::spawn_connection_lifecycle(
			conn_str.clone(),
			subscriptions.clone(),
			queue_subscriptions.clone(),
			client.clone(),
			ready_tx,
			ssl_root_cert_path.clone(),
			ssl_client_cert_path.clone(),
			ssl_client_key_path.clone(),
		));

		let driver = Self {
			pool: Arc::new(pool),
			client,
			subscriptions,
			queue_subscriptions,
			client_ready,
		};

		// Wait for initial connection to be established
		driver.wait_for_client().await?;

		// Create queue tables eagerly so they exist before any publish or subscribe
		{
			let conn = driver
				.pool
				.get()
				.await
				.context("failed to get connection for queue table creation")?;
			conn.batch_execute(
				"CREATE TABLE IF NOT EXISTS ups_queue_subs ( \
				     id TEXT PRIMARY KEY, \
				     subject_hash TEXT NOT NULL, \
				     queue_hash TEXT NOT NULL, \
				     heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT NOW() \
				 ); \
				 CREATE INDEX IF NOT EXISTS ups_queue_subs_subject_queue \
				     ON ups_queue_subs (subject_hash, queue_hash); \
				 CREATE TABLE IF NOT EXISTS ups_queue_messages ( \
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
			.context("failed to create queue tables")?;
			tracing::debug!("queue tables ready");
		}

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
		subscriptions: Arc<HashMap<String, Subscription>>,
		queue_subscriptions: Arc<HashMap<String, Subscription>>,
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
					let subscriptions_clone = subscriptions.clone();
					let queue_subscriptions_clone = queue_subscriptions.clone();
					let poll_handle = tokio::spawn(async move {
						Self::poll_connection(conn, subscriptions_clone, queue_subscriptions_clone)
							.await;
					});

					// Get regular channels to re-subscribe to
					let mut channels = Vec::new();
					subscriptions
						.iter_async(|k, _| {
							channels.push(k.clone());
							true
						})
						.await;

					// Get queue wakeup channels to re-subscribe to
					let mut queue_channels = Vec::new();
					queue_subscriptions
						.iter_async(|k, _| {
							queue_channels.push(k.clone());
							true
						})
						.await;

					let needs_resubscribe = !channels.is_empty() || !queue_channels.is_empty();
					if needs_resubscribe {
						tracing::debug!(
							regular_channels = channels.len(),
							queue_channels = queue_channels.len(),
							"re-subscribing to channels after reconnection"
						);
					}

					for channel in channels.iter().chain(queue_channels.iter()) {
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
		subscriptions: Arc<HashMap<String, Subscription>>,
		queue_subscriptions: Arc<HashMap<String, Subscription>>,
	) where
		T: tokio_postgres::tls::TlsStream + Unpin,
	{
		loop {
			match poll_fn(|cx| conn.poll_message(cx)).await {
				Some(std::result::Result::Ok(AsyncMessage::Notification(note))) => {
					tracing::trace!(channel = %note.channel(), "received notification");
					if let Some(sub) = subscriptions.get_async(note.channel()).await {
						let bytes = match BASE64.decode(note.payload()) {
							std::result::Result::Ok(b) => b,
							std::result::Result::Err(err) => {
								tracing::error!(?err, "failed decoding base64");
								continue;
							}
						};
						tracing::trace!(channel = %note.channel(), bytes_len = bytes.len(), "sending to broadcast channel");
						let _ = sub.tx.send(bytes);
					} else if let Some(sub) = queue_subscriptions.get_async(note.channel()).await {
						// Queue notifications are wakeup signals only; payload lives in the table
						let _ = sub.tx.send(Vec::new());
					} else {
						tracing::warn!(channel = %note.channel(), "received notification for unknown channel");
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
		// Postgres channel names have a 64 character limit
		// Hash the subject to ensure it fits
		let mut hasher = DefaultHasher::new();
		subject.hash(&mut hasher);
		format!("ups_{:x}", hasher.finish())
	}

	fn hash_queue(&self, queue: &str) -> String {
		let mut hasher = DefaultHasher::new();
		queue.hash(&mut hasher);
		format!("{:x}", hasher.finish())
	}

	/// Returns the NOTIFY channel name for a (subject, queue) pair.
	fn queue_channel(&self, subject_hash: &str, queue_hash: &str) -> String {
		// Max length: "ups_q_" (6) + 16 + "_" (1) + 16 = 39 chars, well within 64
		format!("ups_q_{}_{}", subject_hash, queue_hash)
	}

	/// Inserts messages into the queue table and notifies active queue subscribers.
	async fn publish_to_queues(&self, subject: &str, payload: &[u8]) -> Result<()> {
		let subject_hash = self.hash_subject(subject);

		let conn = self
			.pool
			.get()
			.await
			.context("failed to get connection for queue publish")?;

		// Find active queue groups for this subject
		let rows = conn
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
			let channel = self.queue_channel(&subject_hash, &queue_hash);

			conn.execute(
				"INSERT INTO ups_queue_messages (subject_hash, queue_hash, payload) \
				 VALUES ($1, $2, $3)",
				&[&subject_hash, &queue_hash, &payload],
			)
			.await
			.context("failed to insert queue message")?;

			conn.execute(&format!("NOTIFY \"{}\"", channel), &[])
				.await
				.context("failed to notify queue channel")?;
		}

		Ok(())
	}

	fn spawn_subscription_cleanup_task(
		&self,
		subject_hash: String,
		tx: broadcast::Sender<Vec<u8>>,
	) -> tokio_util::sync::DropGuard {
		let driver = self.clone();
		let token = tokio_util::sync::CancellationToken::new();
		let drop_guard = token.clone().drop_guard();

		tokio::spawn(async move {
			token.cancelled().await;
			if tx.receiver_count() == 0 {
				if let Some(client) = &*driver.client.lock().await {
					let sql = format!("UNLISTEN \"{}\"", subject_hash);
					if let Err(err) = client.execute(sql.as_str(), &[]).await {
						tracing::warn!(?err, %subject_hash, "failed to UNLISTEN channel");
					} else {
						tracing::trace!(%subject_hash, "unlistened channel");
					}
				}
				driver.subscriptions.remove_async(&subject_hash).await;
				metrics::POSTGRES_SUBSCRIPTION_COUNT.set(driver.subscriptions.len() as i64);
			}
		});

		drop_guard
	}

	fn spawn_queue_subscription_cleanup_task(
		&self,
		channel: String,
		tx: broadcast::Sender<Vec<u8>>,
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
						tracing::warn!(?err, %channel, "failed to UNLISTEN queue channel");
					} else {
						tracing::trace!(%channel, "unlistened queue channel");
					}
				}
				driver.queue_subscriptions.remove_async(&channel).await;
			}
		});

		drop_guard
	}
}

#[async_trait]
impl PubSubDriver for PostgresDriver {
	async fn subscribe(&self, subject: &str) -> Result<SubscriberDriverHandle> {
		// TODO: To match NATS implementation, LISTEN must be pipelined (i.e. wait for the command
		// to reach the server, but not wait for it to respond). However, this has to ensure that
		// NOTIFY & LISTEN are called on the same connection (not diff connections in a pool) or
		// else there will be race conditions where messages might be published before
		// subscriptions are registered.
		//
		// tokio-postgres currently does not expose the API for pipelining, so we are SOL.
		//
		// We might be able to use a background tokio task in combination with flush if we use the
		// same Postgres connection, but unsure if that will create a bottleneck.

		let hashed = self.hash_subject(subject);

		// Check if we already have a subscription for this channel
		let (rx, drop_guard) = match self.subscriptions.entry_async(hashed.clone()).await {
			scc::hash_map::Entry::Occupied(existing_sub) => {
				// Reuse the existing broadcast channel
				let rx = existing_sub.tx.subscribe();
				let drop_guard =
					self.spawn_subscription_cleanup_task(hashed.clone(), existing_sub.tx.clone());
				(rx, drop_guard)
			}
			scc::hash_map::Entry::Vacant(e) => {
				// Create a new broadcast channel for this subject
				let (tx, rx) = tokio::sync::broadcast::channel(1024);
				let subscription = Subscription::new(tx.clone());

				// Register subscription
				e.insert_entry(subscription.clone());
				metrics::POSTGRES_SUBSCRIPTION_COUNT.set(self.subscriptions.len() as i64);

				// Execute LISTEN command on the async client (for receiving notifications)
				// This only needs to be done once per channel
				// Try to LISTEN if client is available, but don't fail if disconnected
				// The reconnection logic will handle re-subscribing
				if let Some(client) = &*self.client.lock().await {
					match client
						.execute(&format!("LISTEN \"{hashed}\""), &[])
						.instrument(tracing::trace_span!("pg_listen"))
						.await
					{
						Result::Ok(_) => {
							tracing::debug!(%hashed, "successfully subscribed to channel");
						}
						Result::Err(e) => {
							tracing::warn!(?e, %hashed, "failed to LISTEN, will retry on reconnection");
						}
					}
				} else {
					tracing::debug!(%hashed, "client not connected, will LISTEN on reconnection");
				}

				let drop_guard = self.spawn_subscription_cleanup_task(hashed.clone(), tx.clone());
				(rx, drop_guard)
			}
		};

		Ok(Box::new(PostgresSubscriber {
			subject: subject.to_string(),
			rx: Some(rx),
			_drop_guard: drop_guard,
		}))
	}

	async fn queue_subscribe(&self, subject: &str, queue: &str) -> Result<SubscriberDriverHandle> {
		let subject_hash = self.hash_subject(subject);
		let queue_hash = self.hash_queue(queue);
		let channel = self.queue_channel(&subject_hash, &queue_hash);

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

		// Set up a shared LISTEN/broadcast channel for the wakeup signal
		let (rx, drop_guard) = match self.queue_subscriptions.entry_async(channel.clone()).await {
			scc::hash_map::Entry::Occupied(existing_sub) => {
				let rx = existing_sub.tx.subscribe();
				let drop_guard = self.spawn_queue_subscription_cleanup_task(
					channel.clone(),
					existing_sub.tx.clone(),
				);
				(rx, drop_guard)
			}
			scc::hash_map::Entry::Vacant(e) => {
				let (tx, rx) = tokio::sync::broadcast::channel(1024);
				let subscription = Subscription::new(tx.clone());

				e.insert_entry(subscription.clone());

				if let Some(client) = &*self.client.lock().await {
					match client
						.execute(&format!("LISTEN \"{}\"", channel), &[])
						.instrument(tracing::trace_span!("pg_listen_queue"))
						.await
					{
						Result::Ok(_) => {
							tracing::debug!(%channel, "successfully subscribed to queue channel");
						}
						Result::Err(e) => {
							tracing::warn!(?e, %channel, "failed to LISTEN queue channel, will retry on reconnection");
						}
					}
				} else {
					tracing::debug!(%channel, "client not connected, will LISTEN queue channel on reconnection");
				}

				let drop_guard =
					self.spawn_queue_subscription_cleanup_task(channel.clone(), tx.clone());
				(rx, drop_guard)
			}
		};

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
			rx: Some(rx),
			_drop_guard: drop_guard,
			_heartbeat_token: heartbeat_token,
		}))
	}

	async fn publish(&self, subject: &str, payload: &[u8]) -> Result<()> {
		// TODO: See `subscribe` about pipelining

		// Encode payload to base64 and send NOTIFY
		let encoded = BASE64.encode(payload);
		let hashed = self.hash_subject(subject);

		tracing::trace!("attempting to get connection for publish");

		// Wait for listen connection to be ready first if this channel has subscribers
		// This ensures that if we're reconnecting, the LISTEN is re-registered before NOTIFY
		if self.subscriptions.contains_async(&hashed).await {
			self.wait_for_client().await?;
		}

		// Retry getting a connection from the pool with backoff in case the connection is
		// currently disconnected
		let mut backoff = Backoff::default();
		let mut last_error;

		loop {
			match self.pool.get().await {
				Result::Ok(conn) => {
					// Test the connection with a simple query before using it
					match conn.execute("SELECT 1", &[]).await {
						Result::Ok(_) => {
							// Connection is good; run NOTIFY and queue publish in parallel.
							// publish_to_queues acquires its own pool connection so both
							// can proceed concurrently.
							let notify_sql = format!("NOTIFY \"{hashed}\", '{encoded}'");
							let (notify_result, queue_result) = tokio::join!(
								conn.execute(notify_sql.as_str(), &[])
									.instrument(tracing::trace_span!("pg_notify")),
								self.publish_to_queues(subject, payload),
							);
							match notify_result {
								Result::Ok(_) => {
									if let Err(e) = queue_result {
										tracing::warn!(?e, %subject, "failed to publish to queue subscribers");
									}
									return Ok(());
								}
								Result::Err(e) => {
									tracing::debug!(
										?e,
										"NOTIFY failed, retrying with new connection"
									);
									last_error = Some(e.into());
								}
							}
						}
						Result::Err(e) => {
							tracing::debug!(
								?e,
								"connection test failed, retrying with new connection"
							);
							last_error = Some(e.into());
						}
					}
				}
				Result::Err(e) => {
					tracing::debug!(?e, "failed to get connection from pool, retrying");
					last_error = Some(e.into());
				}
			}

			// Check if we should continue retrying
			if !backoff.tick().await {
				return Err(
					last_error.unwrap_or_else(|| anyhow!("failed to publish after retries"))
				);
			}
		}
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
	rx: Option<tokio::sync::broadcast::Receiver<Vec<u8>>>,
	_drop_guard: tokio_util::sync::DropGuard,
}

#[async_trait]
impl SubscriberDriver for PostgresSubscriber {
	async fn next(&mut self) -> Result<DriverOutput> {
		let rx = match self.rx.as_mut() {
			Some(rx) => rx,
			None => return Ok(DriverOutput::Unsubscribed),
		};
		match rx.recv().await {
			std::result::Result::Ok(payload) => Ok(DriverOutput::Message {
				subject: self.subject.clone(),
				payload,
			}),
			Err(tokio::sync::broadcast::error::RecvError::Closed) => Ok(DriverOutput::Unsubscribed),
			Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
				// Try again
				self.next().await
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
	rx: Option<tokio::sync::broadcast::Receiver<Vec<u8>>>,
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
			// Drain any messages that arrived before or between notifications.
			// Do this before borrowing rx so claim_message can borrow self freely.
			if let Some(payload) = self.claim_message().await? {
				return Ok(DriverOutput::Message {
					subject: self.subject.clone(),
					payload,
				});
			}

			// Wait for a wakeup notification, then loop back to claim.
			let rx = match self.rx.as_mut() {
				Some(rx) => rx,
				None => return Ok(DriverOutput::Unsubscribed),
			};
			match rx.recv().await {
				std::result::Result::Ok(_) => {
					// Wakeup received; loop back to claim
				}
				Err(tokio::sync::broadcast::error::RecvError::Closed) => {
					return Ok(DriverOutput::Unsubscribed);
				}
				Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
					// Notifications were dropped while lagged; loop back to claim in case
					// messages are waiting
				}
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
