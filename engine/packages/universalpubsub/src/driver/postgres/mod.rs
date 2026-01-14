use anyhow::*;
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
use tokio::sync::{Mutex, broadcast};
use tokio_postgres::AsyncMessage;
use tokio_postgres_rustls::MakeRustlsConnect;
use tracing::Instrument;

use crate::driver::{PubSubDriver, SubscriberDriver, SubscriberDriverHandle};
use crate::metrics;
use crate::pubsub::DriverOutput;

#[derive(Clone)]
struct Subscription {
	// Channel to send messages to this subscription
	tx: broadcast::Sender<Vec<u8>>,
	// Cancellation token shared by all subscribers of this subject
	token: tokio_util::sync::CancellationToken,
}

impl Subscription {
	fn new(tx: broadcast::Sender<Vec<u8>>) -> Self {
		let token = tokio_util::sync::CancellationToken::new();
		Self { tx, token }
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

#[derive(Clone)]
pub struct PostgresDriver {
	pool: Arc<Pool>,
	client: Arc<Mutex<Option<tokio_postgres::Client>>>,
	subscriptions: Arc<HashMap<String, Subscription>>,
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
		let client: Arc<Mutex<Option<tokio_postgres::Client>>> = Arc::new(Mutex::new(None));

		// Create channel for client ready notifications
		let (ready_tx, client_ready) = tokio::sync::watch::channel(false);

		// Spawn connection lifecycle task
		tokio::spawn(Self::spawn_connection_lifecycle(
			conn_str.clone(),
			subscriptions.clone(),
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
			client_ready,
		};

		// Wait for initial connection to be established
		driver.wait_for_client().await?;

		Ok(driver)
	}

	/// Manages the connection lifecycle with automatic reconnection
	async fn spawn_connection_lifecycle(
		conn_str: String,
		subscriptions: Arc<HashMap<String, Subscription>>,
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
					let poll_handle = tokio::spawn(async move {
						Self::poll_connection(conn, subscriptions_clone).await;
					});

					// Get channels to re-subscribe to
					let mut channels = Vec::new();
					subscriptions
						.iter_async(|k, _| {
							channels.push(k.clone());
							true
						})
						.await;
					let needs_resubscribe = !channels.is_empty();

					if needs_resubscribe {
						tracing::debug!(
							channels=?channels.len(),
							"will re-subscribe to channels after connection starts"
						);
					}

					// Re-subscribe to channels
					if needs_resubscribe {
						tracing::debug!(
							channels=?channels.len(),
							"re-subscribing to channels after reconnection"
						);
						for channel in &channels {
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
					.map_err(|_| anyhow!("connection lifecycle task ended"))?;
			}
		})
		.await
		.map_err(|_| anyhow!("timeout waiting for postgres client connection"))?
	}

	fn hash_subject(&self, subject: &str) -> String {
		// Postgres channel names have a 64 character limit
		// Hash the subject to ensure it fits
		let mut hasher = DefaultHasher::new();
		subject.hash(&mut hasher);
		format!("ups_{:x}", hasher.finish())
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
				let drop_guard = existing_sub.token.clone().drop_guard();
				(rx, drop_guard)
			}
			scc::hash_map::Entry::Vacant(e) => {
				// Create a new broadcast channel for this subject
				let (tx, rx) = tokio::sync::broadcast::channel(1024);
				let subscription = Subscription::new(tx.clone());

				// Register subscription
				e.insert_entry(subscription.clone());
				metrics::POSTGRES_SUBSCRIPTIONS_COUNT.set(self.subscriptions.len() as i64);

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

				// Spawn a single cleanup task for this subscription waiting on its token
				let driver = self.clone();
				let tx_clone = tx.clone();
				let token_clone = subscription.token.clone();
				tokio::spawn(async move {
					token_clone.cancelled().await;
					if tx_clone.receiver_count() == 0 {
						if let Some(client) = &*driver.client.lock().await {
							let sql = format!("UNLISTEN \"{}\"", hashed);
							if let Err(err) = client.execute(sql.as_str(), &[]).await {
								tracing::warn!(?err, %hashed, "failed to UNLISTEN channel");
							} else {
								tracing::trace!(%hashed, "unlistened channel");
							}
						}
						driver.subscriptions.remove_async(&hashed).await;
						metrics::POSTGRES_SUBSCRIPTIONS_COUNT
							.set(driver.subscriptions.len() as i64);
					}
				});

				let drop_guard = subscription.token.clone().drop_guard();
				(rx, drop_guard)
			}
		};

		Ok(Box::new(PostgresSubscriber {
			subject: subject.to_string(),
			rx: Some(rx),
			_drop_guard: drop_guard,
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
							// Connection is good, use it for NOTIFY
							match conn
								.execute(&format!("NOTIFY \"{hashed}\", '{encoded}'"), &[])
								.instrument(tracing::trace_span!("pg_notify"))
								.await
							{
								Result::Ok(_) => return Ok(()),
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
