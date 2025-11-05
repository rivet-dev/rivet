use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::*;
use tokio::sync::broadcast;
use tokio::sync::{RwLock, oneshot};
use uuid::Uuid;

use rivet_util::backoff::Backoff;

use crate::chunking::{ChunkTracker, encode_chunk, split_payload_into_chunks};
use crate::driver::{PubSubDriverHandle, PublishOpts, SubscriberDriverHandle};

pub struct PubSubInner {
	driver: PubSubDriverHandle,
	chunk_tracker: Mutex<ChunkTracker>,
	reply_subscribers: RwLock<HashMap<String, oneshot::Sender<Vec<u8>>>>,
	// Local in-memory subscribers by subject (shared across all drivers)
	local_subscribers: RwLock<HashMap<String, broadcast::Sender<Vec<u8>>>>,
	// Enables/disables local fast-path across all drivers
	memory_optimization: bool,
}

#[derive(Clone)]
pub struct PubSub(Arc<PubSubInner>);

impl Deref for PubSub {
	type Target = PubSubInner;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl PubSub {
	pub fn new(driver: PubSubDriverHandle) -> Self {
		Self::new_with_memory_optimization(driver, true)
	}

	pub fn new_with_memory_optimization(
		driver: PubSubDriverHandle,
		memory_optimization: bool,
	) -> Self {
		let inner = Arc::new(PubSubInner {
			driver,
			chunk_tracker: Mutex::new(ChunkTracker::new()),
			reply_subscribers: RwLock::new(HashMap::new()),
			local_subscribers: RwLock::new(HashMap::new()),
			memory_optimization,
		});

		// Spawn GC task for chunk buffers
		let gc_inner = Arc::downgrade(&inner);
		tokio::spawn(async move {
			let mut interval = tokio::time::interval(crate::chunking::CHUNK_BUFFER_GC_INTERVAL);
			loop {
				interval.tick().await;
				if let Some(inner) = gc_inner.upgrade() {
					inner.chunk_tracker.lock().unwrap().gc();
				} else {
					break;
				}
			}
		});

		Self(inner)
	}

	pub async fn subscribe(&self, subject: &str) -> Result<Subscriber> {
		// Underlying driver subscription
		let driver = self.driver.subscribe(subject).await?;

		if !self.memory_optimization {
			return Ok(Subscriber::new(driver, self.clone()));
		}

		// Ensure a local broadcast channel exists for this subject
		let rx = {
			// Try read first for fast path
			if let Some(sender) = self.local_subscribers.read().await.get(subject) {
				sender.subscribe()
			} else {
				// Create and insert
				let (tx, rx) = broadcast::channel(1024);
				let mut map = self.local_subscribers.write().await;
				// Double-check in case of race
				let rx = if let Some(existing) = map.get(subject) {
					existing.subscribe()
				} else {
					map.insert(subject.to_string(), tx);
					rx
				};
				rx
			}
		};

		// Wrap the driver
		let optimized_driver: SubscriberDriverHandle = Box::new(LocalOptimizedSubscriberDriver {
			subject: subject.to_string(),
			driver,
			local_rx: rx,
		});

		Ok(Subscriber::new(optimized_driver, self.clone()))
	}

	pub async fn publish(&self, subject: &str, payload: &[u8], opts: PublishOpts) -> Result<()> {
		let message_id = *Uuid::new_v4().as_bytes();
		let chunks =
			split_payload_into_chunks(payload, self.driver.max_message_size(), message_id, None)?;
		let chunk_count = chunks.len() as u32;

		let use_local = self
			.should_use_local_subscriber(subject, opts.behavior)
			.await;

		for (chunk_idx, chunk_payload) in chunks.into_iter().enumerate() {
			let encoded = encode_chunk(
				chunk_payload,
				chunk_idx as u32,
				chunk_count,
				message_id,
				None,
			)?;

			if use_local {
				if let Some(sender) = self.local_subscribers.read().await.get(subject) {
					let _ = sender.send(encoded);
				} else {
					tracing::warn!(%subject, "local subscriber disappeared");
					break;
				}
			} else {
				// Use backoff when publishing through the driver
				self.publish_with_backoff(subject, &encoded).await?;
			}
		}
		Ok(())
	}

	pub async fn publish_with_reply(
		&self,
		subject: &str,
		payload: &[u8],
		reply_subject: &str,
		opts: PublishOpts,
	) -> Result<()> {
		let message_id = *Uuid::new_v4().as_bytes();
		let chunks = split_payload_into_chunks(
			payload,
			self.driver.max_message_size(),
			message_id,
			Some(reply_subject),
		)?;
		let chunk_count = chunks.len() as u32;

		let use_local = self
			.should_use_local_subscriber(subject, opts.behavior)
			.await;

		for (chunk_idx, chunk_payload) in chunks.into_iter().enumerate() {
			let encoded = encode_chunk(
				chunk_payload,
				chunk_idx as u32,
				chunk_count,
				message_id,
				Some(reply_subject.to_string()),
			)?;

			if use_local {
				if let Some(sender) = self.local_subscribers.read().await.get(subject) {
					let _ = sender.send(encoded);
				} else {
					tracing::warn!(%subject, "local subscriber disappeared");
					break;
				}
			} else {
				// Use backoff when publishing through the driver
				self.publish_with_backoff(subject, &encoded).await?;
			}
		}
		Ok(())
	}

	async fn publish_with_backoff(&self, subject: &str, encoded: &[u8]) -> Result<()> {
		let mut backoff = Backoff::default();
		loop {
			match self.driver.publish(subject, encoded).await {
				Result::Ok(_) => break,
				Err(err) if !backoff.tick().await => {
					tracing::warn!(?err, "error publishing, cannot retry again");
					return Err(crate::errors::Ups::PublishFailed.build().into());
				}
				Err(err) => {
					tracing::debug!(?err, "error publishing, retrying");
					// Continue retrying
				}
			}
		}
		Ok(())
	}

	pub async fn flush(&self) -> Result<()> {
		self.driver.flush().await
	}

	pub async fn request(&self, subject: &str, payload: &[u8]) -> Result<Response> {
		self.request_with_timeout(subject, payload, Duration::from_secs(30))
			.await
	}

	pub async fn request_with_timeout(
		&self,
		subject: &str,
		payload: &[u8],
		timeout: Duration,
	) -> Result<Response> {
		// Create a unique reply subject for this request
		let reply_subject = format!("_INBOX.{}", Uuid::new_v4());

		// Create a oneshot channel for the response
		let (tx, rx) = oneshot::channel();

		// Register the reply handler
		{
			let mut subscribers = self.reply_subscribers.write().await;
			subscribers.insert(reply_subject.clone(), tx);
		}

		// Subscribe to the reply subject (use local-aware subscribe)
		let mut reply_subscriber = self.subscribe(&reply_subject).await?;

		// Send the request with the reply subject, using local fast-path
		self.publish_with_reply(subject, payload, &reply_subject, PublishOpts::one())
			.await?;

		// Spawn a task to wait for the reply
		let inner = self.0.clone();
		let reply_subject_clone = reply_subject.clone();
		tokio::spawn(async move {
			loop {
				match reply_subscriber.next().await {
					std::result::Result::Ok(NextOutput::Message(msg)) => {
						// Already decoded; forward payload
						let mut subscribers = inner.reply_subscribers.write().await;
						if let Some(tx) = subscribers.remove(&reply_subject_clone) {
							let _ = tx.send(msg.payload);
						}
						break;
					}
					std::result::Result::Ok(NextOutput::Unsubscribed)
					| std::result::Result::Err(_) => break,
				}
			}
		});

		// Wait for response with timeout
		let response = match tokio::time::timeout(timeout, rx).await {
			std::result::Result::Ok(std::result::Result::Ok(payload)) => Response { payload },
			std::result::Result::Ok(std::result::Result::Err(_)) => {
				// Clean up the reply subscription
				self.reply_subscribers.write().await.remove(&reply_subject);
				return Err(crate::errors::Ups::RequestTimeout.build().into());
			}
			std::result::Result::Err(_) => {
				// Timeout elapsed
				self.reply_subscribers.write().await.remove(&reply_subject);
				return Err(crate::errors::Ups::RequestTimeout.build().into());
			}
		};

		Ok(response)
	}

	async fn should_use_local_subscriber(
		&self,
		subject: &str,
		behavior: crate::driver::PublishBehavior,
	) -> bool {
		// Local fast-path for one-subscriber behavior:
		// - When memory_optimization is enabled and behavior == OneSubscriber, deliver directly
		//   to any in-process subscribers via the subject's broadcast channel and skip calling
		//   the underlying driver (avoids network hops and driver overhead).
		// - For Broadcast, always publish via the driver so remote subscribers (and other
		//   processes) receive the message; local subscribers will also receive via the driver.
		// - If there are no local receivers at the time of publish (or the channel disappears),
		//   fall back to the driver publish path.

		if !self.memory_optimization {
			return false;
		}
		if !matches!(behavior, crate::driver::PublishBehavior::OneSubscriber) {
			return false;
		}
		if let Some(sender) = self.local_subscribers.read().await.get(subject) {
			sender.receiver_count() > 0
		} else {
			false
		}
	}
}

pub struct Subscriber {
	driver: SubscriberDriverHandle,
	pubsub: PubSub,
}

impl Subscriber {
	pub fn new(driver: SubscriberDriverHandle, pubsub: PubSub) -> Self {
		Self { driver, pubsub }
	}

	pub async fn next(&mut self) -> Result<NextOutput> {
		loop {
			match self.driver.next().await? {
				DriverOutput::Message {
					subject: _,
					payload,
				} => {
					// Process chunks
					let mut tracker = self.pubsub.chunk_tracker.lock().unwrap();
					match tracker.process_chunk(&payload) {
						std::result::Result::Ok(Some((payload, reply_subject))) => {
							return Ok(NextOutput::Message(Message {
								pubsub: self.pubsub.clone(),
								payload,
								reply: reply_subject,
							}));
						}
						std::result::Result::Ok(None) => continue, // Waiting for more chunks
						std::result::Result::Err(e) => {
							tracing::warn!(?e, "failed to process chunk");
							continue;
						}
					}
				}
				DriverOutput::Unsubscribed => return Ok(NextOutput::Unsubscribed),
			}
		}
	}
}

// Output from drivers (raw binary messages)
pub enum DriverOutput {
	Message { subject: String, payload: Vec<u8> },
	Unsubscribed,
}

// Output from subscriber (after chunking/decoding)
pub enum NextOutput {
	Message(Message),
	Unsubscribed,
}

pub struct Message {
	pub pubsub: PubSub,
	pub payload: Vec<u8>,
	pub reply: Option<String>,
}

impl Message {
	pub async fn reply(&self, payload: &[u8]) -> Result<()> {
		if let Some(ref reply_subject) = self.reply {
			// Replies expect exactly one subscriber and should use local fast-path
			self.pubsub
				.publish(reply_subject, payload, PublishOpts::one())
				.await?;
		}
		Ok(())
	}
}

pub struct Response {
	pub payload: Vec<u8>,
}

/// Internal composite subscriber that merges driver messages with local in-memory messages
struct LocalOptimizedSubscriberDriver {
	subject: String,
	driver: SubscriberDriverHandle,
	local_rx: broadcast::Receiver<Vec<u8>>,
}

#[async_trait::async_trait]
impl crate::driver::SubscriberDriver for LocalOptimizedSubscriberDriver {
	async fn next(&mut self) -> Result<DriverOutput> {
		loop {
			tokio::select! {
				biased;
				// Prefer local messages to reduce latency
				res = self.local_rx.recv() => {
					match res {
						std::result::Result::Ok(payload) => {
							return Ok(DriverOutput::Message { subject: self.subject.clone(), payload });
						}
						std::result::Result::Err(broadcast::error::RecvError::Lagged(_)) => {
							// Skip lagged and continue
							continue;
						}
						std::result::Result::Err(broadcast::error::RecvError::Closed) => {
							// Local channel closed; fall back to driver only
							// Replace with a closed receiver to avoid busy loop
							// We simply continue and rely on driver
						}
					}
				}
				res = self.driver.next() => {
					return res;
				}
			}
		}
	}
}
