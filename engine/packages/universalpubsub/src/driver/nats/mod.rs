use anyhow::{Result, bail};
use async_nats::Client;
use async_trait::async_trait;
use futures_util::StreamExt;
use scc::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::InboxSubject;
use crate::driver::{PubSubDriver, SubscriberDriver, SubscriberDriverHandle};
use crate::pubsub::DriverOutput;

/// > The size is set to 1 MB by default, but can be increased up to 64 MB if needed (though we recommend keeping the max message size to something more reasonable like 8 MB).
///
/// https://docs.nats.io/reference/faq#is-there-a-message-size-limitation-in-nats
///
/// When they say "MB" they mean "MiB." Ignorance strikes again.
pub const NATS_MAX_MESSAGE_SIZE: usize = 1024 * 1024;
static NODE_ID: OnceLock<Uuid> = OnceLock::new();
/// Global map of all request reply subscribers in this process.
static REPLY_SUBSCRIBERS: OnceLock<Arc<HashMap<Uuid, oneshot::Sender<async_nats::Message>>>> =
	OnceLock::new();

#[derive(Clone)]
pub struct NatsDriver {
	client: Client,
	reply_subscribers: Arc<HashMap<Uuid, oneshot::Sender<async_nats::Message>>>,
}

impl NatsDriver {
	pub async fn connect(
		options: async_nats::ConnectOptions,
		server_addrs: impl async_nats::ToServerAddrs,
	) -> Result<Self> {
		tracing::info!("nats connecting");
		let client = options.connect(server_addrs).await?;
		tracing::info!("nats connected");

		let client2 = client.clone();
		let reply_subscribers = REPLY_SUBSCRIBERS
			.get_or_init(|| {
				let reply_subscribers = Arc::new(HashMap::new());

				// Spawn the reply handler task.
				let reply_subscribers2 = reply_subscribers.clone();
				tokio::spawn(async move {
					loop {
						if let Err(err) =
							reply_sub_handler(client2.clone(), reply_subscribers2.clone()).await
						{
							tracing::error!("reply sub handler failed: {err:?}");
						}
					}
				});

				reply_subscribers
			})
			.clone();

		Ok(Self {
			client,
			reply_subscribers,
		})
	}

	pub fn statistics(&self) -> Arc<async_nats::Statistics> {
		self.client.statistics()
	}
}

#[async_trait]
impl PubSubDriver for NatsDriver {
	async fn subscribe(
		&self,
		subject: &str,
		reply_id: Option<Uuid>,
	) -> Result<SubscriberDriverHandle> {
		if let Some(reply_id) = reply_id {
			let (tx, rx) = oneshot::channel();
			self.reply_subscribers.upsert_async(reply_id, tx).await;
			Ok(Box::new(NatsReplySubscriber {
				reply_subscribers: self.reply_subscribers.clone(),
				reply_id,
				rx: Some(rx),
			}))
		} else {
			let subscriber = self.client.subscribe(subject.to_string()).await?;
			Ok(Box::new(NatsSubscriber { subscriber }))
		}
	}

	async fn queue_subscribe(&self, subject: &str, queue: &str) -> Result<SubscriberDriverHandle> {
		let subscriber = self
			.client
			.queue_subscribe(subject.to_string(), queue.to_string())
			.await?;
		Ok(Box::new(NatsSubscriber { subscriber }))
	}

	async fn publish(
		&self,
		subject: &str,
		payload: &[u8],
		reply_subject: Option<&str>,
	) -> Result<()> {
		// When `reply_subject` is set, we rely on the NATS server's built-in no-responders
		// behavior. Since NATS 2.2, a publish carrying a reply subject to a topic with no
		// subscribers causes the server to deliver an empty status-503 message to subscribers
		// of the reply subject (see `NatsSubscriber::next` for the matching status-code
		// branch). This is server-side, not a client `request()` API feature, so it works
		// uniformly regardless of how the reply subject was attached. If you target a NATS
		// server older than 2.2, `NextOutput::NoResponders` will never fire and request
		// callers will hit the timeout path instead.
		if let Some(reply_subject) = reply_subject {
			self.client
				.publish_with_reply(
					subject.to_string(),
					reply_subject.to_string(),
					payload.to_vec().into(),
				)
				.await?;
		} else {
			self.client
				.publish(subject.to_string(), payload.to_vec().into())
				.await?;
		}

		Ok(())
	}

	async fn flush(&self) -> Result<()> {
		self.client.flush().await?;
		Ok(())
	}

	fn max_message_size(&self) -> usize {
		NATS_MAX_MESSAGE_SIZE
	}

	fn new_inbox(&self) -> InboxSubject {
		InboxSubject::new_with_node_id(*NODE_ID.get_or_init(Uuid::new_v4))
	}
}

pub struct NatsSubscriber {
	subscriber: async_nats::Subscriber,
}

#[async_trait]
impl SubscriberDriver for NatsSubscriber {
	async fn next(&mut self) -> Result<DriverOutput> {
		match self.subscriber.next().await {
			Some(msg) => match msg.status {
				None => Ok(DriverOutput::Message {
					subject: msg.subject.to_string(),
					payload: msg.payload.to_vec(),
				}),
				Some(async_nats::StatusCode::NO_RESPONDERS) => Ok(DriverOutput::NoResponders),
				Some(status) => {
					if let Some(description) = msg.description {
						bail!("unexpected status in nats message: {status} {description}");
					} else {
						bail!("unexpected status in nats message: {status}");
					}
				}
			},
			None => Ok(DriverOutput::Unsubscribed),
		}
	}
}

impl Drop for NatsSubscriber {
	fn drop(&mut self) {
		let _ = self.subscriber.unsubscribe();
	}
}

/// Local channel subscriber used for multiplexing request-reply.
pub struct NatsReplySubscriber {
	reply_id: Uuid,
	reply_subscribers: Arc<HashMap<Uuid, oneshot::Sender<async_nats::Message>>>,
	rx: Option<oneshot::Receiver<async_nats::Message>>,
}

#[async_trait]
impl SubscriberDriver for NatsReplySubscriber {
	async fn next(&mut self) -> Result<DriverOutput> {
		let Some(rx) = self.rx.take() else {
			bail!("reply subscriber has already returned a value");
		};

		match rx.await {
			Ok(msg) => match msg.status {
				None => Ok(DriverOutput::Message {
					subject: msg.subject.to_string(),
					payload: msg.payload.to_vec(),
				}),
				Some(async_nats::StatusCode::NO_RESPONDERS) => Ok(DriverOutput::NoResponders),
				Some(status) => {
					if let Some(description) = msg.description {
						bail!("unexpected status in nats message: {status} {description}");
					} else {
						bail!("unexpected status in nats message: {status}");
					}
				}
			},
			Err(_) => Ok(DriverOutput::Unsubscribed),
		}
	}
}

impl Drop for NatsReplySubscriber {
	fn drop(&mut self) {
		let reply_id = self.reply_id;
		let reply_subscribers = self.reply_subscribers.clone();
		tokio::spawn(async move {
			reply_subscribers.remove_async(&reply_id).await;
		});
	}
}

async fn reply_sub_handler(
	client: Client,
	reply_subscribers: Arc<HashMap<Uuid, oneshot::Sender<async_nats::Message>>>,
) -> Result<()> {
	let node_id = NODE_ID.get_or_init(Uuid::new_v4);
	let wildcard_subject = format!("{}.{node_id}.*", InboxSubject::prefix());

	// Unique sub per process
	let mut sub = client.subscribe(wildcard_subject).await?;

	loop {
		let Some(msg) = sub.next().await else {
			bail!("reply sub closed");
		};

		if let Some(subject) = InboxSubject::from_existing(msg.subject.as_str()) {
			if let Some((_, sub)) = reply_subscribers.remove_async(&subject.id).await {
				let _ = sub.send(msg);
			}
		}
	}
}
