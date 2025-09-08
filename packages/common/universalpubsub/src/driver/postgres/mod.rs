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
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tokio_postgres::{AsyncMessage, NoTls};
use tracing::Instrument;

use crate::driver::{PubSubDriver, SubscriberDriver, SubscriberDriverHandle};
use crate::errors;
use crate::pubsub::{Message, NextOutput, Response};

#[derive(Clone)]
struct Subscription {
	// Channel to send requests to this subscription
	tx: tokio::sync::broadcast::Sender<(Vec<u8>, Option<String>)>,
}

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
	memory_optimization: bool,
	pool: Arc<Pool>,
	client: Arc<tokio_postgres::Client>,

	subscriptions: Cache<String, Subscription>,

	// Maps subject to local subscription on this node for fast path
	local_subscriptions: Arc<RwLock<HashMap<String, LocalSubscription>>>,
}

#[derive(Serialize, Deserialize)]
struct Envelope {
	// Base64-encoded payload
	#[serde(rename = "p")]
	payload: String,
	#[serde(rename = "r", skip_serializing_if = "Option::is_none")]
	reply_subject: Option<String>,
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

		let subscriptions: Cache<String, Subscription> =
			Cache::builder().initial_capacity(5).build();
		let subscriptions2 = subscriptions.clone();

		let (client, mut conn) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;
		tokio::spawn(async move {
			// NOTE: This loop will stop automatically when client is dropped
			loop {
				match poll_fn(|cx| conn.poll_message(cx)).await {
					Some(std::result::Result::Ok(AsyncMessage::Notification(note))) => {
						if let Some(sub) = subscriptions2.get(note.channel()).await {
							let env = match serde_json::from_str::<Envelope>(&note.payload()) {
								std::result::Result::Ok(env) => env,
								std::result::Result::Err(err) => {
									tracing::error!(?err, "failed deserializing envelope");
									break;
								}
							};
							let payload = match BASE64
								.decode(env.payload)
								.context("invalid base64 payload")
							{
								std::result::Result::Ok(p) => p,
								std::result::Result::Err(err) => {
									tracing::error!(?err, "failed deserializing envelope");
									break;
								}
							};

							let _ = sub.tx.send((payload, env.reply_subject));
						}
					}
					Some(std::result::Result::Ok(_)) => continue,
					Some(std::result::Result::Err(err)) => {
						tracing::error!(?err, "ups poll loop failed");
						break;
					}
					None => break,
				}
			}

			tracing::info!("ups poll loop stopped");
		});

		Ok(Self {
			memory_optimization,
			pool: Arc::new(pool),
			client: Arc::new(client),
			subscriptions,
			local_subscriptions: Arc::new(RwLock::new(HashMap::new())),
		})
	}
}

#[async_trait]
impl PubSubDriver for PostgresDriver {
	#[tracing::instrument(skip(self), fields(subject))]
	async fn subscribe(&self, subject: &str) -> Result<SubscriberDriverHandle> {
		tracing::debug!(%subject, "starting subscription");

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
					tx: tokio::sync::broadcast::channel(64).0,
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

		// Get the lock ID for this subject
		let lock_id = subject_to_lock_id(subject);
		tracing::debug!(%subject, ?lock_id, "calculated advisory lock id");

		// Convert subject to base64 hash string because Postgres identifiers can only be 63 bytes
		let mut hasher = DefaultHasher::new();
		subject.hash(&mut hasher);
		let subject_hash = BASE64.encode(&hasher.finish().to_be_bytes());

		let rx = self
			.subscriptions
			.entry(subject_hash.clone())
			.or_insert_with(async {
				Subscription {
					tx: tokio::sync::broadcast::channel(128).0,
				}
			})
			.await
			.value()
			.tx
			.subscribe();

		let lock_sql = format!("SELECT pg_try_advisory_lock_shared({})", lock_id);
		let lock_res = self.client.query_one(&lock_sql, &[]).await?;
		let lock_acquired = lock_res.get::<_, bool>(0);
		ensure!(lock_acquired, "Failed to acquire advisory lock for subject");

		let sql = format!("LISTEN {}", quote_ident(&subject_hash));
		let listen_res = self.client.batch_execute(&sql).await;

		if listen_res.is_err() {
			// Release lock on error
			let _ = self
				.client
				.execute("SELECT pg_advisory_unlock_shared($1)", &[&lock_id])
				.await;
		}

		listen_res?;

		tracing::debug!(%subject, "subscription established successfully");
		Ok(Box::new(PostgresSubscriber {
			driver: self.clone(),
			rx,
			local_request_rx,
			lock_id,
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
		let subject_hash = BASE64.encode(&hasher.finish().to_be_bytes());

		// Encode payload
		let env = Envelope {
			payload: BASE64.encode(message),
			reply_subject: None,
		};
		let payload = serde_json::to_string(&env)?;

		// NOTIFY doesn't support parameterized queries, so we need to escape the payload
		// Replace single quotes with two single quotes for SQL escaping
		let escaped_payload = payload.replace('\'', "''");
		let sql = format!(
			"NOTIFY {}, '{}'",
			quote_ident(&subject_hash),
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
			if let Some(local_sub) = subs.get(subject) {
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
				if local_sub.tx.send(request).is_ok() {
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
		let lock_id = subject_to_lock_id(subject);

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

		let mut reply_sub = self.subscribe(&reply_subject).await?;

		// Get another connection from pool to publish the request
		let publish_conn = self
			.pool
			.get()
			.await
			.context("failed to get connection from pool")?;

		// Convert subject to base64 hash string because Postgres identifiers can only be 63 bytes
		let mut hasher = DefaultHasher::new();
		subject.hash(&mut hasher);
		let subject_hash = BASE64.encode(&hasher.finish().to_be_bytes());
		let mut hasher = DefaultHasher::new();
		reply_subject.hash(&mut hasher);
		let reply_subject_hash = BASE64.encode(&hasher.finish().to_be_bytes());

		// Publish request with reply subject encoded
		let env = Envelope {
			payload: BASE64.encode(payload),
			reply_subject: Some(reply_subject_hash.clone()),
		};
		let env_payload = serde_json::to_string(&env)?;

		// NOTIFY doesn't support parameterized queries
		let escaped_payload = env_payload.replace('\'', "''");

		let notify_sql = format!(
			"NOTIFY {}, '{}'",
			quote_ident(&subject_hash),
			escaped_payload
		);
		publish_conn.batch_execute(&notify_sql).await?;

		// Wait for response with optional timeout
		let response_future = async {
			Ok(Response {
				payload: match reply_sub.next().await? {
					NextOutput::Message(msg) => msg.payload,
					NextOutput::Unsubscribed => bail!("reply subscription unsubscribed"),
				},
			})
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

	// NOTE: The reply argument here is already a base64 encoded hash
	async fn send_request_reply(&self, reply: &str, payload: &[u8]) -> Result<()> {
		// Get a connection from the pool
		let conn = self
			.pool
			.get()
			.await
			.context("failed to get connection from pool")?;

		// Publish reply without nested reply
		let env = Envelope {
			payload: BASE64.encode(payload),
			reply_subject: None,
		};
		let payload = serde_json::to_string(&env)?;
		// NOTIFY doesn't support parameterized queries
		let escaped_payload = payload.replace('\'', "''");
		let sql = format!("NOTIFY {}, '{}'", quote_ident(reply), escaped_payload);
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
	rx: tokio::sync::broadcast::Receiver<(Vec<u8>, Option<String>)>,
	local_request_rx: Option<tokio::sync::broadcast::Receiver<LocalRequest>>,
	lock_id: i64,
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
						std::result::Result::Ok((payload, reply_subject)) => {
							tracing::debug!(len=?payload.len(), "received message");

							Ok(NextOutput::Message(Message {
								driver: Arc::new(self.driver.clone()),
								payload,
								reply: reply_subject,
							}))
						}
						std::result::Result::Err(_) => {
							tracing::debug!(?self.subject, ?self.lock_id, "subscription closed");

							Ok(NextOutput::Unsubscribed)
						}
					}
				}
			}
		} else {
			// No memory optimization, just poll regular messages
			match self.rx.recv().await {
				std::result::Result::Ok((payload, reply_subject)) => {
					tracing::debug!(len=?payload.len(), "received message");

					Ok(NextOutput::Message(Message {
						driver: Arc::new(self.driver.clone()),
						payload,
						reply: reply_subject,
					}))
				}
				std::result::Result::Err(_) => {
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

		let lock_id = self.lock_id;
		let driver = self.driver.clone();
		let subject = self.subject.clone();
		let has_local_rx = self.local_request_rx.is_some();

		// Spawn a task to release the lock
		tokio::spawn(async move {
			// Clean up local subscription registration if memory optimization is enabled
			if has_local_rx {
				let mut subs = driver.local_subscriptions.write().await;
				if let Some(local_sub) = subs.get_mut(&subject) {
					// If no more subscriptions for this subject, remove the entry
					if local_sub.tx.receiver_count() == 0 {
						subs.remove(&subject);
					}
				}
			}

			if let Some(sub) = driver.subscriptions.get(&subject).await {
				if sub.tx.receiver_count() == 0 {
					driver.subscriptions.invalidate(&subject).await;

					let mut hasher = DefaultHasher::new();
					subject.hash(&mut hasher);
					let subject_hash = BASE64.encode(&hasher.finish().to_be_bytes());

					let sql = format!("UNLISTEN {}", quote_ident(&subject_hash));
					let unlisten_res = driver.client.batch_execute(&sql).await;

					if let std::result::Result::Err(err) = unlisten_res {
						tracing::error!(%subject, ?err, "failed to unlisten subject");
					}
				}
			}

			let _ = driver
				.client
				.execute("SELECT pg_advisory_unlock_shared($1)", &[&lock_id])
				.await;
		});
	}
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
