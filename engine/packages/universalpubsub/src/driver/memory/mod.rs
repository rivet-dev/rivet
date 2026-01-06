use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use anyhow::*;
use async_trait::async_trait;
use scc::HashMap;
use tokio::sync::mpsc;

use crate::driver::{PubSubDriver, SubscriberDriver, SubscriberDriverHandle};
use crate::metrics;
use crate::pubsub::DriverOutput;

type Subscribers = HashMap<String, Vec<mpsc::UnboundedSender<Vec<u8>>>>;

/// This is arbitrary.
const MEMORY_MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10MiB
const GC_INTERVAL: Duration = Duration::from_secs(60);

pub struct MemoryDriverInner {
	channel: String,
	subscribers: Subscribers,
}

#[derive(Clone)]
pub struct MemoryDriver(Arc<MemoryDriverInner>);

impl Deref for MemoryDriver {
	type Target = MemoryDriverInner;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl MemoryDriver {
	pub fn new(channel: String) -> Self {
		let inner = Arc::new(MemoryDriverInner {
			channel,
			subscribers: HashMap::new(),
		});

		// Spawn GC task to clean up closed subscribers
		let gc_inner = Arc::downgrade(&inner);
		tokio::spawn(async move {
			let mut interval = tokio::time::interval(GC_INTERVAL);
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			loop {
				interval.tick().await;
				if let Some(inner) = gc_inner.upgrade() {
					// Clean up closed senders
					inner
						.subscribers
						.retain_async(|_, senders| {
							// Retain only senders that are not closed
							senders.retain(|sender| !sender.is_closed());
							// Remove the entire subject entry if no senders remain
							!senders.is_empty()
						})
						.await;
					metrics::MEMORY_SUBSCRIBERS_COUNT.set(inner.subscribers.len() as i64);
				} else {
					break;
				}
			}
		});

		Self(inner)
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

		self.subscribers
			.entry_async(subject_with_channel.clone())
			.await
			.or_default()
			.push(tx);
		metrics::MEMORY_SUBSCRIBERS_COUNT.set(self.subscribers.len() as i64);

		Ok(Box::new(MemorySubscriber {
			subject: subject_with_channel,
			rx,
		}))
	}

	async fn publish(&self, subject: &str, payload: &[u8]) -> Result<()> {
		let subject_with_channel = self.subject_with_channel(subject);

		if let Some(subs) = self.subscribers.get_async(&subject_with_channel).await {
			for tx in &*subs {
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
