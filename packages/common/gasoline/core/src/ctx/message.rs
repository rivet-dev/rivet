use std::{
	fmt::{self, Debug},
	marker::PhantomData,
	sync::Arc,
};

use rivet_pools::UpsPool;
use rivet_util::Id;
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::Instrument;
use universalpubsub::{NextOutput, Subscriber};

use crate::{
	error::{WorkflowError, WorkflowResult},
	message::{Message, NatsMessage, NatsMessageWrapper},
	utils::{self, tags::AsTags},
};

#[derive(Clone)]
pub struct MessageCtx {
	/// The connection used to communicate with NATS.
	nats: UpsPool,

	ray_id: Id,

	config: rivet_config::Config,
}

impl MessageCtx {
	#[tracing::instrument(skip_all, fields(%ray_id))]
	pub fn new(
		config: &rivet_config::Config,
		pools: &rivet_pools::Pools,
		_cache: &rivet_cache::Cache,
		ray_id: Id,
	) -> WorkflowResult<Self> {
		Ok(MessageCtx {
			nats: pools.ups().map_err(WorkflowError::PoolsGeneric)?,
			ray_id,
			config: config.clone(),
		})
	}
}

// MARK: Publishing messages
impl MessageCtx {
	/// Publishes a message to NATS and to a durable message stream if a topic is
	/// set.
	///
	/// Use `subscribe` to consume these messages ephemerally and `tail` to read
	/// the most recently sent message.
	///
	/// This spawns a background task that calls `message_wait` internally and does not wait for the message to
	/// finish publishing. This is done since there are very few cases where a
	/// service should need to wait or fail if a message does not publish
	/// successfully.
	#[tracing::instrument(skip_all, fields(message=M::NAME))]
	pub async fn message<M>(
		&self,
		tags: impl AsTags + 'static,
		message_body: M,
	) -> WorkflowResult<()>
	where
		M: Message,
	{
		let client = self.clone();
		let spawn_res = tokio::task::Builder::new()
			.name("gasoline::message_async")
			.spawn(
				async move {
					match client.message_wait::<M>(tags, message_body).await {
						Ok(_) => {}
						Err(err) => {
							tracing::error!(?err, "failed to publish message");
						}
					}
				}
				.instrument(tracing::info_span!("message_bg")),
			);
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn message_async task");
		}

		Ok(())
	}

	/// Same as `message` but waits for the message to successfully publish.
	///
	/// This is useful in scenarios where we need to publish a large amount of
	/// messages at once so we put the messages in a queue instead of submitting
	/// a large number of tasks to Tokio at once.
	#[tracing::instrument(skip_all, fields(message = M::NAME))]
	pub async fn message_wait<M>(&self, tags: impl AsTags, message_body: M) -> WorkflowResult<()>
	where
		M: Message,
	{
		let nats_subject = M::nats_subject();
		let duration_since_epoch = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or_else(|err| unreachable!("time is broken: {}", err));
		let ts = duration_since_epoch.as_millis() as i64;

		// Serialize the body
		let body_buf =
			serde_json::to_string(&message_body).map_err(WorkflowError::SerializeMessage)?;
		let body_buf_len = body_buf.len();
		let body_buf = serde_json::value::RawValue::from_string(body_buf)
			.map_err(WorkflowError::SerializeMessage)?;

		// Serialize message
		let req_id = Id::new_v1(self.config.dc_label());
		let message = NatsMessageWrapper {
			req_id,
			ray_id: self.ray_id,
			tags: tags.as_tags()?,
			ts,
			body: &body_buf,
		};
		let message_buf = serde_json::to_vec(&message).map_err(WorkflowError::SerializeMessage)?;

		tracing::debug!(
			%nats_subject,
			body_bytes = ?body_buf_len,
			message_bytes = ?message_buf.len(),
			"publish message"
		);

		// It's important to write to the stream as fast as possible in order to
		// ensure messages are handled quickly.
		let message_buf = Arc::new(message_buf);
		self.message_publish_nats::<M>(&nats_subject, message_buf)
			.await;

		Ok(())
	}

	/// Publishes the message to NATS.
	#[tracing::instrument(level = "debug", skip_all)]
	async fn message_publish_nats<M>(&self, nats_subject: &str, message_buf: Arc<Vec<u8>>)
	where
		M: Message,
	{
		// Infinite backoff since we want to wait until the service reboots.
		let mut backoff = rivet_util::backoff::Backoff::default_infinite();
		loop {
			// Ignore for infinite backoff
			backoff.tick().await;

			let nats_subject = nats_subject.to_owned();

			tracing::trace!(
				%nats_subject,
				message_len = message_buf.len(),
				"publishing message to nats"
			);
			if let Err(err) = self.nats.publish(&nats_subject, &(*message_buf)).await {
				tracing::warn!(?err, "publish message failed, trying again");
				continue;
			}

			tracing::debug!("publish nats message succeeded");
			break;
		}
	}
}

impl MessageCtx {
	pub fn config(&self) -> &rivet_config::Config {
		&self.config
	}
}

// MARK: Subscriptions
impl MessageCtx {
	/// Listens for gasoline messages globally on NATS.
	#[tracing::instrument(skip_all, fields(message = M::NAME))]
	pub async fn subscribe<M>(&self, tags: impl AsTags) -> WorkflowResult<SubscriptionHandle<M>>
	where
		M: Message,
	{
		self.subscribe_opt::<M>(SubscribeOpts {
			tags: tags.as_tags()?,
			flush_nats: true,
		})
		.in_current_span()
		.await
	}

	/// Listens for gasoline messages globally on NATS.
	#[tracing::instrument(skip_all, fields(message = M::NAME))]
	pub async fn subscribe_opt<M>(
		&self,
		opts: SubscribeOpts,
	) -> WorkflowResult<SubscriptionHandle<M>>
	where
		M: Message,
	{
		let nats_subject = M::nats_subject();

		// Create subscription and flush immediately.
		tracing::debug!(%nats_subject, tags = ?opts.tags, "creating subscription");
		let subscription = self
			.nats
			.subscribe(&nats_subject)
			.await
			.map_err(|x| WorkflowError::CreateSubscription(x.into()))?;
		if opts.flush_nats {
			self.nats
				.flush()
				.await
				.map_err(|x| WorkflowError::FlushNats(x.into()))?;
		}

		// Return handle
		let subscription = SubscriptionHandle::new(nats_subject, subscription, opts.tags.clone());
		Ok(subscription)
	}
}

#[derive(Debug)]
pub struct SubscribeOpts {
	pub tags: serde_json::Value,
	pub flush_nats: bool,
}

/// Used to receive messages from other contexts.
///
/// This subscription will automatically close when dropped.
pub struct SubscriptionHandle<M>
where
	M: Message,
{
	_message: PhantomData<M>,
	_guard: DropGuard,
	subject: String,
	subscription: Subscriber,
	tags: serde_json::Value,
}

impl<M> Debug for SubscriptionHandle<M>
where
	M: Message,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SubscriptionHandle")
			.field("subject", &self.subject)
			.field("tags", &self.tags)
			.finish()
	}
}

impl<M> SubscriptionHandle<M>
where
	M: Message,
{
	#[tracing::instrument(level = "debug", skip_all)]
	fn new(subject: String, subscription: Subscriber, tags: serde_json::Value) -> Self {
		let token = CancellationToken::new();

		{
			let token = token.clone();
			let spawn_res = tokio::task::Builder::new()
				.name("gasoline::message_wait_drop")
				.spawn(
					async move {
						token.cancelled().await;

						tracing::trace!("closing subscription");

						// We don't worry about calling `subscription.drain()` since the
						// entire subscription wrapper is dropped anyways, so we can't
						// call `.recv()`.
					}
					.instrument(tracing::trace_span!("subscription_wait_drop")),
				);
			if let Err(err) = spawn_res {
				tracing::error!(?err, "failed to spawn message_wait_drop task");
			}
		}

		SubscriptionHandle {
			_message: Default::default(),
			_guard: token.drop_guard(),
			subject,
			subscription,
			tags,
		}
	}

	/// Waits for the next message in the subscription.
	///
	/// This future can be safely dropped.
	#[tracing::instrument(name="message_next", skip_all, fields(message = M::NAME))]
	pub async fn next(&mut self) -> WorkflowResult<NatsMessage<M>> {
		tracing::debug!("waiting for message");

		loop {
			// Poll the subscription.
			//
			// Use blocking threads instead of `try_next`, since I'm not sure
			// try_next works as intended.
			let nats_message = match self.subscription.next().await {
				Ok(NextOutput::Message(msg)) => msg,
				Ok(NextOutput::Unsubscribed) => {
					tracing::debug!("unsubscribed");
					return Err(WorkflowError::SubscriptionUnsubscribed);
				}
				Err(err) => {
					tracing::warn!(?err, "subscription error");
					return Err(WorkflowError::CreateSubscription(err.into()));
				}
			};

			let message_wrapper = NatsMessage::<M>::deserialize_wrapper(&nats_message.payload)?;

			// Check if the subscription tags match a subset of the message tags
			if utils::is_value_subset(&self.tags, &message_wrapper.tags) {
				let message = NatsMessage::<M>::deserialize_from_wrapper(message_wrapper)?;
				tracing::debug!(?message, "received message");

				return Ok(message);
			}

			// Message tags don't match, continue with loop
		}
	}

	/// Converts the subscription in to a stream.
	pub fn into_stream(self) -> impl futures_util::Stream<Item = WorkflowResult<NatsMessage<M>>> {
		futures_util::stream::try_unfold(self, |mut sub| {
			async move {
				let message = sub.next().await?;
				Ok(Some((message, sub)))
			}
			.in_current_span()
		})
	}
}
