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
	/// This does not limit it to a single subscriber, but instead enables in-memory optimizations
	/// to speed this up.
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
