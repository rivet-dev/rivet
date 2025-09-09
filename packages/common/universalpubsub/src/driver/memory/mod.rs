use std::collections::HashMap;
use std::sync::Arc;

use anyhow::*;
use async_trait::async_trait;
use tokio::sync::{RwLock, mpsc};

use crate::driver::{PubSubDriver, SubscriberDriver, SubscriberDriverHandle};
use crate::pubsub::DriverOutput;

type Subscribers = Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<Vec<u8>>>>>>;

/// This is arbitrary.
const MEMORY_MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10MiB

#[derive(Clone)]
pub struct MemoryDriver {
	channel: String,
	subscribers: Subscribers,
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
			subject: subject_with_channel,
			rx,
		}))
	}

	async fn publish(&self, subject: &str, payload: &[u8]) -> Result<()> {
		let subject_with_channel = self.subject_with_channel(subject);
		let subscribers = self.subscribers.read().await;

		if let Some(subs) = subscribers.get(&subject_with_channel) {
			for tx in subs {
				let _ = tx.send(payload.to_vec());
			}
		}

		Ok(())
	}

	async fn flush(&self) -> Result<()> {
		Ok(())
	}

	fn max_message_size(&self) -> usize {
		MEMORY_MAX_MESSAGE_SIZE
	}
}

pub struct MemorySubscriber {
	subject: String,
	rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

#[async_trait]
impl SubscriberDriver for MemorySubscriber {
	async fn next(&mut self) -> Result<DriverOutput> {
		match self.rx.recv().await {
			Some(payload) => Ok(DriverOutput::Message {
				subject: self.subject.clone(),
				payload,
			}),
			None => Ok(DriverOutput::Unsubscribed),
		}
	}
}
