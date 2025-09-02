use std::sync::Arc;
use std::time::Duration;

use anyhow::*;
use async_trait::async_trait;

pub mod memory;
pub mod nats;
pub mod postgres;

use crate::pubsub::{NextOutput, Response};

pub type PubSubDriverHandle = Arc<dyn PubSubDriver>;

#[async_trait]
pub trait PubSubDriver: Send + Sync {
	async fn subscribe(&self, subject: &str) -> Result<Box<dyn SubscriberDriver>>;
	async fn publish(&self, subject: &str, message: &[u8]) -> Result<()>;
	async fn request(
		&self,
		subject: &str,
		payload: &[u8],
		timeout: Option<Duration>,
	) -> Result<Response>;
	async fn send_request_reply(&self, reply: &str, payload: &[u8]) -> Result<()>;
	async fn flush(&self) -> Result<()>;
}

pub type SubscriberDriverHandle = Box<dyn SubscriberDriver>;

#[async_trait]
pub trait SubscriberDriver: Send + Sync {
	async fn next(&mut self) -> Result<NextOutput>;
}
