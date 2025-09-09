use anyhow::*;
use async_nats::Client;
use async_trait::async_trait;
use futures_util::StreamExt;

use crate::driver::{PubSubDriver, SubscriberDriver, SubscriberDriverHandle};
use crate::pubsub::DriverOutput;

/// > The size is set to 1 MB by default, but can be increased up to 64 MB if needed (though we recommend keeping the max message size to something more reasonable like 8 MB).
///
/// https://docs.nats.io/reference/faq#is-there-a-message-size-limitation-in-nats
///
/// When they say "MB" they mean "MiB." Ignorance strikes again.
pub const NATS_MAX_MESSAGE_SIZE: usize = 1024 * 1024;

#[derive(Clone)]
pub struct NatsDriver {
	client: Client,
}

impl NatsDriver {
	pub async fn connect(
		options: async_nats::ConnectOptions,
		server_addrs: impl async_nats::ToServerAddrs,
	) -> Result<Self> {
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
		Ok(Box::new(NatsSubscriber { subscriber }))
	}

	async fn publish(&self, subject: &str, payload: &[u8]) -> Result<()> {
		self.client
			.publish(subject.to_string(), payload.to_vec().into())
			.await?;
		Ok(())
	}

	async fn flush(&self) -> Result<()> {
		self.client.flush().await?;
		Ok(())
	}

	fn max_message_size(&self) -> usize {
		NATS_MAX_MESSAGE_SIZE
	}
}

pub struct NatsSubscriber {
	subscriber: async_nats::Subscriber,
}

#[async_trait]
impl SubscriberDriver for NatsSubscriber {
	async fn next(&mut self) -> Result<DriverOutput> {
		match self.subscriber.next().await {
			Some(msg) => Ok(DriverOutput::Message {
				subject: msg.subject.to_string(),
				payload: msg.payload.to_vec(),
			}),
			None => Ok(DriverOutput::Unsubscribed),
		}
	}
}

impl Drop for NatsSubscriber {
	fn drop(&mut self) {
		let _ = self.subscriber.unsubscribe();
	}
}
