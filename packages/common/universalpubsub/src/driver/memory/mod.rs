use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::*;
use async_trait::async_trait;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::driver::{PubSubDriver, SubscriberDriver, SubscriberDriverHandle};
use crate::pubsub::{Message, NextOutput, Response};

type Subscribers = Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<MemoryMessage>>>>>;

#[derive(Clone, Debug)]
struct MemoryMessage {
	payload: Vec<u8>,
	reply_to: Option<String>,
}

#[derive(Clone)]
pub struct MemoryDriver {
	channel: String,
	subscribers: Subscribers,
	// No pending requests tracking needed for simple in-memory behavior
}

impl MemoryDriver {
	pub fn new(channel: String) -> Self {
		Self {
			channel,
			subscribers: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	fn subject_with_channel(&self, subject: &str) -> String {
		format!("{}::{}", self.channel, subject)
	}
}

#[async_trait]
impl PubSubDriver for MemoryDriver {
	async fn subscribe(&self, subject: &str) -> Result<SubscriberDriverHandle> {
		let (tx, rx) = mpsc::unbounded_channel();
		let subject_with_channel = self.subject_with_channel(subject);

		let mut subscribers = self.subscribers.write().await;
		subscribers
			.entry(subject_with_channel.clone())
			.or_default()
			.push(tx);

		Ok(Box::new(MemorySubscriber {
			driver: self.clone(),
			rx,
		}))
	}

	async fn publish(&self, subject: &str, message: &[u8]) -> Result<()> {
		let subject_with_channel = self.subject_with_channel(subject);
		let subscribers = self.subscribers.read().await;

		if let Some(subs) = subscribers.get(&subject_with_channel) {
			let msg = MemoryMessage {
				payload: message.to_vec(),
				reply_to: None,
			};
			for tx in subs {
				let _ = tx.send(msg.clone());
			}
		}

		Ok(())
	}

	async fn flush(&self) -> Result<()> {
		Ok(())
	}

	async fn request(
		&self,
		subject: &str,
		payload: &[u8],
		timeout: Option<Duration>,
	) -> Result<Response> {
		let subject_with_channel = self.subject_with_channel(subject);

		// Check if there are any subscribers for this subject
		{
			let subscribers = self.subscribers.read().await;
			if !subscribers.contains_key(&subject_with_channel)
				|| subscribers
					.get(&subject_with_channel)
					.map_or(true, |subs| subs.is_empty())
			{
				return Err(crate::errors::Ups::NoResponders.build().into());
			}
		}

		// Create a unique reply subject for this request
		let reply_subject = format!("_INBOX.{}", Uuid::new_v4());
		let reply_subject_with_channel = self.subject_with_channel(&reply_subject);

		// Create a oneshot channel for the response
		let (tx, rx) = tokio::sync::oneshot::channel();

		// Subscribe to the reply subject to receive the response
		{
			let mut subscribers = self.subscribers.write().await;
			let (reply_tx, mut reply_rx) = mpsc::unbounded_channel();
			subscribers
				.entry(reply_subject_with_channel.clone())
				.or_default()
				.push(reply_tx);

			// Spawn a task to wait for the reply
			tokio::spawn(async move {
				if let Some(msg) = reply_rx.recv().await {
					let _ = tx.send(Response {
						payload: msg.payload,
					});
				}
			});
		}

		// Send the request message with reply_to field to first subscriber (if any)
		{
			let subscribers = self.subscribers.read().await;
			if let Some(subs) = subscribers.get(&subject_with_channel) {
				if let Some(tx) = subs.first() {
					let msg = MemoryMessage {
						payload: payload.to_vec(),
						reply_to: Some(reply_subject.clone()),
					};
					let _ = tx.send(msg);
				}
			}
		}

		// Wait for response with optional timeout
		let response = if let Some(timeout_duration) = timeout {
			match tokio::time::timeout(timeout_duration, rx).await {
				std::result::Result::Ok(result) => match result {
					std::result::Result::Ok(response) => response,
					std::result::Result::Err(_) => {
						// Channel error - shouldn't happen in normal operation
						return Err(crate::errors::Ups::RequestTimeout.build().into());
					}
				},
				std::result::Result::Err(_) => {
					// Timeout elapsed
					// Clean up the reply subscription
					let subscribers = self.subscribers.clone();
					let reply_subject = reply_subject_with_channel.clone();
					tokio::spawn(async move {
						let mut subs = subscribers.write().await;
						subs.remove(&reply_subject);
					});
					return Err(crate::errors::Ups::RequestTimeout.build().into());
				}
			}
		} else {
			match rx.await {
				std::result::Result::Ok(response) => response,
				std::result::Result::Err(_) => {
					// Channel closed without response - shouldn't happen
					return Err(anyhow!("Request failed: no response received"));
				}
			}
		};

		Ok(response)
	}

	async fn send_request_reply(&self, reply: &str, payload: &[u8]) -> Result<()> {
		self.publish(reply, payload).await
	}
}

pub struct MemorySubscriber {
	driver: MemoryDriver,
	rx: mpsc::UnboundedReceiver<MemoryMessage>,
}

#[async_trait]
impl SubscriberDriver for MemorySubscriber {
	async fn next(&mut self) -> Result<NextOutput> {
		match self.rx.recv().await {
			Some(msg) => Ok(NextOutput::Message(Message {
				driver: Arc::new(self.driver.clone()),
				payload: msg.payload,
				reply: msg.reply_to,
			})),
			None => Ok(NextOutput::Unsubscribed),
		}
	}
}
