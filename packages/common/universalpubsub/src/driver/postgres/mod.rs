use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

use anyhow::*;
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use deadpool_postgres::{Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime};
use futures_util::future::poll_fn;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tokio_postgres::{AsyncMessage, NoTls};
use tracing::Instrument;

use crate::driver::{PubSubDriver, SubscriberDriver, SubscriberDriverHandle};
use crate::errors;
use crate::pubsub::{Message, NextOutput, Response};

// Represents a local subscription that can handle request/response
struct LocalSubscription {
	// Channel to send requests to this subscription
	tx: tokio::sync::broadcast::Sender<LocalRequest>,
}

// Request sent to a local subscription
#[derive(Clone)]
struct LocalRequest {
	payload: Vec<u8>,
	reply_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
}

#[derive(Clone)]
pub struct PostgresDriver {
	conn_str: String,
	pool: Arc<Pool>,
	memory_optimization: bool,
	// Maps subject to local subscription on this node for fast path
	local_subscriptions: Arc<RwLock<HashMap<String, LocalSubscription>>>,
}

#[derive(Serialize, Deserialize)]
struct Envelope {
	// Base64-encoded payload
	p: String,
	// Optional reply subject
	#[serde(skip_serializing_if = "Option::is_none")]
	r: Option<String>,
}

impl PostgresDriver {
	#[tracing::instrument(skip(conn_str), fields(memory_optimization))]
	pub async fn connect(conn_str: String, memory_optimization: bool) -> Result<Self> {
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
		let pool = config
			.create_pool(Some(Runtime::Tokio1), NoTls)
			.context("failed to create postgres pool")?;
		tracing::debug!("postgres pool created successfully");

		let driver = Self {
			conn_str,
			pool: Arc::new(pool),
			memory_optimization,
			local_subscriptions: Arc::new(RwLock::new(HashMap::new())),
		};
		tracing::info!("postgres driver connected successfully");
		Ok(driver)
	}

	fn quote_ident(subject: &str) -> String {
		// Double-quote and escape any embedded quotes for safe identifier usage
		let escaped = subject.replace('"', "\"\"");
		format!("\"{}\"", escaped)
	}

	/// Convert a subject name to a PostgreSQL advisory lock ID
	/// Uses SHA256 hash truncated to 63 bits to avoid collisions
	fn subject_to_lock_id(subject: &str) -> i64 {
		let mut hasher = Sha256::new();
		hasher.update(subject.as_bytes());
		let hash = hasher.finalize();

		// Take first 8 bytes and convert to i64, using only 63 bits to avoid sign issues
		let mut bytes = [0u8; 8];
		bytes.copy_from_slice(&hash[0..8]);
		let hash_u64 = u64::from_be_bytes(bytes);
		(hash_u64 & 0x7FFFFFFFFFFFFFFF) as i64
	}
}

#[async_trait]
impl PubSubDriver for PostgresDriver {
	#[tracing::instrument(skip(self), fields(subject))]
	async fn subscribe(&self, subject: &str) -> Result<SubscriberDriverHandle> {
		tracing::debug!(%subject, "starting subscription");
		// Get the lock ID for this subject
		let lock_id = Self::subject_to_lock_id(subject);
		tracing::debug!(%subject, ?lock_id, "calculated advisory lock id");

		// Create a single connection for both subscription and lock holding
		let (client, mut connection) =
			tokio_postgres::connect(&self.conn_str, tokio_postgres::NoTls).await?;

		// Set up message forwarding
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
		let subject_owned = subject.to_string();

		// Set up local request handling channel if memory optimization is enabled
		let local_request_rx = if self.memory_optimization {
			// Register this subscription in the local map
			tracing::debug!(
				%subject,
				"registering local subscription for memory optimization"
			);
			let mut subs = self.local_subscriptions.write().await;
			let local_rx = subs
				.entry(subject.to_string())
				.or_insert_with(|| LocalSubscription {
					tx: tokio::sync::broadcast::channel::<LocalRequest>(64).0,
				})
				.tx
				.subscribe();
			tracing::debug!(
				%subject,
				"local subscription registered"
			);

			Some(local_rx)
		} else {
			None
		};

		// Create channels for coordinating initialization
		let (listen_done_tx, listen_done_rx) = tokio::sync::oneshot::channel();
		let (lock_done_tx, lock_done_rx) = tokio::sync::oneshot::channel();

		// We need to wrap the client in Arc for sharing with Drop impl
		let client = Arc::new(client);
		let client_clone = client.clone();
		let listen_subject = subject_owned.clone();

		// Spawn task to handle connection, lock acquisition, and LISTEN
		tokio::spawn(async move {
			// First acquire the lock while polling the connection
			let lock_sql = format!("SELECT pg_try_advisory_lock_shared({})", lock_id);
			let lock_future = client_clone.query_one(&lock_sql, &[]);
			tokio::pin!(lock_future);

			let mut lock_done = false;
			let mut lock_done_tx = Some(lock_done_tx);
			let mut listen_started = false;

			// Convert subject to base64 hash string because Postgres identifiers can only be 63 bytes
			let mut hasher = DefaultHasher::new();
			listen_subject.hash(&mut hasher);
			let subject = BASE64.encode(&hasher.finish().to_be_bytes());

			// Execute LISTEN while polling the connection
			let sql = format!("LISTEN {}", Self::quote_ident(&subject));
			let listen_future = client_clone.batch_execute(&sql);
			tokio::pin!(listen_future);

			let mut listen_done = false;
			let mut listen_done_tx = Some(listen_done_tx);

			loop {
				tokio::select! {
					// First acquire the lock
					result = &mut lock_future, if !lock_done => {
						lock_done = true;
						if let Some(tx) = lock_done_tx.take() {
							let lock_acquired = result.as_ref().map(|row| row.get::<_, bool>(0)).unwrap_or(false);
							let _ = tx.send(result.map(|_| lock_acquired).map_err(|e| anyhow::Error::new(e)));
						}
						listen_started = true;
					}
					// Then execute LISTEN
					result = &mut listen_future, if listen_started && !listen_done => {
						listen_done = true;
						if let Some(tx) = listen_done_tx.take() {
							let _ = tx.send(result.map_err(|e| anyhow::Error::new(e)));
						}
					}
					// Poll messages
					msg = poll_fn(|cx| connection.poll_message(cx)) => {
						match msg {
							Some(std::result::Result::Ok(AsyncMessage::Notification(note))) => {
								if note.channel() == subject {
									let _ = tx.send(note.payload().to_string());
								}
							}
							Some(std::result::Result::Ok(_)) => continue,
							Some(std::result::Result::Err(_)) => break,
							None => break,
						}
					}
				}
			}
		});

		// Wait for lock acquisition to complete
		tracing::debug!(%subject, ?lock_id, "waiting for advisory lock acquisition");
		match lock_done_rx.await {
			std::result::Result::Ok(std::result::Result::Ok(true)) => {
				tracing::debug!(%subject, ?lock_id, "advisory lock acquired successfully");
			}
			std::result::Result::Ok(std::result::Result::Ok(false)) => {
				tracing::warn!(
					%subject,
					?lock_id,
					"failed to acquire advisory lock - another subscriber may already exist"
				);
				return Err(anyhow!("Failed to acquire advisory lock for subject"));
			}
			std::result::Result::Ok(std::result::Result::Err(err)) => {
				return Err(err);
			}
			std::result::Result::Err(_) => {
				return Err(anyhow!("Failed to acquire lock"));
			}
		}

		// Wait for LISTEN to complete
		tracing::debug!(%subject, "waiting for LISTEN command to complete");
		match listen_done_rx.await {
			std::result::Result::Ok(std::result::Result::Ok(())) => {
				tracing::debug!(%subject, "LISTEN command completed successfully");
			}
			std::result::Result::Ok(std::result::Result::Err(err)) => {
				// Release lock on error
				let _ = client
					.execute("SELECT pg_advisory_unlock_shared($1)", &[&lock_id])
					.await;
				return Err(err);
			}
			std::result::Result::Err(_) => {
				// Release lock on error
				let _ = client
					.execute("SELECT pg_advisory_unlock_shared($1)", &[&lock_id])
					.await;
				return Err(anyhow!("Failed to confirm LISTEN"));
			}
		}

		tracing::info!(%subject, "subscription established successfully");
		Ok(Box::new(PostgresSubscriber {
			driver: self.clone(),
			rx,
			local_request_rx,
			lock_id,
			client,
			subject: subject.to_string(),
		}))
	}

	#[tracing::instrument(skip(self, message), fields(subject, message_len = message.len()))]
	async fn publish(&self, subject: &str, message: &[u8]) -> Result<()> {
		tracing::debug!(%subject, message_len = message.len(), "publishing message");
		// Get a connection from the pool
		let conn = self
			.pool
			.get()
			.await
			.context("failed to get connection from pool")?;

		// Convert subject to base64 hash string because Postgres identifiers can only be 63 bytes
		let mut hasher = DefaultHasher::new();
		subject.hash(&mut hasher);
		let subject = BASE64.encode(&hasher.finish().to_be_bytes());

		// Encode payload
		let env = Envelope {
			p: BASE64.encode(message),
			r: None,
		};
		let payload = serde_json::to_string(&env)?;

		// NOTIFY doesn't support parameterized queries, so we need to escape the payload
		// Replace single quotes with two single quotes for SQL escaping
		let escaped_payload = payload.replace('\'', "''");
		let sql = format!(
			"NOTIFY {}, '{}'",
			Self::quote_ident(&subject),
			escaped_payload
		);
		conn.batch_execute(&sql)
			.instrument(tracing::debug_span!("notify_execute", %subject))
			.await?;
		tracing::debug!(%subject, "message published successfully");
		Ok(())
	}

	async fn flush(&self) -> Result<()> {
		// No-op for Postgres
		Ok(())
	}

	#[tracing::instrument(skip(self, payload), fields(subject, payload_len = payload.len(), ?timeout))]
	async fn request(
		&self,
		subject: &str,
		payload: &[u8],
		timeout: Option<Duration>,
	) -> Result<Response> {
		tracing::debug!(
			%subject,
			payload_len = payload.len(),
			?timeout,
			"starting request"
		);
		// Memory fast path: check if we have local subscribers first
		if self.memory_optimization {
			let subs = self.local_subscriptions.read().await;
			if let Some(local_tx) = subs.get(subject) {
				tracing::debug!(
					%subject,
					"using memory fast path for request"
				);

				// Create a channel for the reply
				let (reply_tx, mut reply_rx) = tokio::sync::mpsc::channel(1);

				// Send the request to the local subscription
				let request = LocalRequest {
					payload: payload.to_vec(),
					reply_tx,
				};

				// Try to send the request
				if local_tx.tx.send(request).is_ok() {
					// Drop early to clear lock
					drop(subs);

					// Wait for response with optional timeout
					let response_future = async {
						match reply_rx.recv().await {
							Some(response_payload) => Ok(Response {
								payload: response_payload,
							}),
							None => Err(anyhow!("local subscription closed")),
						}
					};

					// Apply timeout if specified
					if let Some(dur) = timeout {
						return match tokio::time::timeout(dur, response_future).await {
							std::result::Result::Ok(resp) => resp,
							std::result::Result::Err(_) => {
								Err(errors::Ups::RequestTimeout.build().into())
							}
						};
					} else {
						return response_future.await;
					}
				}
				// If send failed, the subscription might be dead, clean it up later
				// and fall through to normal path
			}
		}

		// Normal path: check for listeners via database
		tracing::debug!(%subject, "checking for remote listeners via database");
		// Get a connection from the pool for checking listeners
		let conn = self
			.pool
			.get()
			.await
			.context("failed to get connection from pool")?;

		// First check if there are any listeners for this subject
		let lock_id = Self::subject_to_lock_id(subject);

		// Check if there are any shared advisory locks (listeners) for this subject
		// Query pg_locks directly to avoid lock acquisition overhead
		// Split the 64-bit lock_id into two 32-bit integers for pg_locks query
		let classid = (lock_id >> 32) as i32;
		let objid = (lock_id & 0xFFFFFFFF) as i32;

		let check_sql = "
            SELECT EXISTS (
                SELECT 1 FROM pg_locks 
                WHERE locktype = 'advisory' 
                AND classid = $1::int
                AND objid = $2::int
                AND mode = 'ShareLock'
            ) AS has_listeners
        ";
		let row = conn.query_one(check_sql, &[&classid, &objid]).await?;
		let has_listeners: bool = row.get(0);
		tracing::debug!(
			%subject,
			?has_listeners,
			"checked for listeners in database"
		);

		if !has_listeners {
			tracing::warn!(%subject, "no listeners found for subject");
			return Err(errors::Ups::NoResponders.build().into());
		}

		// Drop the pool connection before creating new dedicated connections
		drop(conn);

		// Create a temporary reply subject and a dedicated listener connection
		let reply_subject = format!("_INBOX.{}", uuid::Uuid::new_v4());

		let (client, mut connection) =
			tokio_postgres::connect(&self.conn_str, tokio_postgres::NoTls).await?;

		// Setup connection and LISTEN in a task
		let (listen_done_tx, listen_done_rx) = tokio::sync::oneshot::channel();
		let reply_subject_clone = reply_subject.clone();

		// Spawn task to handle connection and LISTEN
		let (response_tx, mut response_rx) = tokio::sync::mpsc::unbounded_channel();
		tokio::spawn(async move {
			// Convert subject to base64 hash string because Postgres identifiers can only be 63 bytes
			let mut hasher = DefaultHasher::new();
			reply_subject_clone.hash(&mut hasher);
			let reply_subject = BASE64.encode(&hasher.finish().to_be_bytes());

			// LISTEN reply subject first to avoid race
			let listen_sql = format!("LISTEN {}", Self::quote_ident(&reply_subject));
			let listen_future = client.batch_execute(&listen_sql);
			tokio::pin!(listen_future);

			let mut listen_done = false;
			let mut listen_done_tx = Some(listen_done_tx);

			loop {
				tokio::select! {
					result = &mut listen_future, if !listen_done => {
						listen_done = true;
						if let Some(tx) = listen_done_tx.take() {
							let _ = tx.send(result.map_err(|e| anyhow::Error::new(e)));
						}
					}
					msg = poll_fn(|cx| connection.poll_message(cx)) => {
						match msg {
							Some(std::result::Result::Ok(tokio_postgres::AsyncMessage::Notification(note))) => {
								if note.channel() == reply_subject {
									let _ = response_tx.send(note.payload().to_string());
								}
							}
							Some(std::result::Result::Ok(_)) => continue,
							Some(std::result::Result::Err(_)) => break,
							None => break,
						}
					}
				}
			}
		});

		// Wait for LISTEN to complete
		match listen_done_rx.await {
			std::result::Result::Ok(std::result::Result::Ok(())) => {}
			std::result::Result::Ok(std::result::Result::Err(err)) => return Err(err),
			std::result::Result::Err(_) => return Err(anyhow!("Failed to setup LISTEN")),
		}

		// Get another connection from pool to publish the request
		let publish_conn = self
			.pool
			.get()
			.await
			.context("failed to get connection from pool")?;

		// Convert subject to base64 hash string because Postgres identifiers can only be 63 bytes
		let mut hasher = DefaultHasher::new();
		subject.hash(&mut hasher);
		let subject = BASE64.encode(&hasher.finish().to_be_bytes());

		// Publish request with reply subject encoded
		let env = Envelope {
			p: BASE64.encode(payload),
			r: Some(reply_subject.clone()),
		};
		let env_payload = serde_json::to_string(&env)?;
		// NOTIFY doesn't support parameterized queries
		let escaped_payload = env_payload.replace('\'', "''");
		let notify_sql = format!(
			"NOTIFY {}, '{}'",
			Self::quote_ident(&subject),
			escaped_payload
		);
		publish_conn.batch_execute(&notify_sql).await?;

		// Wait for response with optional timeout
		let response_future = async {
			match response_rx.recv().await {
				Some(payload_str) => {
					let env: Envelope = serde_json::from_str(&payload_str)?;
					let bytes = BASE64.decode(env.p).context("invalid base64 payload")?;
					Ok(Response { payload: bytes })
				}
				None => Err(anyhow!("subscription closed")),
			}
		};

		// Apply timeout if specified
		if let Some(dur) = timeout {
			match tokio::time::timeout(dur, response_future).await {
				std::result::Result::Ok(resp) => resp,
				std::result::Result::Err(_) => Err(errors::Ups::RequestTimeout.build().into()),
			}
		} else {
			response_future.await
		}
	}

	async fn send_request_reply(&self, reply: &str, payload: &[u8]) -> Result<()> {
		// Get a connection from the pool
		let conn = self
			.pool
			.get()
			.await
			.context("failed to get connection from pool")?;

		// Convert subject to base64 hash string because Postgres identifiers can only be 63 bytes
		let mut hasher = DefaultHasher::new();
		reply.hash(&mut hasher);
		let reply_subject = BASE64.encode(&hasher.finish().to_be_bytes());

		// Publish reply without nested reply
		let env = Envelope {
			p: BASE64.encode(payload),
			r: None,
		};
		let payload = serde_json::to_string(&env)?;
		// NOTIFY doesn't support parameterized queries
		let escaped_payload = payload.replace('\'', "''");
		let sql = format!(
			"NOTIFY {}, '{}'",
			Self::quote_ident(&reply_subject),
			escaped_payload
		);
		conn.batch_execute(&sql).await?;
		Ok(())
	}
}

// Special driver for handling local replies
struct LocalReplyDriver {
	reply_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
}

#[async_trait]
impl PubSubDriver for LocalReplyDriver {
	async fn subscribe(&self, _subject: &str) -> Result<SubscriberDriverHandle> {
		Err(anyhow!("LocalReplyDriver does not support subscribe"))
	}

	async fn publish(&self, _subject: &str, _message: &[u8]) -> Result<()> {
		Err(anyhow!("LocalReplyDriver does not support publish"))
	}

	async fn flush(&self) -> Result<()> {
		Ok(())
	}

	async fn request(
		&self,
		_subject: &str,
		_payload: &[u8],
		_timeout: Option<Duration>,
	) -> Result<Response> {
		Err(anyhow!("LocalReplyDriver does not support request"))
	}

	async fn send_request_reply(&self, _reply: &str, payload: &[u8]) -> Result<()> {
		tracing::debug!("sending local request reply");

		// Send the reply through the local channel
		let _ = self.reply_tx.send(payload.to_vec()).await;

		Ok(())
	}
}

pub struct PostgresSubscriber {
	driver: PostgresDriver,
	rx: tokio::sync::mpsc::UnboundedReceiver<String>,
	local_request_rx: Option<tokio::sync::broadcast::Receiver<LocalRequest>>,
	lock_id: i64,
	client: Arc<tokio_postgres::Client>,
	subject: String,
}

#[async_trait]
impl SubscriberDriver for PostgresSubscriber {
	#[tracing::instrument(skip(self), fields(subject = %self.subject, lock_id = %self.lock_id))]
	async fn next(&mut self) -> Result<NextOutput> {
		tracing::debug!("waiting for message");

		// If we have a local request receiver, poll both channels
		if let Some(ref mut local_rx) = self.local_request_rx {
			tokio::select! {
				// Check for local requests (memory fast path)
				local_req = local_rx.recv() => {
					match local_req {
						std::result::Result::Ok(req) => {
							// Create a synthetic reply subject for local request
							let reply_subject = format!("_LOCAL.{}", uuid::Uuid::new_v4());

							// Create a wrapper driver that will handle the reply
							let local_driver = LocalReplyDriver {
								reply_tx: req.reply_tx,
							};

							tracing::debug!(len=?req.payload.len(), "received local message");

							// Return the request as a message with the local reply driver
							Ok(NextOutput::Message(Message {
								driver: Arc::new(local_driver),
								payload: req.payload,
								reply: Some(reply_subject),
							}))
						}
						std::result::Result::Err(_) => {
							tracing::debug!("no local subscription senders");

							Ok(NextOutput::Unsubscribed)
						}
					}
				}
				// Check for regular PostgreSQL messages
				msg = self.rx.recv() => {
					match msg {
						Some(payload_str) => {
							let env: Envelope = serde_json::from_str(&payload_str)?;
							let bytes = BASE64.decode(env.p).context("invalid base64 payload")?;

							tracing::debug!(len=?bytes.len(), "received message");

							Ok(NextOutput::Message(Message {
								driver: Arc::new(self.driver.clone()),
								payload: bytes,
								reply: env.r,
							}))
						}
						None => {
							tracing::debug!(?self.subject, ?self.lock_id, "subscription closed");

							Ok(NextOutput::Unsubscribed)
						}
					}
				}
			}
		} else {
			// No memory optimization, just poll regular messages
			match self.rx.recv().await {
				Some(payload_str) => {
					let env: Envelope = serde_json::from_str(&payload_str)?;
					let bytes = BASE64.decode(env.p).context("invalid base64 payload")?;

					tracing::debug!(len=?bytes.len(), "received message");

					Ok(NextOutput::Message(Message {
						driver: Arc::new(self.driver.clone()),
						payload: bytes,
						reply: env.r,
					}))
				}
				None => {
					tracing::debug!("subscription closed");

					Ok(NextOutput::Unsubscribed)
				}
			}
		}
	}
}

impl Drop for PostgresSubscriber {
	fn drop(&mut self) {
		tracing::debug!(subject = %self.subject, ?self.lock_id, "dropping postgres subscriber");
		// Release the advisory lock when the subscriber is dropped
		let lock_id = self.lock_id;
		let client = self.client.clone();

		// Clean up local subscription registration if memory optimization is enabled
		if self.local_request_rx.is_some() {
			let subject = self.subject.clone();
			let local_subs = self.driver.local_subscriptions.clone();

			tokio::spawn(async move {
				let mut subs = local_subs.write().await;
				if let Some(local_tx) = subs.get_mut(&subject) {
					// If no more subscriptions for this subject, remove the entry
					if local_tx.tx.receiver_count() == 0 {
						subs.remove(&subject);
					}
				}
			});
		}

		// We need to release the lock in a blocking context since Drop is not async
		// Spawn a task to release the lock
		tokio::spawn(async move {
			let _ = client
				.execute("SELECT pg_advisory_unlock_shared($1)", &[&lock_id])
				.await;
		});
	}
}
