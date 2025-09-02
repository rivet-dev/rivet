use std::sync::Arc;
use std::time::Duration;

use anyhow::*;
use async_nats::{Client, client::RequestErrorKind};
use async_trait::async_trait;
use futures_util::StreamExt;

use crate::driver::{PubSubDriver, SubscriberDriver, SubscriberDriverHandle};
use crate::errors;
use crate::pubsub::{Message, NextOutput, Response};

#[derive(Clone)]
pub struct NatsDriver {
	client: Client,
}

impl NatsDriver {
	pub async fn connect(
		options: async_nats::ConnectOptions,
		server_addrs: impl async_nats::ToServerAddrs,
	) -> Result<Self> {
		// NOTE: async-nats adds ConnectionInfo.no_responders by default

		tracing::debug!("nats connecting");
		let client = options.connect(server_addrs).await?;
		tracing::debug!("nats connected");
		Ok(Self { client })
	}
}

#[async_trait]
impl PubSubDriver for NatsDriver {
	async fn subscribe(&self, subject: &str) -> Result<SubscriberDriverHandle> {
		let subscriber = self.client.subscribe(subject.to_string()).await?;
		Ok(Box::new(NatsSubscriber {
			driver: self.clone(),
			subscriber,
		}))
	}

	async fn publish(&self, subject: &str, message: &[u8]) -> Result<()> {
		self.client
			.publish(subject.to_string(), message.to_vec().into())
			.await?;
		Ok(())
	}

	async fn flush(&self) -> Result<()> {
		self.client.flush().await?;
		Ok(())
	}

	async fn request(
		&self,
		subject: &str,
		payload: &[u8],
		timeout: Option<Duration>,
	) -> Result<Response> {
		let request_future = self
			.client
			.request(subject.to_string(), payload.to_vec().into());

		let request = if let Some(timeout) = timeout {
			match tokio::time::timeout(timeout, request_future).await {
				std::result::Result::Ok(result) => match result {
					std::result::Result::Ok(msg) => msg,
					std::result::Result::Err(err) => match err.kind() {
						RequestErrorKind::NoResponders => {
							return Err(errors::Ups::NoResponders.build().into());
						}
						RequestErrorKind::TimedOut => {
							return Err(errors::Ups::RequestTimeout.build().into());
						}
						_ => bail!(err),
					},
				},
				std::result::Result::Err(_) => {
					return Err(errors::Ups::RequestTimeout.build().into());
				}
			}
		} else {
			match request_future.await {
				std::result::Result::Ok(msg) => msg,
				std::result::Result::Err(err) => match err.kind() {
					RequestErrorKind::NoResponders => {
						return Err(errors::Ups::NoResponders.build().into());
					}
					RequestErrorKind::TimedOut => {
						return Err(errors::Ups::RequestTimeout.build().into());
					}
					_ => bail!(err),
				},
			}
		};

		Ok(Response {
			payload: request.payload.to_vec(),
		})
	}

	async fn send_request_reply(&self, reply: &str, payload: &[u8]) -> Result<()> {
		self.publish(reply, payload).await
	}
}

pub struct NatsSubscriber {
	driver: NatsDriver,
	subscriber: async_nats::Subscriber,
}

#[async_trait]
impl SubscriberDriver for NatsSubscriber {
	async fn next(&mut self) -> Result<NextOutput> {
		match self.subscriber.next().await {
			Some(msg) => Ok(NextOutput::Message(Message {
				driver: Arc::new(self.driver.clone()),
				payload: msg.payload.to_vec(),
				reply: msg.reply.map(|r| r.to_string()),
			})),
			None => Ok(NextOutput::Unsubscribed),
		}
	}
}

impl Drop for NatsSubscriber {
	fn drop(&mut self) {
		let _ = self.subscriber.unsubscribe();
	}
}
