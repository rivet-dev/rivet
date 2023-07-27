use chirp_types::{endpoint::Endpoint, message::Message};
use futures_util::stream::{StreamExt, TryStreamExt};
use global_error::prelude::*;
use rand::Rng;
use redis::{self, AsyncCommands};
use rivet_pools::prelude::*;
use rivet_util::Backoff;
use std::{
	collections::HashSet,
	env,
	fmt::{self, Debug},
	marker::PhantomData,
	sync::Arc,
	time::Duration,
};
use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::Instrument;
use types::rivet::chirp;
use uuid::Uuid;

use crate::{
	endpoint::{self, RpcResponse},
	error::ClientError,
	message::{self, ReceivedMessage},
	metrics, redis_keys,
};

/// Time (in ms) that we subtract from the anchor grace period in order to
/// validate that there is not a race condition between the anchor validity and
/// writing to Redis.
const TAIL_ANCHOR_VALID_GRACE: i64 = 250;

pub type SharedClientHandle = Arc<SharedClient>;

/// Global manager for communicating with other Chirp clients.
///
/// This is not used to make Chirp requests, but only to act as a manager for
/// other clients. To make Chirp requests, see `SharedClient::wrap` to acquire a
/// client with the appropriate context.
///
/// This is separated from the `Client` since each Chirp request needs to have
/// its own contextual information about who is making the request.
pub struct SharedClient {
	/// The connection used to communicate with NATS.
	nats: NatsPool,

	/// Used for writing to durable streams. This cache is persistent.
	redis_chirp: RedisPool,

	/// Used for caching values. This cache is ephemeral.
	redis_cache: RedisPool,

	/// The region of Chirp workers to communicate with.
	///
	/// We don't send RPC requests outside of this region, since there is a
	/// cluster of all Chirp workers in every region, so we'd be sending
	/// requests out of the datacenter for no reason.
	region: String,
}

impl SharedClient {
	pub fn new(
		nats: NatsPool,
		redis_chirp: RedisPool,
		redis_cache: RedisPool,
		region: String,
	) -> SharedClientHandle {
		let spawn_res = tokio::task::Builder::new()
			.name("chirp_client::metrics_update_update")
			.spawn(metrics::start_update_uptime());
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn user_presence_touch task");
		}

		Arc::new(SharedClient {
			nats,
			redis_chirp,
			redis_cache,
			region,
		})
	}

	#[tracing::instrument(skip(pools))]
	pub fn from_env(pools: rivet_pools::Pools) -> Result<SharedClientHandle, ClientError> {
		let region = env::var("CHIRP_REGION")
			.map_err(|_| ClientError::MissingEnvVar("CHIRP_REGION".into()))?;

		Ok(SharedClient::new(
			pools.nats()?,
			pools.redis_chirp()?,
			pools.redis_cache()?,
			region,
		))
	}

	pub fn wrap_new(self: Arc<Self>, context_name: &str) -> Client {
		let req_id = Uuid::new_v4();

		self.wrap(
			req_id,
			Uuid::new_v4(),
			vec![chirp::TraceEntry {
				context_name: context_name.into(),
				req_id: Some(req_id.into()),
				ts: rivet_util::timestamp::now(),
				run_context: match rivet_util::env::run_context() {
					rivet_util::env::RunContext::Service => chirp::RunContext::Service,
					rivet_util::env::RunContext::Test => chirp::RunContext::Test,
				} as i32,
			}],
		)
	}

	/// Creates a new `Client` with the appropriate context to make requests to
	/// other Chirp workers. See `Client` for more details.
	pub fn wrap(
		self: Arc<Self>,
		parent_req_id: Uuid,
		ray_id: Uuid,
		trace: Vec<chirp::TraceEntry>,
	) -> Client {
		// Not the same as the request's ts because this cannot be overridden by debug start ts
		let ts = rivet_util::timestamp::now();

		let redis_cache = self.redis_cache.clone();
		Client::new(
			self,
			parent_req_id,
			ray_id,
			trace,
			Arc::new(chirp_perf::PerfCtxInner::new(
				redis_cache,
				ts,
				parent_req_id,
				ray_id,
			)),
			ts,
			None,
		)
	}

	pub fn wrap_with(
		self: Arc<Self>,
		parent_req_id: Uuid,
		ray_id: Uuid,
		ts: i64,
		trace: Vec<chirp::TraceEntry>,
		perf_ctx: chirp_perf::PerfCtxInner,
		drop_fn: Option<DropFn>,
	) -> Client {
		Client::new(
			self,
			parent_req_id,
			ray_id,
			trace,
			Arc::new(perf_ctx),
			ts,
			drop_fn,
		)
	}

	pub fn region(&self) -> &str {
		&self.region
	}
}

type DropFn = Box<dyn Fn() + Send>;

/// Used to communicate with other Chirp clients.
///
/// This should be built from `SharedClient::wrap` in order to create the
/// necessary context.
#[derive(Clone)]
pub struct Client {
	_guard: Arc<DropGuard>,

	inner: SharedClientHandle,

	/// Request ID of the Chirp request that created this client.
	///
	/// i.e. this is the parent request ID to any request that will be made from
	/// this client.
	parent_req_id: Uuid,

	/// Ray ID that this client is associated with.
	///
	/// All actions made with this client will be associated with this ray. We
	/// can look up the ray ID later in order to collect metrics about a
	/// collection of requests.
	///
	/// Ray IDs are defined when some message enters the cluster. For example, API
	/// servers define new ray IDs when a client makes a new request. CRON jobs
	/// also make a new ray ID for each execution.
	ray_id: Uuid,

	/// A trace of all parent requests.
	trace: Arc<Vec<chirp::TraceEntry>>,

	perf_ctx: chirp_perf::PerfCtx,

	/// The timestamp that the Client is wrapped. Unrelated to any following
	/// Requests.
	ts: i64,
}

impl Debug for Client {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Client")
			.field("region", &self.region)
			.field("parent_req_id", &self.parent_req_id)
			.field("ray_id", &self.ray_id)
			.finish()
	}
}

impl std::ops::Deref for Client {
	type Target = Arc<SharedClient>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl Client {
	#[tracing::instrument(skip_all)]
	fn new(
		inner: SharedClientHandle,
		parent_req_id: Uuid,
		ray_id: Uuid,
		trace: Vec<chirp::TraceEntry>,
		perf_ctx: chirp_perf::PerfCtx,
		ts: i64,
		drop_fn: Option<DropFn>,
	) -> Client {
		let token = CancellationToken::new();

		// Submit performance on drop
		{
			let token = token.clone();
			let perf_ctx = perf_ctx.clone();
			let spawn_res = tokio::task::Builder::new()
				.name("chirp_client::client_wait_drop")
				.spawn(
					async move {
						token.cancelled().await;
						perf_ctx.submit().await;

						metrics::CHIRP_CLIENT_ACTIVE.dec();

						if let Some(drop_fn) = drop_fn {
							drop_fn();
						}
					}
					.instrument(tracing::info_span!("client_wait_drop")),
				);
			if let Err(err) = spawn_res {
				tracing::error!(?err, "failed to spawn client_wait_drop task");
			}
		}

		metrics::CHIRP_CLIENT_ACTIVE.inc();

		Client {
			_guard: Arc::new(token.drop_guard()),
			inner,
			parent_req_id,
			ray_id,
			trace: Arc::new(trace),
			perf_ctx,
			ts,
		}
	}

	pub fn parent_req_id(&self) -> Uuid {
		self.parent_req_id
	}

	pub fn ray_id(&self) -> Uuid {
		self.ray_id
	}

	#[deprecated(note = "use Client::perf")]
	pub fn perf_ctx(&self) -> &chirp_perf::PerfCtx {
		&self.perf_ctx
	}

	pub fn perf(&self) -> &chirp_perf::PerfCtx {
		&self.perf_ctx
	}

	pub fn ts(&self) -> i64 {
		self.ts
	}
}

impl Client {
	/// Calls another Chirp worker within the same region region and waits for a
	/// response.
	///
	/// We must always use RPCs instead of publish-and-forget for workers since NATS provides at
	/// least once delivery and does not provide durable queues. It's the job of the caller to
	/// make sure the message gets completed.
	///
	/// ## `request_timeout`
	/// How long to wait for a response from the worker in milliseconds. This should be
	/// greater than `worker_task_timeout` in order to gracefully receive a timeout
	/// response from the service.
	///
	/// This is important, since NATS only guarantees at least once deliveries, so it's up to
	/// Chirp to automatically retry requests.
	#[tracing::instrument(err, skip_all, fields(service = M::NAME))]
	pub async fn rpc<M>(
		&self,
		region_id: Option<&str>,
		req_body: M::Request,
		dont_log_body: bool,
	) -> GlobalResult<RpcResponse<M::Response>>
	where
		M: Endpoint,
	{
		let req_id = Uuid::new_v4();

		let rpc_perf = self.perf_ctx.start_rpc(M::NAME, req_id).await;

		let subject = endpoint::subject(region_id.unwrap_or_else(|| self.region.as_str()), M::NAME);
		let res = self
			.rpc_inner::<M>(req_id, subject.clone(), req_body, None, dont_log_body)
			.await
			.map_err(|err| match err {
				ClientError::GlobalError(error) => error,
				err => err.into(),
			});

		rpc_perf.end();

		res
	}

	/// See `Client::rpc` for information on RPC calls.
	///
	/// ## `req_debug`
	/// If calling from tests, `req_debug` allows customizing how the request is handled.
	#[tracing::instrument(err, skip_all, fields(service = M::NAME))]
	pub async fn rpc_debug<M>(
		&self,
		region_id: Option<&str>,
		req_body: M::Request,
		req_debug: Option<chirp::RequestDebug>,
		req_dont_log_body: bool,
	) -> GlobalResult<RpcResponse<M::Response>>
	where
		M: Endpoint,
	{
		let req_id = Uuid::new_v4();

		let rpc_perf = self.perf_ctx.start_rpc(M::NAME, req_id).await;

		let subject = endpoint::subject(region_id.unwrap_or_else(|| self.region.as_str()), M::NAME);
		let res = self
			.rpc_inner::<M>(
				req_id,
				subject.clone(),
				req_body,
				req_debug,
				req_dont_log_body,
			)
			.await
			.map_err(|err| match err {
				ClientError::GlobalError(error) => error,
				err => err.into(),
			});

		rpc_perf.end();

		res
	}

	#[tracing::instrument(level = "debug", skip(req_body))]
	async fn rpc_inner<M>(
		&self,
		req_id: Uuid,
		subject: String,
		req_body: M::Request,
		req_debug: Option<chirp::RequestDebug>,
		req_dont_log_body: bool,
	) -> Result<RpcResponse<M::Response>, ClientError>
	where
		M: Endpoint,
	{
		// Serialize request body
		let mut req_body_buf = Vec::with_capacity(prost::Message::encoded_len(&req_body));
		prost::Message::encode(&req_body, &mut req_body_buf)
			.map_err(ClientError::EncodeRequestBody)?;

		if req_dont_log_body {
			tracing::info!(
				?subject,
				?req_debug,
				body_bytes = ?req_body_buf.len(),
				"rpc req"
			);
		} else {
			tracing::info!(
				?subject,
				?req_body,
				?req_debug,
				body_bytes = ?req_body_buf.len(),
				"rpc req"
			);
		}

		// Send the request
		let res_body_buf = self
			.rpc_inner_bytes(
				req_id,
				subject.clone(),
				req_body_buf,
				req_debug,
				req_dont_log_body,
				M::TIMEOUT,
			)
			.await?;

		// Decode the response
		let res_body = <M::Response as prost::Message>::decode(res_body_buf.as_slice())
			.map_err(ClientError::DecodeResponseBody)?;
		if req_dont_log_body {
			tracing::info!(?subject, "rpc res");
		} else {
			tracing::info!(?subject, ?res_body, "rpc res");
		}

		Ok(RpcResponse { body: res_body })
	}

	#[tracing::instrument(skip(req_body_buf))]
	async fn rpc_inner_bytes(
		&self,
		req_id: Uuid,
		subject: String,
		req_body_buf: Vec<u8>,
		req_debug: Option<chirp::RequestDebug>,
		req_dont_log_body: bool,
		timeout: Duration,
	) -> Result<Vec<u8>, ClientError> {
		// Serialize request
		let req = chirp::Request {
			req_id: Some(req_id.into()),
			ray_id: Some(self.ray_id.into()),
			ts: rivet_util::timestamp::now(),
			trace: (*self.trace).clone(),
			body: req_body_buf,
			debug: req_debug,
			dont_log_body: req_dont_log_body,
		};
		let mut req_buf = Vec::with_capacity(prost::Message::encoded_len(&req));
		prost::Message::encode(&req, &mut req_buf).map_err(ClientError::EncodeRequest)?;

		// Publish message and wait for response
		let mut service_unavailable_backoff = Backoff::default();
		let response = 'req: loop {
			// Handle service unavailable
			let res = self
				.rpc_inner_call(subject.clone(), req_buf.clone(), timeout)
				.await;
			match res {
				Ok(x) => break 'req x,
				Err(ClientError::NatsResponseStatus(status))
					if status == nats::StatusCode::NO_RESPONDERS =>
				{
					tracing::warn!(tick_index = %service_unavailable_backoff.tick_index(), "service unavailable");
					if service_unavailable_backoff.tick().await {
						return Err(ClientError::NatsResponseStatus(status));
					} else {
						continue 'req;
					}
				}
				Err(ClientError::RpcAckTimedOut) => {
					tracing::warn!(tick_index = %service_unavailable_backoff.tick_index(), "rpc ack timed out");
					if service_unavailable_backoff.tick().await {
						return Err(ClientError::RpcAckTimedOut);
					} else {
						continue 'req;
					}
				}
				Err(err) => {
					return Err(err);
				}
			}
		};

		// Parse and handle response
		let res = <chirp::Response as prost::Message>::decode(&response.payload[..])
			.map_err(ClientError::DecodeResponse)?;
		match res.kind {
			Some(chirp::response::Kind::Ok(ok)) => Ok(ok.body),
			Some(chirp::response::Kind::Err(err)) => {
				tracing::warn!(?subject, ?err, "rpc err");

				match err.kind {
					Some(chirp::response::err::Kind::Internal(err)) => {
						Err(ClientError::GlobalError(GlobalError::Internal {
							ty: err.ty,
							message: err.message,
							debug: err.debug,
							code: chirp::ErrorCode::from_i32(err.code)
								.unwrap_or(chirp::ErrorCode::Internal),
							retry_immediately: false,
						}))
					}
					Some(chirp::response::err::Kind::BadRequest(err)) => {
						Err(ClientError::GlobalError(GlobalError::BadRequest {
							code: err.code,
							context: err.context,
							metadata: err.metadata,
						}))
					}
					None => {
						tracing::error!(kind = ?err.kind, "unexpected err kind");

						Err(ClientError::MalformedResponse)
					}
				}
			}
			Some(_) | None => {
				tracing::error!(kind = ?res.kind, "unexpected res kind");
				Err(ClientError::MalformedResponse)
			}
		}
	}

	/// Interfaces with NATS to send the RPC request.
	///
	/// May be called multiple times from `rpc_inner` if retrying an RPC request
	/// in a backoff loop.
	#[tracing::instrument(skip(req_buf))]
	async fn rpc_inner_call(
		&self,
		subject: String,
		req_buf: Vec<u8>,
		timeout: Duration,
	) -> Result<nats::Message, ClientError> {
		// We don't use `self.nats.request(...)` intentionally. Unsure why.

		// Build subscription with reply inbox
		//
		// Will be flushed after `publish_with_reply_and_headers`
		let reply = self.nats.new_inbox();
		let mut sub = self
			.nats
			.subscribe(reply.clone())
			.await
			.map_err(|x| ClientError::CreateSubscription(x.into()))?;

		// Manually publish message
		self.nats
			.publish_with_reply_and_headers(
				subject.clone(),
				reply.clone(),
				Default::default(),
				req_buf.into(),
			)
			.await
			.map_err(ClientError::PublishRequest)?;
		self.nats
			.flush()
			.await
			.map_err(|x| ClientError::FlushNats(x.into()))?;

		// Wait for the ack. If this times out, we'll retry with a
		// backoff.
		let ack_timeout = Duration::from_secs(15);
		match tokio::time::timeout(ack_timeout, sub.next()).await {
			Ok(Some(msg)) => {
				if let Some(status) = msg.status {
					// If this is a 503, we'll retry the request with a
					// backoff.
					return Err(ClientError::NatsResponseStatus(status));
				}

				let res = <chirp::Response as prost::Message>::decode(&*msg.payload)
					.map_err(ClientError::DecodeResponse)?;
				match res.kind {
					Some(chirp::response::Kind::Ack(_)) => {
						tracing::trace!("received ack");
					}
					Some(_) => {
						tracing::trace!("received res before ack");
						return Ok(msg);
					}
					None => {
						return Err(ClientError::MalformedResponse);
					}
				}
			}
			Ok(None) => {
				tracing::warn!("rpc ack unsubscribed");
				return Err(ClientError::RpcSubscriptionUnsubscribed);
			}
			Err(_) => {
				tracing::warn!("req ack timed out");
				return Err(ClientError::RpcAckTimedOut);
			}
		}

		// Wait for the response message.
		match tokio::time::timeout(timeout, sub.next()).await {
			Ok(Some(msg)) => {
				return Ok(msg);
			}
			Ok(None) => {
				tracing::warn!("rpc unsubscribed");
				return Err(ClientError::RpcSubscriptionUnsubscribed);
			}
			Err(_) => {
				tracing::warn!("req timed out");
				return Err(ClientError::RpcTimedOut);
			}
		}
	}

	/// Publishes a message to NATS and to a durable event stream if a topic is
	/// set.
	///
	/// Use `subscribe` to consume these messages ephemerally, `tail` to read
	/// the most recently sent message, and worker consumers to consume durable
	/// streams.
	///
	/// This spawns a background task that calls `message_wait` internally and does not wait for the message to
	/// finish publishing. This is done since there are very few cases where a
	/// service should need to wait or fail if a message does not publish
	/// successfully.
	#[tracing::instrument(err, skip_all)]
	pub async fn message<M>(
		&self,
		parameters: Vec<String>,
		message_body: M,
		opts: MessageOptions,
	) -> Result<(), ClientError>
	where
		M: Message,
	{
		let client = self.clone();
		let spawn_res = tokio::task::Builder::new()
			.name("chirp_client::message_async")
			.spawn(
				async move {
					match client
						.message_wait::<M>(parameters, message_body, opts)
						.await
					{
						Ok(_) => {}
						Err(err) => {
							tracing::error!(?err, "failed to publish message");
						}
					}
				}
				.instrument(tracing::info_span!("async_message")),
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
		parameters: Vec<String>,
		message_body: M,
		opts: MessageOptions,
	) -> Result<(), ClientError>
	where
		M: Message,
	{
		if parameters.len() != M::PARAMETERS.len() {
			return Err(ClientError::MismatchedMessageParameterCount);
		}

		let nats_subject = message::serialize_message_nats_subject::<M, _>(&parameters);
		let duration_since_epoch = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or_else(|err| unreachable!("time is broken: {}", err));
		let ts = duration_since_epoch.as_millis() as i64;

		// Encode the body
		let mut body_buf = Vec::with_capacity(message_body.encoded_len());
		message_body
			.encode(&mut body_buf)
			.map_err(ClientError::EncodeMessage)?;
		let body_buf_len = body_buf.len();

		// Encode message
		let req_id = Uuid::new_v4();
		let message = chirp::Message {
			req_id: Some(req_id.into()),
			ray_id: Some(self.ray_id.into()),
			parameters: parameters.clone(),
			ts,
			trace: (*self.trace).clone(),
			body: body_buf,
		};
		let mut message_buf = Vec::with_capacity(prost::Message::encoded_len(&message));
		prost::Message::encode(&message, &mut message_buf).map_err(ClientError::EncodeMessage)?;

		if opts.dont_log_body {
			tracing::info!(
				?nats_subject,
				body_bytes = ?body_buf_len,
				message_bytes = ?message_buf.len(),
				"publish message"
			);
		} else {
			tracing::info!(
				?nats_subject,
				?message_body,
				body_bytes = ?body_buf_len,
				message_bytes = ?message_buf.len(),
				"publish message"
			);
		}
		self.perf().mark_rpc(
			format!("msg.tx.{}", nats_subject.replace(".", "\\.")),
			self.ray_id,
			req_id,
		);

		// Write to Redis and NATS.
		//
		// It's important to write to the stream as fast as possible in order to
		// ensure messages are handled quickly.
		self.message_write_redis::<M>(&parameters, message_buf.as_slice(), req_id, ts)
			.await;
		self.message_publish_nats::<M>(&nats_subject, message_buf.as_slice())
			.await;

		Ok(())
	}

	/// Writes a message to a Redis durable stream and tails.
	#[tracing::instrument(level = "debug", skip_all)]
	async fn message_write_redis<M>(
		&self,
		parameters: &[String],
		message_buf: &[u8],
		req_id: Uuid,
		ts: i64,
	) where
		M: Message,
	{
		if M::TOPIC.is_none() {
			return;
		}

		// Generate permuted wildcard tail keys
		let parameters_str = parameters
			.iter()
			.map(|x| escape_parameter_wildcards(x))
			.collect::<Vec<String>>();
		let permuted_wildcard_parameters =
			generate_permuted_wildcard_parameters::<M>(&parameters_str);

		// Write message to Redis stream
		let span = self.perf().start(M::PERF_LABEL_WRITE_STREAM).await;
		let mut backoff = rivet_util::Backoff::default_infinite();
		loop {
			// Ignore for infinite backoff
			backoff.tick().await;

			let mut conn = self.redis_chirp.clone();

			let mut pipe = redis::pipe();
			pipe.atomic();

			// Write to stream
			let topic_key = redis_keys::message_topic(M::NAME);
			pipe.xadd_maxlen(
				&topic_key,
				redis::streams::StreamMaxlen::Approx(8192),
				"*",
				&[("m", &message_buf)],
			)
			.ignore();

			// Write tails for all permuted parameters
			for wildcard_parameters in &permuted_wildcard_parameters {
				// Write tail
				if let Some(ttl) = M::TAIL_TTL {
					// Write single tail message

					let tail_key = redis_keys::message_tail::<M, _>(wildcard_parameters);

					// Save message
					pipe.hset(
						&tail_key,
						redis_keys::message_tail::REQUEST_ID,
						req_id.to_string(),
					)
					.ignore();
					pipe.hset(&tail_key, redis_keys::message_tail::TS, ts)
						.ignore();
					pipe.hset(&tail_key, redis_keys::message_tail::BODY, message_buf)
						.ignore();

					// Automatically expire
					pipe.expire(&tail_key, ttl as usize).ignore();

					// Write history
					if M::HISTORY {
						let history_key = redis_keys::message_history::<M, _>(wildcard_parameters);

						// Remove old entries 10% of the time.
						//
						// This is a slow operation, so we perform this
						// sparingly.
						if rand::thread_rng().gen_bool(0.1) {
							pipe.cmd("ZREMRANGEBYSCORE")
								.arg(&history_key)
								.arg("-inf")
								.arg(rivet_util::timestamp::now() - ttl * 1000)
								.ignore();
						}

						// Save message
						pipe.zadd(&history_key, message_buf, ts).ignore();

						// Automatically expire
						pipe.expire(&history_key, ttl as usize).ignore();
					} else {
					}
				}
			}

			// Write to Redis
			match pipe.query_async::<_, ()>(&mut conn).await {
				Ok(_) => {
					tracing::debug!("write to redis stream succeeded");
					break;
				}
				Err(err) => {
					tracing::error!(?err, "failed to write to redis");
				}
			}
		}
		span.end();
	}

	/// Publishes the message to NATS.
	#[tracing::instrument(level = "debug", skip_all)]
	async fn message_publish_nats<M>(&self, nats_subject: &str, message_buf: &[u8])
	where
		M: Message,
	{
		// Publish message to NATS. Do this after a successful write to
		// Redis in order to verify that tailing messages doesn't end up in a
		// race condition that misses a message from the database.
		//
		// Infinite backoff since we want to wait until the service reboots.
		let span = self.perf().start(M::PERF_LABEL_PUBLISH).await;
		let mut backoff = rivet_util::Backoff::default_infinite();
		loop {
			// Ignore for infinite backoff
			backoff.tick().await;

			let nats_subject = nats_subject.to_owned();
			let message_buf = message_buf.to_vec();

			if let Err(err) = self
				.nats
				.publish(nats_subject.clone(), message_buf.into())
				.await
			{
				tracing::warn!(?err, "publish message failed, trying again");
				continue;
			}

			// TODO: Most messages don't need to be flushed immediately. We
			// should add an option to enable high performance message
			// publishing to enable flushing immediately after publishing.
			// if let Err(err) = self.nats.flush().await {
			// 	tracing::error!(?err, "flush message failed, the message probably sent");
			// 	break;
			// }

			tracing::debug!("publish nats message succeeded");
			break;
		}
		span.end();
	}

	/// Sends a Chirp message and awaits for a Chirp response.
	#[tracing::instrument(
		err,
		skip_all,
		fields(req_message = M1::NAME, res_message = M2::NAME)
	)]
	pub async fn message_with_subscribe<M1, M2>(
		&self,
		parameters1: Vec<String>,
		message_body: M1,
		parameters2: Option<Vec<String>>,
		filter_trace: bool,
	) -> Result<ReceivedMessage<M2>, ClientError>
	where
		M1: Message,
		M2: Message,
	{
		// We need to flush on a subscription because we need to guarantee that
		// NATS receives the subscription immediately in order to ensure it's
		// synchronized with other services.
		//
		// e.g. this would lead to a lost response:
		// 1. api-identity: subscribe to chirp.msg.msg-user-create-complete
		// 2. api-identity: XADD message to redis
		// 3. user-create: XREAD message from redis
		// 4. user-create: publish user-create-complete to NATS
		//                 ^ this message will get lost ^
		// 5. api-identity: flush the subscription
		// 6. api-identity: hang indefinitely send no message was received
		let mut sub = self
			.subscribe::<M2>(parameters2.unwrap_or_else(|| parameters1.clone()))
			.await?;

		self.message::<M1>(parameters1, message_body, Default::default())
			.await?;

		sub.next_with_trace(filter_trace).await
	}

	/// Sends a Chirp message and awaits for a Chirp from two different
	/// messages and returns a result.
	#[tracing::instrument(
		err,
		skip_all,
		fields(
			req_message = M::NAME,
			res_ok_message = MOk::NAME,
			res_err_message = MErr::NAME
		),
	)]
	pub async fn message_with_result<M, MOk, MErr>(
		&self,
		parameters: Vec<String>,
		message_body: M,
		parameters_ok: Option<Vec<String>>,
		parameters_err: Option<Vec<String>>,
		filter_trace: bool,
	) -> Result<Result<ReceivedMessage<MOk>, ReceivedMessage<MErr>>, ClientError>
	where
		M: Message,
		MOk: Message,
		MErr: Message,
	{
		// See why this is flushed in `Client::message_with_subscribe`
		let (mut sub_ok, mut sub_err) = tokio::try_join!(
			self.subscribe_opt::<MOk>(SubscribeOpts {
				parameters: parameters_ok.unwrap_or_else(|| parameters.clone()),
				// Will be flushed in batch
				flush_nats: false,
			}),
			self.subscribe_opt::<MErr>(SubscribeOpts {
				parameters: parameters_err.unwrap_or_else(|| parameters.clone()),
				// Will be flushed in batch
				flush_nats: false,
			}),
		)?;

		self.nats
			.flush()
			.await
			.map_err(|x| ClientError::FlushNats(x.into()))?;

		self.message::<M>(parameters, message_body, Default::default())
			.await?;

		tokio::select! {
			msg = sub_ok.next_with_trace(filter_trace) => {
				msg.map(Ok)
			}
			msg = sub_err.next_with_trace(filter_trace) => {
				msg.map(Err)
			}
		}
	}

	/// Listens for Chirp messages globally on NATS.
	#[tracing::instrument(level = "debug", err, skip_all)]
	pub async fn subscribe<M>(
		&self,
		parameters: Vec<String>,
	) -> Result<SubscriptionHandle<M>, ClientError>
	where
		M: Message,
	{
		self.subscribe_opt::<M>(SubscribeOpts {
			parameters,
			flush_nats: true,
		})
		.await
	}

	/// Listens for Chirp messages globally on NATS.
	#[tracing::instrument(err, skip_all, fields(message = M::NAME))]
	pub async fn subscribe_opt<M>(
		&self,
		opts: SubscribeOpts,
	) -> Result<SubscriptionHandle<M>, ClientError>
	where
		M: Message,
	{
		let lifetime_perf = self.perf().start(M::PERF_LABEL_SUBSCRIBE).await;

		let nats_subject = message::serialize_message_nats_subject::<M, _>(&opts.parameters);

		// Create subscription and flush immediately.
		tracing::info!(%nats_subject, parameters = ?opts.parameters, "creating subscription");
		let subscription = self
			.nats
			.subscribe(nats_subject.clone())
			.await
			.map_err(|x| ClientError::CreateSubscription(x.into()))?;
		if opts.flush_nats {
			self.nats
				.flush()
				.await
				.map_err(|x| ClientError::FlushNats(x.into()))?;
		}

		// Return handle
		let subscription = SubscriptionHandle::new(
			nats_subject,
			subscription,
			self.perf().clone(),
			lifetime_perf,
			self.parent_req_id(),
		);
		Ok(subscription)
	}

	/// Reads the tail message of a stream without waiting for a message.
	#[tracing::instrument(err, skip_all, fields(message = M::NAME))]
	pub async fn tail_read<M>(
		&self,
		parameters: Vec<String>,
	) -> Result<Option<ReceivedMessage<M>>, ClientError>
	where
		M: Message,
	{
		if M::TAIL_TTL.is_none() {
			return Err(ClientError::CannotTailMessage { name: M::NAME });
		}

		let lifetime_perf = self.perf().start(M::PERF_LABEL_TAIL_READ).await;

		let mut conn = self.redis_chirp.clone();

		// Fetch message
		let tail_key = redis_keys::message_tail::<M, _>(&parameters);
		let msg_buf = conn
			.hget::<_, _, Option<Vec<u8>>>(&tail_key, redis_keys::message_tail::BODY)
			.await?;

		// Decode message
		let msg = if let Some(msg_buf) = msg_buf {
			let msg = ReceivedMessage::<M>::decode(msg_buf.as_slice())?;
			tracing::info!(?msg, "immediate read tail message");
			Some(msg)
		} else {
			tracing::info!("no tail message to read");
			None
		};

		lifetime_perf.end();

		Ok(msg)
	}

	/// Used by API services to tail an event (by start time) after a given timestamp.
	///
	/// Because this waits indefinitely until next event, it is recommended to use this inside
	/// of a `rivet_util::macros::select_with_timeout!` block:
	/// ```rust
	/// use rivet_util as util;
	///
	/// let event_sub = tail_anchor!([ctx, anchor] message_test());
	///
	/// // Consumes anchor or times out after 1 minute
	/// util::macros::select_with_timeout!(
	/// 	event = event_sub => {
	/// 		let _event = event?;
	/// 	}
	/// );
	/// ```
	#[tracing::instrument(err, skip_all, fields(message = M::NAME))]
	pub async fn tail_anchor<M>(
		&self,
		parameters: Vec<String>,
		anchor: &TailAnchor,
	) -> Result<TailAnchorResponse<M>, ClientError>
	where
		M: Message,
	{
		let Some(ttl) = M::TAIL_TTL else {
			return Err(ClientError::CannotTailMessage { name: M::NAME });
		};

		let lifetime_perf = self.perf().start(M::PERF_LABEL_TAIL_ANCHOR).await;

		// Validate anchor is valid
		if !anchor.is_valid(ttl) {
			return Ok(TailAnchorResponse::AnchorExpired);
		}

		// Create subscription. Do this before reading from the log in order to
		// ensure consistency.
		//
		// Leave flush enabled in order to ensure that subscription is
		// registered with NATS before continuing.
		let mut sub = self.subscribe(parameters.clone()).await?;

		// Read the tail log
		let tail_read = self.tail_read(parameters.clone()).await?;

		// Check if valid or wait for subscription
		let (msg, source) = match tail_read {
			Some(msg) if msg.ts > anchor.start_time => (msg, "tail_read"),
			_ => {
				// Wait for next message if tail not present
				let msg = sub.next().await?;
				(msg, "subscription")
			}
		};

		tracing::info!(?msg, %source, ?anchor, "read tail message");

		lifetime_perf.end();

		Ok(TailAnchorResponse::Message(msg))
	}

	/// Fetches all recently dispatched messages after the given anchor anchor.
	///
	/// Used in situations where we need all messages published between the
	/// anchor and now. This is different than just calling `tail_anchor`, since
	/// `tail_anchor` only returns the most recent message.
	#[tracing::instrument(skip_all, fields(message = M::NAME))]
	pub async fn tail_all<M>(
		&self,
		parameters: Vec<Vec<String>>,
		anchor: &TailAnchor,
		config: TailAllConfig,
	) -> Result<TailAllResponse<M>, ClientError>
	where
		M: Message,
	{
		let Some(ttl) = M::TAIL_TTL else {
			return Err(ClientError::CannotTailMessage { name: M::NAME });
		};

		let lifetime_perf = self.perf().start(M::PERF_LABEL_TAIL_ALL).await;

		// Check if the anchor is within the message's TTL
		let (start_time, anchor_status) = if anchor.is_valid(ttl) {
			// Anchor is valid
			(anchor.start_time, TailAllAnchorStatus::Valid)
		} else {
			// Anchor is expired
			match config.anchor_expired_behavior {
				TailAllAnchorExpiredBehavior::ReturnImmediately => {
					return Ok(TailAllResponse {
						messages: Vec::new(),
						anchor_status: TailAllAnchorStatus::Expired,
						subs: Vec::new(),
					});
				}
				TailAllAnchorExpiredBehavior::UseCurrentAnchor => (
					rivet_util::timestamp::now(),
					TailAllAnchorStatus::ExpiredFallbackToCurrent,
				),
			}
		};

		// Create subscriptions for new updates if we will need to collect messages
		// during the grace period or need to return the subscriptions. Do this
		// before reading from the log in order to ensure consistency.
		let subs = if config.collect_grace.is_some()
			|| config.empty_grace.is_some()
			|| matches!(
				config.post_logs_behavior,
				TailAllPostLogsBehavior::ReturnAlways
			) {
			let subs = futures_util::future::try_join_all(parameters.iter().map(|params| {
				self.subscribe_opt::<M>(SubscribeOpts {
					parameters: params.clone(),
					// Will be flushed in batch
					flush_nats: false,
				})
			}))
			.await?;
			self.nats
				.flush()
				.await
				.map_err(|x| ClientError::FlushNats(x.into()))?;

			subs
		} else {
			Vec::new()
		};

		// Read the recent messages from all parameters
		let mut conn = self.redis_chirp.clone();
		let mut messages = Vec::new();
		for params in &parameters {
			// TODO: Do this in a batch
			// Fetch messages from Redis.
			//
			// If there is no history, read from the tail.
			//
			// If we are only reading one message, then read from the tail since
			// it's faster.
			if M::HISTORY && config.message_limit != 1 {
				// Fetch message
				let history_key = redis_keys::message_history::<M, _>(&params);
				let msg_bufs = conn
					.zrangebyscore_limit::<_, _, _, Vec<Vec<u8>>>(
						&history_key,
						format!("({start_time}"),
						"+inf",
						0,
						config.message_limit as isize,
					)
					.await?;

				// Decode message
				for msg_buf in msg_bufs {
					let msg = ReceivedMessage::<M>::decode(msg_buf.as_slice())?;
					messages.push(msg);
				}
			} else {
				// Fetch message
				let tail_key = redis_keys::message_tail::<M, _>(&params);
				let msg_buf = conn
					.hget::<_, _, Option<Vec<u8>>>(&tail_key, redis_keys::message_tail::BODY)
					.await?;

				// Decode message
				if let Some(msg_buf) = msg_buf {
					let msg = ReceivedMessage::<M>::decode(msg_buf.as_slice())?;

					if msg.ts > anchor.start_time {
						messages.push(msg);
					}
				}
			}
		}

		match config.post_logs_behavior {
			TailAllPostLogsBehavior::ReturnAlways | TailAllPostLogsBehavior::ReturnIfMessages => {
				if matches!(
					config.post_logs_behavior,
					TailAllPostLogsBehavior::ReturnAlways
				) || !messages.is_empty()
				{
					// Sort messages by timestamp
					messages.sort_by_key(|x| x.ts);

					return Ok(TailAllResponse {
						messages,
						anchor_status,
						subs,
					});
				}
			}
			TailAllPostLogsBehavior::None => {}
		};

		/// Inserts a message in to the message list if it does not already
		/// exist.
		fn insert_message<M>(
			messages: &mut Vec<ReceivedMessage<M>>,
			msg_req_ids: &mut HashSet<Uuid>,
			msg: ReceivedMessage<M>,
		) where
			M: Message,
		{
			// Validate this is not a duplicate message
			if !msg_req_ids.contains(&msg.req_id) {
				msg_req_ids.insert(msg.req_id);
				messages.push(msg);
			}
		}

		// Collect more messages from the subscriptions
		if !subs.is_empty() {
			// Aggregate all known request IDs in order to deduplicate messages
			// received in the subscription
			let mut msg_req_ids = messages.iter().map(|x| x.req_id).collect::<HashSet<Uuid>>();

			// Merge all subscription streams
			let streams = subs.into_iter().map(|x| x.into_stream().boxed());
			let mut all_subs = futures_util::stream::select_all(streams);

			let collect_more = if messages.is_empty() {
				// Wait for empty period
				if let Some(empty_grace) = config.empty_grace {
					// Wait for a message
					let collect_finish = tokio::time::Instant::now() + empty_grace;
					tokio::select! {
						_ = tokio::time::sleep_until(collect_finish) => {
							// No messages were received in empty period
							false
						}
						msg = all_subs.try_next() => {
							let msg = msg?;
							if let Some(msg) = msg {
								insert_message(&mut messages, &mut msg_req_ids, msg);
								true
							} else {
								tracing::warn!("subscription stream ended prematurely");
								false
							}
						}
					}
				} else {
					// Trigger the collection grace period immediately even
					// though there are no messages. This is because the
					// collection grace period also acts a sort of rate
					// limit when this function is polled, so we should wait
					// with a grace no matter what.
					true
				}
			} else {
				// Trigger the collection grace period immediately
				true
			};

			// Collect messages in grace period
			if collect_more {
				if let Some(collect_grace) = config.collect_grace {
					let collect_finish = tokio::time::Instant::now() + collect_grace;
					loop {
						// Check if we're over the message limit
						if messages.len() >= config.message_limit {
							break;
						}

						// Wait for a message or expiration
						tokio::select! {
							_ = tokio::time::sleep_until(collect_finish) => {
								break;
							}
							msg = all_subs.try_next() => {
								let msg = msg?;
								if let Some(msg) = msg {
									insert_message(&mut messages, &mut msg_req_ids, msg);
								} else {
									tracing::warn!("subscription stream ended prematurely");
									break;
								}
							}
						}
					}
				}
			}
		}

		// Sort messages by timestamp
		messages.sort_by_key(|x| x.ts);

		tracing::info!(?messages, ?anchor, ?start_time, "read recent tail messages");

		lifetime_perf.end();

		Ok(TailAllResponse {
			messages,
			anchor_status,
			subs: Vec::new(),
		})
	}

	/// A helper function that makes it possible to pass a client directly to the `op!` macro.
	/// There is no reason to ever use this function directly.
	#[doc(hidden)]
	pub fn chirp(&self) -> &Client {
		self
	}
}

#[derive(Debug)]
pub struct SubscribeOpts {
	pub parameters: Vec<String>,
	pub flush_nats: bool,
}

/// Used to receive messages from other Chirp clients.
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
	perf: chirp_perf::PerfCtx,
	parent_req_id: Uuid,
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
	fn new(
		subject: String,
		subscription: nats::Subscriber,
		perf: chirp_perf::PerfCtx,
		lifetime_perf: chirp_perf::Span,
		parent_req_id: Uuid,
	) -> Self {
		let token = CancellationToken::new();

		{
			let token = token.clone();
			let spawn_res = tokio::task::Builder::new()
				.name("chirp_client::message_wait_drop")
				.spawn(
					async move {
						token.cancelled().await;

						tracing::info!("closing subscription");

						// End lifetime
						lifetime_perf.end();

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
			perf,
			parent_req_id,
		}
	}

	/// Waits for the next message in the subscription.
	///
	/// This future can be safely dropped.
	#[tracing::instrument]
	pub async fn next(&mut self) -> Result<ReceivedMessage<M>, ClientError> {
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
	) -> Result<ReceivedMessage<M>, ClientError> {
		self.next_inner(filter_trace).await
	}

	/// This future can be safely dropped.
	#[tracing::instrument(level = "trace")]
	async fn next_inner(&mut self, filter_trace: bool) -> Result<ReceivedMessage<M>, ClientError> {
		tracing::info!("waiting for message");

		loop {
			// Poll the subscription.
			//
			// Use blocking threads instead of `try_next`, since I'm not sure
			// try_next works as intended.
			//
			// We use `next_timeout` so we don't block indefinitely on a
			// subscription that never delivers a message.
			let nats_message = match self.subscription.next().await {
				Some(x) => x,
				None => {
					tracing::debug!("unsubscribed");
					return Err(ClientError::SubscriptionUnsubscribed);
				}
			};

			if filter_trace {
				let (_message_wrapper, trace) =
					ReceivedMessage::<M>::decode_inner(&nats_message.payload[..])?;

				// Check if the message trace stack originates from this client
				//
				// We intentionally use the request ID instead of just checking the ray ID because
				// there may be multiple calls to `message_with_subscribe` within the same ray.
				// Explicitly checking the parent request ensures the response is unique to this
				// message.
				if trace
					.iter()
					.rev()
					.any(|trace_entry| trace_entry.req_id() == self.parent_req_id)
				{
					let message = ReceivedMessage::<M>::decode(&nats_message.payload[..])?;
					tracing::info!(?message, "received message");

					self.perf.mark_rpc(
						format!("msg.rx.{}", self.subject.replace(".", "\\.")),
						message.ray_id(),
						message.req_id(),
					);

					return Ok(message);
				}
			} else {
				let message = ReceivedMessage::<M>::decode(&nats_message.payload[..])?;
				tracing::info!(?message, "received message");

				self.perf.mark_rpc(
					format!("msg.rx.{}", self.subject.replace(".", "\\.")),
					message.ray_id(),
					message.req_id(),
				);

				return Ok(message);
			}

			// Message not from parent, continue with loop
		}
	}

	/// Converts the subscription in to a stream.
	pub fn into_stream(
		self,
	) -> impl futures_util::Stream<Item = Result<ReceivedMessage<M>, ClientError>> {
		futures_util::stream::try_unfold(self, |mut sub| async move {
			let msg = sub.next().await?;
			Ok(Some((msg, sub)))
		})
	}
}

/// Defines the anchor for `Client::tail_anchor`.
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

/// Generates a list of all possible wildcard combinations for parameters.
///
/// Generate all possible wildcard possibilities of the parameters. For
/// example, publishing `user-follow.123.456` will write the following
/// keys:
/// * `user-follow.123.456`
/// * `user-follow.*.456`
/// * `user-follow.123.*`
/// * `user-follow.*.*`
//
/// Note that you should be careful how many keys you enable permutations
/// for, since the amount of keys generated by this quickly gets out of
/// hand. e.g. for the given inputs, the number of keys produced are:
/// 2 => 2
/// 3 => 6
/// 4 => 12
/// 5 => 20
///
/// You can explicitly disable wildcards for specific parameters in service's
/// topic config.
fn generate_permuted_wildcard_parameters<M>(src: &[String]) -> HashSet<Vec<&str>>
where
	M: Message,
{
	fn permute_wildcards<'a, M>(
		src: &'a [String],
		out: &mut HashSet<Vec<&'a str>>,
		arr: &mut Vec<bool>,
		n: usize,
		i: usize,
	) where
		M: Message,
	{
		if i == n {
			// Substitute wildcards for parameters that call for it in this
			// permutation & have it enabled.
			let parameters = arr
				.iter()
				.enumerate()
				.map(|(i, wildcard)| {
					// Add a wildcard if this permutation calls for it and
					// wildcards are enabled for this parameter
					if *wildcard && M::PARAMETERS[i].wildcard {
						"*"
					} else {
						src[i].as_str()
					}
				})
				.collect::<Vec<&str>>();
			out.insert(parameters);
			return;
		}

		arr[i] = false;
		permute_wildcards::<M>(src, out, arr, n, i + 1);

		arr[i] = true;
		permute_wildcards::<M>(src, out, arr, n, i + 1);
	}

	// List of subjects. We use a HashSet since some parameters may already
	// be wildcards.
	let mut permuted_keys = HashSet::new();
	// State for the permutation
	let mut arr = vec![false; src.len()];
	let arr_len = arr.len();
	permute_wildcards::<M>(src, &mut permuted_keys, &mut arr, arr_len, 0);

	permuted_keys
}

/// Escapes all wildcards in a parameter. For example:
/// `abc*d\n` => `abc\*d\\n`
fn escape_parameter_wildcards(x: &str) -> String {
	x.replace("\\", "\\\\").replace("*", "\\*")
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

/// Configuration for `tail_all`.
pub struct TailAllConfig {
	/// What to do when the anchor is older than the message's TTL.
	pub anchor_expired_behavior: TailAllAnchorExpiredBehavior,

	/// The maximum number of messages to collect at once.
	pub message_limit: usize,

	/// How long to collect messages for after fetching the log.
	///
	/// This is done by creating a subscription to messages before fetching the
	/// recent messages then polling those subscriptions for this duration.
	///
	/// This is useful for capturing bursts of messages and/or limiting the rate
	/// at which messages can be called.
	///
	/// For example, if polling `tail_all` on a rapid stream of messages,
	/// if there is no `collect_grace`, this will poll as fast the messages get
	/// published, which is likely wasting a lot of resources.
	pub collect_grace: Option<Duration>,

	/// If no messages were fetched for the database, we'll wait for another
	/// message for this duration before triggering the grace period and
	/// returning.
	pub empty_grace: Option<Duration>,

	/// Immediately returns messages and created subs after fetching logs. No
	/// subscriptions are polled.
	post_logs_behavior: TailAllPostLogsBehavior,
}

impl TailAllConfig {
	/// Used when only needs to read the tail messages and not create
	/// subscriptions.
	///
	/// Be careful with this: using `collect_grace` will act as a rate limit and
	/// prevent polling endpoints from calling this in a tight loop.
	pub fn read_no_grace() -> TailAllConfig {
		TailAllConfig {
			anchor_expired_behavior: TailAllAnchorExpiredBehavior::ReturnImmediately,
			message_limit: 256,
			collect_grace: None,
			empty_grace: None,
			post_logs_behavior: TailAllPostLogsBehavior::None,
		}
	}

	/// Simply reads the history with a grace period.
	pub fn read() -> TailAllConfig {
		TailAllConfig {
			anchor_expired_behavior: TailAllAnchorExpiredBehavior::ReturnImmediately,
			message_limit: 256,
			collect_grace: Some(Duration::from_millis(100)),
			empty_grace: None,
			post_logs_behavior: TailAllPostLogsBehavior::None,
		}
	}

	/// Used for wait requests that need a collect grace & empty grace period.
	pub fn wait() -> TailAllConfig {
		TailAllConfig {
			anchor_expired_behavior: TailAllAnchorExpiredBehavior::UseCurrentAnchor,
			message_limit: 256,
			collect_grace: Some(Duration::from_millis(100)),
			empty_grace: Some(Duration::from_secs(60)),
			post_logs_behavior: TailAllPostLogsBehavior::None,
		}
	}

	/// Use for wait requests that need a collect grace & empty grace period,
	/// but define custom behavior if the anchor is invalid.
	pub fn wait_return_immediately() -> TailAllConfig {
		TailAllConfig {
			anchor_expired_behavior: TailAllAnchorExpiredBehavior::ReturnImmediately,
			message_limit: 256,
			collect_grace: Some(Duration::from_millis(100)),
			empty_grace: Some(Duration::from_secs(60)),
			post_logs_behavior: TailAllPostLogsBehavior::None,
		}
	}

	/// Returns messages from log immediately but collects messages from subs
	// with a grace period. This ensures we always have the latest message.
	pub fn return_logs_immediately() -> TailAllConfig {
		TailAllConfig {
			anchor_expired_behavior: TailAllAnchorExpiredBehavior::UseCurrentAnchor,
			message_limit: 256,
			collect_grace: Some(Duration::from_millis(100)),
			empty_grace: Some(Duration::from_secs(60)),
			post_logs_behavior: TailAllPostLogsBehavior::ReturnIfMessages,
		}
	}

	/// Returns messages from log as well as all subs instead of polling them.
	pub fn return_after_logs() -> TailAllConfig {
		TailAllConfig {
			anchor_expired_behavior: TailAllAnchorExpiredBehavior::UseCurrentAnchor,
			message_limit: 256,
			collect_grace: None,
			empty_grace: None,
			post_logs_behavior: TailAllPostLogsBehavior::ReturnAlways,
		}
	}

	/// Same as `return_after_logs` but returns immediately when given an expired anchor.
	pub fn return_after_logs_immediately() -> TailAllConfig {
		TailAllConfig {
			anchor_expired_behavior: TailAllAnchorExpiredBehavior::ReturnImmediately,
			message_limit: 256,
			collect_grace: None,
			empty_grace: None,
			post_logs_behavior: TailAllPostLogsBehavior::ReturnAlways,
		}
	}
}

#[derive(Default, Debug, Clone)]
pub struct MessageOptions {
	pub dont_log_body: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TailAllAnchorExpiredBehavior {
	/// Don't do anything and return an empty list of messages.
	ReturnImmediately,

	/// Use the current timestamp as the anchor. This should be paired  with
	/// `collect_grace`.
	UseCurrentAnchor,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TailAllPostLogsBehavior {
	/// Does nothing after fetching log.
	None,

	/// Returns after log was fetched if messages were found. Otherwise
	/// polls subscriptions as usual.
	ReturnIfMessages,

	/// Always returns immediately after log was fetched.
	ReturnAlways,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TailAllAnchorStatus {
	/// Anchor was valid.
	Valid,

	/// Anchor was older than the TTL of the message.
	Expired,

	/// Anchor was older that the TTL of the message, but
	/// `TailAllAnchorExpiredBehavior::UseCurrentAnchor` was provided, so we
	/// used the current timestamp.
	ExpiredFallbackToCurrent,

	/// Message did not provide a TTL.
	NoMessageTtl,
}

#[derive(Debug)]
pub struct TailAllResponse<M>
where
	M: Message + Debug,
{
	pub messages: Vec<ReceivedMessage<M>>,
	pub anchor_status: TailAllAnchorStatus,
	pub subs: Vec<SubscriptionHandle<M>>,
}

impl<M> TailAllResponse<M>
where
	M: Message + Debug,
{
	/// Returns a placeholder `TailAllResponse`.
	pub fn empty() -> Self {
		TailAllResponse {
			messages: Vec::new(),
			anchor_status: TailAllAnchorStatus::Valid,
			subs: Vec::new(),
		}
	}
}
