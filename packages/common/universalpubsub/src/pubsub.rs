use std::time::Duration;

use anyhow::*;

use crate::driver::{PubSubDriverHandle, SubscriberDriverHandle};

#[derive(Clone)]
pub struct PubSub {
	driver: PubSubDriverHandle,
}

impl PubSub {
	pub fn new(driver: PubSubDriverHandle) -> Self {
		Self { driver }
	}

	pub async fn subscribe(&self, subject: &str) -> Result<Subscriber> {
		let subscriber_driver = self.driver.subscribe(subject).await?;
		Ok(Subscriber::new(subscriber_driver))
	}

	pub async fn publish(&self, subject: &str, payload: &[u8]) -> Result<()> {
		self.driver.publish(subject, payload).await
	}

	pub async fn flush(&self) -> Result<()> {
		self.driver.flush().await
	}

	pub async fn request(&self, subject: &str, payload: &[u8]) -> Result<Response> {
		self.driver.request(subject, payload, None).await
	}

	pub async fn request_with_timeout(
		&self,
		subject: &str,
		payload: &[u8],
		timeout: Duration,
	) -> Result<Response> {
		self.driver.request(subject, payload, Some(timeout)).await
	}
}

pub struct Subscriber {
	driver: SubscriberDriverHandle,
}

impl Subscriber {
	pub fn new(driver: SubscriberDriverHandle) -> Self {
		Self { driver }
	}

	pub async fn next(&mut self) -> Result<NextOutput> {
		self.driver.next().await
	}
}

pub enum NextOutput {
	Message(Message),
	Unsubscribed,
}

pub struct Message {
	pub driver: PubSubDriverHandle,
	pub payload: Vec<u8>,
	pub reply: Option<String>,
}

impl Message {
	pub async fn reply(&self, payload: &[u8]) -> Result<()> {
		if let Some(ref reply_subject) = self.reply {
			self.driver.send_request_reply(reply_subject, payload).await
		} else {
			Ok(())
		}
	}
}

pub struct Response {
	pub payload: Vec<u8>,
}
