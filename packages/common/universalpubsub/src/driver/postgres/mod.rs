use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use anyhow::*;
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD as BASE64;
use deadpool_postgres::{Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime};
use futures_util::future::poll_fn;
use moka::future::Cache;
use tokio_postgres::{AsyncMessage, NoTls};
use tracing::Instrument;

use crate::driver::{PubSubDriver, SubscriberDriver, SubscriberDriverHandle};
use crate::pubsub::DriverOutput;

#[derive(Clone)]
struct Subscription {
	// Channel to send messages to this subscription
	tx: tokio::sync::broadcast::Sender<Vec<u8>>,
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
	client: Arc<tokio_postgres::Client>,
	subscriptions: Cache<String, Subscription>,
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
							let bytes = match BASE64.decode(note.payload()) {
								std::result::Result::Ok(b) => b,
								std::result::Result::Err(err) => {
									tracing::error!(?err, "failed decoding base64");
									break;
								}
							};
							let _ = sub.tx.send(bytes);
						}
					}
					Some(std::result::Result::Ok(_)) => {
						// Ignore other async messages
					}
					Some(std::result::Result::Err(err)) => {
						tracing::error!(?err, "async postgres error");
						break;
					}
					None => {
						tracing::debug!("async postgres connection closed");
						break;
					}
				}
			}
			tracing::debug!("listen connection closed");
		});

		Ok(Self {
			pool: Arc::new(pool),
			client: Arc::new(client),
			subscriptions,
		})
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
		// TODO: To match NATS implementation, LIST must be pipelined (i.e. wait for the command
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
		let rx = if let Some(existing_sub) = self.subscriptions.get(&hashed).await {
			// Reuse the existing broadcast channel
			existing_sub.tx.subscribe()
		} else {
			// Create a new broadcast channel for this subject
			let (tx, rx) = tokio::sync::broadcast::channel(1024);
			let subscription = Subscription { tx: tx.clone() };

			// Register subscription
			self.subscriptions
				.insert(hashed.clone(), subscription)
				.await;

			// Execute LISTEN command on the async client (for receiving notifications)
			// This only needs to be done once per channel
			let span = tracing::trace_span!("pg_listen");
			self.client
				.execute(&format!("LISTEN \"{hashed}\""), &[])
				.instrument(span)
				.await?;

			rx
		};

		Ok(Box::new(PostgresSubscriber {
			subject: subject.to_string(),
			rx,
		}))
	}

	async fn publish(&self, subject: &str, payload: &[u8]) -> Result<()> {
		// TODO: See `subscribe` about pipelining

		// Encode payload to base64 and send NOTIFY
		let encoded = BASE64.encode(payload);
		let conn = self.pool.get().await?;
		let hashed = self.hash_subject(subject);
		let span = tracing::trace_span!("pg_notify");
		conn.execute(&format!("NOTIFY \"{hashed}\", '{encoded}'"), &[])
			.instrument(span)
			.await?;

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
	rx: tokio::sync::broadcast::Receiver<Vec<u8>>,
}

#[async_trait]
impl SubscriberDriver for PostgresSubscriber {
	async fn next(&mut self) -> Result<DriverOutput> {
		match self.rx.recv().await {
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
