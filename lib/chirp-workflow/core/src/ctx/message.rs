use std::{
	fmt::{self, Debug},
	marker::PhantomData,
	sync::Arc,
};

use futures_util::StreamExt;
use rivet_pools::prelude::redis::AsyncCommands;
use rivet_pools::prelude::*;
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::Instrument;
use uuid::Uuid;

use crate::{
	error::{WorkflowError, WorkflowResult},
	message::{self, Message, MessageWrapper, ReceivedMessage, TraceEntry},
};

/// Time (in ms) that we subtract from the anchor grace period in order to
/// validate that there is not a race condition between the anchor validity and
/// writing to Redis.
const TAIL_ANCHOR_VALID_GRACE: i64 = 250;

#[derive(Clone)]
pub struct MessageCtx {
	/// The connection used to communicate with NATS.
	nats: NatsPool,

	/// Used for writing to message tails. This cache is ephemeral.
	redis_chirp_ephemeral: RedisPool,

	req_id: Uuid,
	ray_id: Uuid,
	trace: Vec<TraceEntry>,
}

impl MessageCtx {
	pub async fn new(
		conn: &rivet_connection::Connection,
		req_id: Uuid,
		ray_id: Uuid,
	) -> WorkflowResult<Self> {
		Ok(MessageCtx {
			nats: conn.nats().await?,
			redis_chirp_ephemeral: conn.redis_chirp_ephemeral().await?,
			req_id,
			ray_id,
			trace: conn
				.chirp()
				.trace()
				.iter()
				.cloned()
				.map(TryInto::try_into)
				.collect::<WorkflowResult<Vec<_>>>()?,
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
	#[tracing::instrument(err, skip_all, fields(message = M::NAME))]
	pub async fn message<M>(&self, tags: serde_json::Value, message_body: M) -> WorkflowResult<()>
	where
		M: Message,
	{
		let client = self.clone();
		let spawn_res = tokio::task::Builder::new()
			.name("chirp_workflow::message_async")
			.spawn(
				async move {
					match client.message_wait::<M>(tags, message_body).await {
						Ok(_) => {}
						Err(err) => {
							tracing::error!(?err, "failed to publish message");
						}
					}
				}
				.in_current_span(),
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
	#[tracing::instrument(err, skip_all, fields(message = M::NAME))]
	pub async fn message_wait<M>(
		&self,
		tags: serde_json::Value,
		message_body: M,
	) -> WorkflowResult<()>
	where
		M: Message,
	{
		let tags_str = cjson::to_string(&tags).map_err(WorkflowError::SerializeMessageTags)?;
		let nats_subject = message::serialize_message_nats_subject::<M>(&tags_str);
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
		let req_id = Uuid::new_v4();
		let message = MessageWrapper {
			req_id: req_id,
			ray_id: self.ray_id,
			tags: tags.clone(),
			ts,
			trace: self.trace.clone(),
			allow_recursive: false, // TODO:
			body: &body_buf,
		};
		let message_buf = serde_json::to_vec(&message).map_err(WorkflowError::SerializeMessage)?;

		// TODO: opts.dont_log_body
		if true {
			tracing::info!(
				%nats_subject,
				body_bytes = ?body_buf_len,
				message_bytes = ?message_buf.len(),
				"publish message"
			);
		} else {
			tracing::info!(
				%nats_subject,
				?message_body,
				body_bytes = ?body_buf_len,
				message_bytes = ?message_buf.len(),
				"publish message"
			);
		}

		// Write to Redis and NATS.
		//
		// It's important to write to the stream as fast as possible in order to
		// ensure messages are handled quickly.
		let message_buf = Arc::new(message_buf);
		self.message_write_redis::<M>(&tags_str, message_buf.clone(), req_id, ts)
			.await;
		self.message_publish_nats::<M>(&nats_subject, message_buf)
			.await;

		Ok(())
	}

	/// Writes a message to a Redis durable stream and tails.
	#[tracing::instrument(level = "debug", skip_all)]
	async fn message_write_redis<M>(
		&self,
		tags_str: &str,
		message_buf: Arc<Vec<u8>>,
		req_id: Uuid,
		ts: i64,
	) where
		M: Message,
	{
		// Write tail
		let tail_key = redis_keys::message_tail::<M>(tags_str);

		let mut pipe = redis::pipe();

		// Save message
		pipe.hset(
			&tail_key,
			redis_keys::message_tail::REQUEST_ID,
			req_id.to_string(),
		)
		.ignore();
		pipe.hset(&tail_key, redis_keys::message_tail::TS, ts)
			.ignore();
		pipe.hset(
			&tail_key,
			redis_keys::message_tail::BODY,
			message_buf.as_slice(),
		)
		.ignore();

		let mut conn = self.redis_chirp_ephemeral.clone();
		match pipe.query_async::<_, ()>(&mut conn).await {
			Ok(_) => {
				tracing::debug!("write to redis tail succeeded");
			}
			Err(err) => {
				tracing::error!(?err, "failed to write to redis tail");
			}
		}

		// Automatically expire
		pipe.expire(&tail_key, M::TAIL_TTL.as_millis() as usize)
			.ignore();
	}

	/// Publishes the message to NATS.
	#[tracing::instrument(level = "debug", skip_all)]
	async fn message_publish_nats<M>(&self, nats_subject: &str, message_buf: Arc<Vec<u8>>)
	where
		M: Message,
	{
		// Publish message to NATS. Do this after a successful write to
		// Redis in order to verify that tailing messages doesn't end up in a
		// race condition that misses a message from the database.
		//
		// Infinite backoff since we want to wait until the service reboots.
		let mut backoff = rivet_util::Backoff::default_infinite();
		loop {
			// Ignore for infinite backoff
			backoff.tick().await;

			let nats_subject = nats_subject.to_owned();

			tracing::trace!(
				%nats_subject,
				message_len = message_buf.len(),
				"publishing message to nats"
			);
			if let Err(err) = self
				.nats
				.publish(nats_subject.clone(), (*message_buf).clone().into())
				.await
			{
				tracing::warn!(?err, "publish message failed, trying again");
				continue;
			}

			tracing::debug!("publish nats message succeeded");
			break;
		}
	}
}

// MARK: Subscriptions
impl MessageCtx {
	/// Listens for Chirp workflow messages globally on NATS.
	#[tracing::instrument(level = "debug", err, skip_all)]
	pub async fn subscribe<M>(
		&self,
		tags: &serde_json::Value,
	) -> WorkflowResult<SubscriptionHandle<M>>
	where
		M: Message,
	{
		self.subscribe_opt::<M>(SubscribeOpts {
			tags,
			flush_nats: true,
		})
		.await
	}

	/// Listens for Chirp workflow messages globally on NATS.
	#[tracing::instrument(err, skip_all, fields(message = M::NAME))]
	pub async fn subscribe_opt<M>(
		&self,
		opts: SubscribeOpts<'_>,
	) -> WorkflowResult<SubscriptionHandle<M>>
	where
		M: Message,
	{
		let tags_str = cjson::to_string(opts.tags).map_err(WorkflowError::SerializeMessageTags)?;
		let nats_subject = message::serialize_message_nats_subject::<M>(&tags_str);

		// Create subscription and flush immediately.
		tracing::info!(%nats_subject, tags = ?opts.tags, "creating subscription");
		let subscription = self
			.nats
			.subscribe(nats_subject.clone())
			.await
			.map_err(|x| WorkflowError::CreateSubscription(x.into()))?;
		if opts.flush_nats {
			self.nats
				.flush()
				.await
				.map_err(|x| WorkflowError::FlushNats(x.into()))?;
		}

		// Return handle
		let subscription = SubscriptionHandle::new(nats_subject, subscription, self.req_id);
		Ok(subscription)
	}

	/// Reads the tail message of a stream without waiting for a message.
	#[tracing::instrument(err, skip_all, fields(message = M::NAME))]
	pub async fn tail_read<M>(
		&self,
		tags: serde_json::Value,
	) -> WorkflowResult<Option<ReceivedMessage<M>>>
	where
		M: Message,
	{
		let mut conn = self.redis_chirp_ephemeral.clone();

		// Fetch message
		let tags_str = cjson::to_string(&tags).map_err(WorkflowError::SerializeMessageTags)?;
		let tail_key = redis_keys::message_tail::<M>(&tags_str);
		let message_buf = conn
			.hget::<_, _, Option<Vec<u8>>>(&tail_key, redis_keys::message_tail::BODY)
			.await?;

		// Deserialize message
		let message = if let Some(message_buf) = message_buf {
			let message = ReceivedMessage::<M>::deserialize(message_buf.as_slice())?;
			tracing::info!(?message, "immediate read tail message");

			let recv_lag = (rivet_util::timestamp::now() as f64 - message.ts as f64) / 1000.;
			crate::metrics::MESSAGE_RECV_LAG
				.with_label_values(&[M::NAME])
				.observe(recv_lag);

			Some(message)
		} else {
			tracing::info!("no tail message to read");
			None
		};

		Ok(message)
	}

	/// Used by API services to tail an message (by start time) after a given timestamp.
	///
	/// Because this waits indefinitely until next message, it is recommended to use this inside
	/// of a `rivet_util::macros::select_with_timeout!` block:
	/// ```rust
	/// use rivet_util as util;
	///
	/// let message_sub = tail_anchor!([ctx, anchor] message_test());
	///
	/// // Consumes anchor or times out after 1 minute
	/// util::macros::select_with_timeout!(
	/// 	message = message_sub => {
	/// 		let _message = message?;
	/// 	}
	/// );
	/// ```
	#[tracing::instrument(err, skip_all, fields(message = M::NAME))]
	pub async fn tail_anchor<M>(
		&self,
		tags: serde_json::Value,
		anchor: &TailAnchor,
	) -> WorkflowResult<TailAnchorResponse<M>>
	where
		M: Message,
	{
		// Validate anchor is valid
		if !anchor.is_valid(M::TAIL_TTL.as_millis() as i64) {
			return Ok(TailAnchorResponse::AnchorExpired);
		}

		// Create subscription. Do this before reading from the log in order to
		// ensure consistency.
		//
		// Leave flush enabled in order to ensure that subscription is
		// registered with NATS before continuing.
		let mut sub = self.subscribe(&tags).await?;

		// Read the tail log
		let tail_read = self.tail_read(tags).await?;

		// Check if valid or wait for subscription
		let (message, source) = match tail_read {
			Some(message) if message.ts > anchor.start_time => (message, "tail_read"),
			_ => {
				// Wait for next message if tail not present
				let message = sub.next().await?;
				(message, "subscription")
			}
		};

		tracing::info!(?message, %source, ?anchor, "read tail message");

		Ok(TailAnchorResponse::Message(message))
	}
}

#[derive(Debug)]
pub struct SubscribeOpts<'a> {
	pub tags: &'a serde_json::Value,
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
	subscription: nats::Subscriber,
	req_id: Uuid,
}

impl<M> Debug for SubscriptionHandle<M>
where
	M: Message,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SubscriptionHandle")
			.field("subject", &self.subject)
			.finish()
	}
}

impl<M> SubscriptionHandle<M>
where
	M: Message,
{
	#[tracing::instrument(level = "debug", skip_all)]
	fn new(subject: String, subscription: nats::Subscriber, req_id: Uuid) -> Self {
		let token = CancellationToken::new();

		{
			let token = token.clone();
			let spawn_res = tokio::task::Builder::new()
				.name("chirp_workflow::message_wait_drop")
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
			req_id,
		}
	}

	/// Waits for the next message in the subscription.
	///
	/// This future can be safely dropped.
	#[tracing::instrument]
	pub async fn next(&mut self) -> WorkflowResult<ReceivedMessage<M>> {
		self.next_inner(false).await
	}

	// TODO: Add a full config struct to pass to `next` that impl's `Default`
	/// Waits for the next message in the subscription that originates from the
	/// parent request ID via trace.
	///
	/// This future can be safely dropped.
	#[tracing::instrument]
	pub async fn next_with_trace(
		&mut self,
		filter_trace: bool,
	) -> WorkflowResult<ReceivedMessage<M>> {
		self.next_inner(filter_trace).await
	}

	/// This future can be safely dropped.
	#[tracing::instrument(level = "trace")]
	async fn next_inner(&mut self, filter_trace: bool) -> WorkflowResult<ReceivedMessage<M>> {
		tracing::info!("waiting for message");

		loop {
			// Poll the subscription.
			//
			// Use blocking threads instead of `try_next`, since I'm not sure
			// try_next works as intended.
			let nats_message = match self.subscription.next().await {
				Some(x) => x,
				None => {
					tracing::debug!("unsubscribed");
					return Err(WorkflowError::SubscriptionUnsubscribed);
				}
			};

			if filter_trace {
				let message_wrapper =
					ReceivedMessage::<M>::deserialize_wrapper(&nats_message.payload[..])?;

				// Check if the message trace stack originates from this client
				//
				// We intentionally use the request ID instead of just checking the ray ID because
				// there may be multiple calls to `message_with_subscribe` within the same ray.
				// Explicitly checking the parent request ensures the response is unique to this
				// message.
				if message_wrapper
					.trace
					.iter()
					.rev()
					.any(|trace_entry| trace_entry.req_id == self.req_id)
				{
					let message = ReceivedMessage::<M>::deserialize(&nats_message.payload[..])?;
					tracing::info!(?message, "received message");

					return Ok(message);
				}
			} else {
				let message = ReceivedMessage::<M>::deserialize(&nats_message.payload[..])?;
				tracing::info!(?message, "received message");

				let recv_lag = (rivet_util::timestamp::now() as f64 - message.ts as f64) / 1000.;
				crate::metrics::MESSAGE_RECV_LAG
					.with_label_values(&[M::NAME])
					.observe(recv_lag);

				return Ok(message);
			}

			// Message not from parent, continue with loop
		}
	}

	/// Converts the subscription in to a stream.
	pub fn into_stream(
		self,
	) -> impl futures_util::Stream<Item = WorkflowResult<ReceivedMessage<M>>> {
		futures_util::stream::try_unfold(self, |mut sub| async move {
			let message = sub.next().await?;
			Ok(Some((message, sub)))
		})
	}
}

#[derive(Debug, Clone)]
pub struct TailAnchor {
	pub start_time: i64,
}

impl TailAnchor {
	pub fn new(start_time: i64) -> Self {
		TailAnchor { start_time }
	}

	pub fn is_valid(&self, ttl: i64) -> bool {
		self.start_time > rivet_util::timestamp::now() - ttl * 1000 - TAIL_ANCHOR_VALID_GRACE
	}
}

#[derive(Debug)]
pub enum TailAnchorResponse<M>
where
	M: Message + Debug,
{
	Message(ReceivedMessage<M>),

	/// Anchor was older than the TTL of the message.
	AnchorExpired,
}

impl<M> TailAnchorResponse<M>
where
	M: Message + Debug,
{
	/// Returns the timestamp of the message if exists.
	///
	/// Useful for endpoints that need to return a new anchor.
	pub fn msg_ts(&self) -> Option<i64> {
		match self {
			Self::Message(msg) => Some(msg.msg_ts()),
			Self::AnchorExpired => None,
		}
	}
}

mod redis_keys {
	use std::{
		collections::hash_map::DefaultHasher,
		hash::{Hash, Hasher},
	};

	use crate::message::Message;

	/// HASH
	pub fn message_tail<M>(tags_str: &str) -> String
	where
		M: Message,
	{
		// Get hash of the tags
		let mut hasher = DefaultHasher::new();
		tags_str.hash(&mut hasher);

		format!("{{topic:{}:{:x}}}:tail", M::NAME, hasher.finish())
	}

	pub mod message_tail {
		pub const REQUEST_ID: &str = "r";
		pub const TS: &str = "t";
		pub const BODY: &str = "b";
	}
}
