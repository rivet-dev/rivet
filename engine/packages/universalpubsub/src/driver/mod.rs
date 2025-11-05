use std::sync::Arc;

use anyhow::*;
use async_trait::async_trait;

pub mod memory;
pub mod nats;
pub mod postgres;

pub type PubSubDriverHandle = Arc<dyn PubSubDriver>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublishBehavior {
	/// Publishes a message to a single subscriber.
	///
	/// This should not be used if there will ever be more than one subscription at a time to the given topic
	/// on a global scale. Its intended to enable in-memory optimizations where a subscription that exists on
	/// the same machine as the published message will not have to communicate with the driver but can instead
	/// be delivered entirely in-memory.
	OneSubscriber,

	/// Publishes a message to multiple subscribers.
	Broadcast,
}

#[derive(Clone, Copy, Debug)]
pub struct PublishOpts {
	pub behavior: PublishBehavior,
}

impl PublishOpts {
	pub const fn one() -> Self {
		Self {
			behavior: PublishBehavior::OneSubscriber,
		}
	}

	pub const fn broadcast() -> Self {
		Self {
			behavior: PublishBehavior::Broadcast,
		}
	}
}

#[async_trait]
pub trait PubSubDriver: Send + Sync {
	async fn subscribe(&self, subject: &str) -> Result<Box<dyn SubscriberDriver>>;
	async fn publish(&self, subject: &str, message: &[u8]) -> Result<()>;
	async fn flush(&self) -> Result<()>;
	fn max_message_size(&self) -> usize;
}

pub type SubscriberDriverHandle = Box<dyn SubscriberDriver>;

#[async_trait]
pub trait SubscriberDriver: Send + Sync {
	async fn next(&mut self) -> Result<crate::pubsub::DriverOutput>;
}
