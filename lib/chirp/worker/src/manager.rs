use chirp_metrics as metrics;
use futures_util::StreamExt;
use global_error::{GlobalError, GlobalResult};
use prost::Message;
use redis::{self, AsyncCommands};
use rivet_connection::Connection;
use rivet_operation::OperationContext;
use rivet_pools::prelude::*;
use rivet_util::CleanExit;
use std::{
	fmt::{self, Debug},
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::time;
use tracing::Instrument;
use types::rivet::chirp;

use crate::{
	config::{Config, WorkerKind},
	error::ManagerError,
	request::{RedisMessageMeta, Request},
	worker::Worker,
};

/// How long to wait before retrying a connection if an error occurs.
const CONN_ERROR_THROTTLE: Duration = Duration::from_secs(1);

/// How frequently to call `XAUTOCLAIM`.
const CLAIM_INTERVAL: Duration = Duration::from_secs(5);

struct WorkerResponseSummary {
	error: Option<chirp::response::Err>,
}

pub struct Manager<W>
where
	W: Worker,
{
	pub(crate) config: Arc<Config>,

	worker: W,
	shared_client: chirp_client::SharedClientHandle,
	cache: rivet_cache::Cache,
	pub(crate) pools: rivet_pools::Pools,

	// Cloned copies of the pools that we've asserted exist.
	nats: NatsPool,
	redis_chirp: RedisPool,
	redis_cache: RedisPool,
}

impl<W> Debug for Manager<W>
where
	W: Worker,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Manager").finish()
	}
}

impl<W> Manager<W>
where
	W: Worker,
{
	#[tracing::instrument(err, skip(shared_client, pools, cache, worker))]
	pub async fn new(
		config: Config,
		shared_client: chirp_client::SharedClientHandle,
		pools: rivet_pools::Pools,
		cache: rivet_cache::Cache,
		worker: W,
	) -> Result<Arc<Self>, ManagerError> {
		tracing::info!(config_data = ?config, "init worker manager");

		let nats = pools.nats()?;
		let redis_chirp = pools.redis_chirp()?;
		let redis_cache = pools.redis_cache()?;

		let manager = Arc::new(Manager {
			config: Arc::new(config),
			worker,
			shared_client,
			cache,
			pools,

			nats,
			redis_chirp,
			redis_cache,
		});

		Ok(manager)
	}

	#[doc(hidden)]
	pub fn __test_conn(&self) -> nats::Client {
		self.pools.nats().unwrap()
	}

	#[tracing::instrument]
	pub async fn start(self: Arc<Self>) -> Result<(), ManagerError> {
		// Build the subscription
		match &self.config.worker_kind {
			WorkerKind::Rpc { group } => {
				let subject =
					chirp_client::endpoint::subject(&self.config.region, &self.config.service_name);

				self.clone().rpc_receiver(subject, group.clone()).await;
			}
			WorkerKind::Consumer { topic, group } => {
				// Create a dedicated connection for blocking Redis requests
				// that won't block other requests in the pool.
				let url = std::env::var("REDIS_URL_CHIRP").expect("REDIS_URL_CHIRP");
				let redis_chirp_conn = redis::cluster::ClusterClient::new(vec![url.as_str()])
					.map_err(ManagerError::BuildRedis)?
					.get_async_connection()
					.await
					.map_err(ManagerError::BuildRedis)?;

				self.clone()
					.worker_receiver(
						redis_chirp_conn,
						topic.clone(),
						format!("{}--{}", group, W::NAME),
					)
					.await;
			}
		}

		Ok(())
	}

	#[tracing::instrument]
	async fn rpc_receiver(self: Arc<Self>, subject: String, group: String) -> CleanExit {
		'conn: loop {
			// Acquire subscription
			tracing::info!(%subject, %group, "creating rpc subscription");
			let mut sub = match self
				.nats
				.queue_subscribe(subject.clone(), group.clone())
				.await
			{
				Ok(x) => x,
				Err(err) => {
					tracing::error!(?err, "failed to create rpc subscription");
					tokio::time::sleep(CONN_ERROR_THROTTLE).await;
					continue 'conn;
				}
			};

			tracing::info!("started");
			loop {
				tokio::select! {
					msg = sub.next() => {
						// Handle error receiving message
						let mut msg = match msg {
							Some(msg) => msg,
							None => {
								tracing::error!("nats subscription error");
								tokio::time::sleep(CONN_ERROR_THROTTLE).await;
								continue 'conn;
							}
						};

						// Take ownership of the data and handle the message
						let msg_data = std::mem::take(&mut msg.payload);
						let spawn_res = tokio::task::Builder::new()
							.name("chirp_worker::handle_raw_msg_rpc")
							.spawn(self.clone().handle_raw_msg(msg_data.to_vec(), Some(msg), None));
						if let Err(err) = spawn_res {
							tracing::error!(?err, "failed to spawn handle_raw_msg_rpc task");
						}
					}
					_ = tokio::signal::ctrl_c() => {
						tracing::info!("sub received ctrl c");
						return CleanExit;
					}
				}
			}
		}
	}

	#[tracing::instrument(skip(redis_chirp_conn))]
	async fn worker_receiver(
		self: Arc<Self>,
		mut redis_chirp_conn: RedisPool,
		topic: String,
		group: String,
	) -> CleanExit {
		let topic_key = chirp_client::redis_keys::message_topic(&topic);
		let consumer = self.config.worker_instance.clone();

		// Retry with a 15 second padding
		let pending_retry_time = W::TIMEOUT + Duration::from_secs(15);

		// Timestamp of the last call to `XAUTOCLAIM`
		let mut last_claim_ts = None;

		// Setup consumer
		'setup: loop {
			// Create stream & group
			tracing::info!(%topic, %group, %topic_key, "creating group and stream");
			match redis_chirp_conn
				.xgroup_create_mkstream::<_, _, _, ()>(&topic_key, &group, "$")
				.await
			{
				Ok(_) => {}
				Err(err) if err.code() == Some("BUSYGROUP") => {
					tracing::info!("consumer group already created");
					break 'setup;
				}
				Err(err) => {
					tracing::error!(?err, err2=%err, "failed to create group and stream, retrying");
					tokio::time::sleep(CONN_ERROR_THROTTLE).await;
					continue 'setup;
				}
			}
		}

		tracing::info!("started");
		'msg: loop {
			// TODO: Acquire an independent connection

			// Pull messages
			let pull_fut = self.clone().pull_redis_stream_msgs(
				&mut redis_chirp_conn,
				&topic_key,
				&group,
				&consumer,
				pending_retry_time,
				last_claim_ts,
			);
			tokio::select! {
				res = pull_fut => {
					match res {
						PullRedisStatus::PulledMessages => {}
						PullRedisStatus::ClaimedPending => {
							last_claim_ts = Some(Instant::now());
						}
						PullRedisStatus::ConnErr =>  {
							tokio::time::sleep(CONN_ERROR_THROTTLE).await;
							continue 'msg;
						}
					}
				}
				_ = tokio::signal::ctrl_c() => {
					tracing::info!("sub received ctrl c");
					return CleanExit;
				}
			};
		}
	}

	#[tracing::instrument(skip(self, redis_chirp_conn))]
	async fn pull_redis_stream_msgs(
		self: Arc<Self>,
		redis_chirp_conn: &mut RedisPool,
		topic_key: &str,
		group: &str,
		consumer: &str,
		pending_retry_time: Duration,
		last_claim_ts: Option<Instant>,
	) -> PullRedisStatus {
		if last_claim_ts.map_or(true, |x| Instant::now().duration_since(x) > CLAIM_INTERVAL) {
			// We don't use XAUTOCLAIM because of https://github.com/redis/redis/issues/10198

			// Read pending messages until we manage to claim some
			let mut claim_attempts = 0;
			let claimed_msgs = loop {
				// Fetch pending messages
				tracing::trace!("fetching pending messages");
				let pending_msgs = match redis::cmd("XPENDING")
					.arg(&topic_key)
					.arg(&group)
					.arg("IDLE")
					.arg(pending_retry_time.as_millis() as i64)
					.arg("-")
					.arg("+")
					.arg(1usize)
					.query_async::<_, redis::streams::StreamPendingCountReply>(redis_chirp_conn)
					.await
				{
					Ok(x) => x,
					Err(err) => {
						tracing::error!(?err, "failed to fetch pending messages");
						return PullRedisStatus::ConnErr;
					}
				};

				// If no pending messages, skip
				if pending_msgs.ids.is_empty() {
					tracing::trace!("not pending messages in pel");
					break Vec::new();
				}

				// Attempt to claim messages. This will return the messages that are
				// actually claimed for this consumer. This is important in order to
				// handle race conditions with other services claiming the same
				// messages.
				let msg_ids = pending_msgs
					.ids
					.iter()
					.map(|x| x.id.as_str())
					.collect::<Vec<_>>();
				if msg_ids.is_empty() {
					tracing::trace!("no pending messages");
					return PullRedisStatus::ClaimedPending;
				}
				tracing::trace!(?msg_ids, "claiming pending messages");
				let claimed_msgs = match redis_chirp_conn
					.xclaim::<_, _, _, _, _, redis::streams::StreamClaimReply>(
						topic_key,
						group,
						consumer,
						pending_retry_time.as_millis() as i64,
						&msg_ids,
					)
					.await
				{
					Ok(x) => x,
					Err(err) => {
						tracing::error!(?err, "failed to claim messages");
						return PullRedisStatus::ConnErr;
					}
				};
				tracing::trace!(pending_len = ?pending_msgs.ids.len(), claimed_len = ?claimed_msgs.ids.len(), "claimed pending messages");

				// Find and delete any messages that are in the PEL but already
				// deleted from the stream. These are messages that have already
				// been trimmed from the stream but still live in PEL (which is
				// what XPENDING reads from).
				//
				// If we don't do this, XPENDING will keep returning the same
				// messages and XCLAIM will never return the messages themselves
				// since they no longer exist in the stream.
				//
				// See https://github.com/redis/redis/issues/7021
				//
				// This behavior may be different on Redis 7. See
				// https://github.com/redis/redis/pull/10227
				let mut pipe = redis::pipe();
				for msg in &pending_msgs.ids {
					// This message was claimed successfully, do nothing
					if claimed_msgs.ids.iter().any(|x| x.id == msg.id) {
						continue;
					}

					// Check if the message still exists in the stream
					let messages = match redis_chirp_conn
						.xrange::<_, _, _, redis::streams::StreamRangeReply>(
							topic_key, &msg.id, &msg.id,
						)
						.await
					{
						Ok(x) => x,
						Err(err) => {
							tracing::error!(
								?err,
								msg_id = ?msg.id,
								"failed to read unclaimed pending message, continuing"
							);
							continue;
						}
					};
					if !messages.ids.is_empty() {
						tracing::warn!(msg_id = ?msg.id, "message exists in stream but was unable to be claimed, unsure what's going on here");
						continue;
					}

					// Message does not exist in the stream, delete it
					tracing::warn!(
						msg_id = ?msg.id,
						"message does not exist in the stream anymore, deleting from PEL"
					);
					pipe.xack(topic_key, group, &[&msg.id]);
				}

				// Execute ack messages pipe
				match pipe
					.query_async::<_, Vec<i32>>(&mut *redis_chirp_conn)
					.await
				{
					Ok(_) => {
						tracing::trace!("successfully deleted message no longer in stream");
					}
					Err(err) => {
						tracing::error!(
							?err,
							"failed to delete messages no longer in stream, continuing anyway"
						);
					}
				}

				// Determine wether or not to break the loop
				claim_attempts += 1;
				if !claimed_msgs.ids.is_empty() {
					break claimed_msgs.ids;
				} else if claim_attempts > 16 {
					tracing::warn!("exceeded 16 claim attempts, breaking claim loop");
					break claimed_msgs.ids;
				} else {
					tracing::info!("no claimed messages, requesting more pending messages");
				}
			};

			// Handle found messages
			'msg: for msg in claimed_msgs {
				let msg_value = if let Some(x) = msg.map.get("m") {
					x
				} else {
					// This message will be re-processed in case we have a
					// schema migration.
					tracing::warn!(id = %msg.id, map = ?msg.map, "missing field `b` in redis stream message");
					continue 'msg;
				};

				let msg_buf = match redis::from_redis_value::<Vec<u8>>(msg_value) {
					Ok(x) => x,
					Err(err) => {
						tracing::warn!(
							?err,
							?msg_value,
							"could not decode message redis value to buf"
						);
						continue 'msg;
					}
				};

				// Process the message
				let spawn_res = tokio::task::Builder::new()
					.name("chirp_worker::handle_raw_msg_consumer_pending")
					.spawn(self.clone().handle_raw_msg(
						msg_buf,
						None,
						Some(RedisMessageMeta {
							topic_key: topic_key.to_owned(),
							group: group.to_owned(),
							id: msg.id,
							parameters: None,
						}),
					));
				if let Err(err) = spawn_res {
					tracing::error!(?err, "failed to spawn handle_raw_msg_consumer_pending task");
				}
			}

			PullRedisStatus::ClaimedPending
		} else {
			// Read a message from the Redis stream
			let keys = &[&topic_key];
			let read_options = redis::streams::StreamReadOptions::default()
				.group(&group, &consumer)
				.block(30_000)
				.count(1);
			let res = match redis_chirp_conn
				.xread_options::<_, _, redis::streams::StreamReadReply>(keys, &[">"], &read_options)
				.await
			{
				Ok(x) => x,
				Err(err) => {
					tracing::error!(?err, "failed to read stream messages");
					return PullRedisStatus::ConnErr;
				}
			};

			let key = if let Some(x) = res.keys.first() {
				x
			} else {
				tracing::trace!("no keys provided");
				return PullRedisStatus::PulledMessages;
			};

			tracing::trace!(len = key.ids.len(), "read stream messages");
			'read_id: for id in &key.ids {
				let msg_value = if let Some(x) = id.map.get("m") {
					x
				} else {
					// This message will be re-processed in case we have a
					// schema migration.
					tracing::warn!(id = %id.id, map = ?id.map, "missing field `b` in redis stream message");
					continue 'read_id;
				};

				let msg_buf = match redis::from_redis_value::<Vec<u8>>(msg_value) {
					Ok(x) => x,
					Err(err) => {
						tracing::warn!(
							?err,
							?msg_value,
							"could not decode message redis value to buf"
						);
						continue 'read_id;
					}
				};

				// Process the message
				let spawn_res = tokio::task::Builder::new()
					.name("chirp_worker::handle_raw_msg_consumer")
					.spawn(self.clone().handle_raw_msg(
						msg_buf,
						None,
						Some(RedisMessageMeta {
							topic_key: topic_key.to_owned(),
							group: group.to_owned(),
							id: id.id.clone(),
							parameters: None,
						}),
					));
				if let Err(err) = spawn_res {
					tracing::error!(?err, "failed to spawn handle_raw_msg_consumer task");
				}
			}

			PullRedisStatus::PulledMessages
		}
	}

	/// Processes a `nats::Message` in to a `Request`.
	#[tracing::instrument(level = "trace", skip_all)]
	async fn handle_raw_msg(
		self: Arc<Self>,
		raw_msg_buf: Vec<u8>,
		nats_message: Option<nats::Message>,
		mut redis_message_meta: Option<RedisMessageMeta>,
	) {
		let worker_name = match &self.config.worker_kind {
			WorkerKind::Rpc { .. } => self.config.service_name.clone(),
			WorkerKind::Consumer { group, .. } => format!("{}--{}", group, W::NAME),
		};

		tracing::trace!(
			?worker_name,
			bytes = raw_msg_buf.len(),
			?nats_message,
			"received raw message"
		);

		// Parse request structure
		let (req_id_proto, ray_id_proto, req_ts, trace, body_buf, dont_log_body, req_debug) =
			match &self.config.worker_kind {
				WorkerKind::Rpc { .. } => {
					match chirp::Request::decode(raw_msg_buf.as_slice()) {
						Ok(req) => {
							let reply = if let Some(x) =
								nats_message.as_ref().and_then(|x| x.reply.clone())
							{
								x
							} else {
								tracing::error!("handling rpc without provided nats reply");
								return;
							};

							// Ack the request
							{
								// Build response
								let res = chirp::Response {
									kind: Some(chirp::response::Kind::Ack(chirp::response::Ack {})),
								};
								let mut res_buf =
									Vec::with_capacity(prost::Message::encoded_len(&res));
								match prost::Message::encode(&res, &mut res_buf) {
									Ok(_) => {}
									Err(err) => {
										tracing::error!(
											?err,
											"failed to encode ack message, skipping request"
										);
										return;
									}
								}

								// Send ack response.
								//
								// We do this in the background (which will race
								// with the main response) since we want to handle
								// the response as fast as possible without waiting.
								// This means that the ack and the response may be
								// out of order, which is expected.
								tracing::trace!("sending ack message");
								match self.nats.publish(reply, res_buf.into()).await {
									Ok(_) => {}
									Err(err) => {
										tracing::error!(?err, "failed to send ack response");
									}
								}
							}

							// Parse the request
							tracing::info!(
								req_id = ?req.req_id,
								ray_id = ?req.ray_id,
								ts = ?req.ts,
								trace = ?req.trace,
								body_bytes = ?req.body.len(),
								"received request"
							);
							(
								req.req_id,
								req.ray_id,
								req.ts,
								req.trace,
								req.body,
								req.dont_log_body,
								req.debug,
							)
						}
						Err(err) => {
							tracing::error!(?err, "failed to decode chirp request");
							return;
						}
					}
				}
				WorkerKind::Consumer { .. } => match chirp::Message::decode(raw_msg_buf.as_slice())
				{
					Ok(msg) => {
						// Calculate recv lag
						let recv_lag =
							(rivet_util::timestamp::now() as f64 - msg.ts as f64) / 1000.;
						metrics::CHIRP_MESSAGE_RECV_LAG
							.with_label_values(&[&worker_name])
							.observe(recv_lag);

						tracing::info!(
							// TODO: Add back once we can decode UUIDs in Chirp
							// req_id = ?msg.req_id,
							// ray_id = ?msg.ray_id,
							parameters = ?msg.parameters,
							ts = ?msg.ts,
							trace = ?msg.trace,
							body_bytes = ?msg.body.len(),
							?recv_lag,
							"received message"
						);

						// Enrich Redis metadata for debugging
						if let Some(x) = &mut redis_message_meta {
							x.parameters = Some(msg.parameters.clone());
						}

						(
							msg.req_id, msg.ray_id, msg.ts, msg.trace, msg.body, false, None,
						)
					}
					Err(err) => {
						tracing::error!(?err, "failed to decode chirp message");
						return;
					}
				},
			};
		let (req_id, ray_id) = if let (Some(req_id), Some(ray_id)) = (req_id_proto, ray_id_proto) {
			(req_id.as_uuid(), ray_id.as_uuid())
		} else {
			tracing::error!("missing request data");
			return;
		};

		// Parse body
		let req_body = match W::Request::decode(body_buf.as_slice()) {
			Ok(x) => x,
			Err(err) => {
				tracing::error!(?err, "failed to decode request body");
				return;
			}
		};
		if !dont_log_body {
			tracing::info!(?req_body, "request");
		} else {
			tracing::info!("request")
		}

		let worker_req = {
			// Build client
			let ts = rivet_util::timestamp::now();
			let client = self.shared_client.clone().wrap_with(
				req_id,
				ray_id,
				ts,
				{
					let mut x = trace.clone();
					x.push(chirp::TraceEntry {
						context_name: worker_name.clone(),
						req_id: req_id_proto.clone(),
						ts: rivet_util::timestamp::now(),
						run_context: chirp::RunContext::Service as i32,
					});
					x
				},
				chirp_perf::PerfCtxInner::new(self.redis_cache.clone(), ts, req_id, ray_id),
			);
			let conn = Connection::new(client, self.pools.clone(), self.cache.clone());

			let ts = req_debug
				.as_ref()
				.and_then(|dbg| {
					if dbg.override_ts != 0 {
						Some(dbg.override_ts)
					} else {
						None
					}
				})
				.unwrap_or(ts);

			// Build request
			Request {
				conn: conn.clone(),
				nats_message,
				redis_message_meta,
				req_id,
				ray_id,
				ts,
				req_ts,
				op_ctx: OperationContext::new(
					worker_name,
					W::TIMEOUT,
					conn,
					req_id,
					ray_id,
					ts,
					req_ts,
					req_body,
					trace,
				),
				dont_log_body,
			}
		};

		// Handle request
		self.handle_req(worker_req).await;
	}

	#[tracing::instrument(
		skip_all,
		fields(
			worker_name = %req.op_ctx.name(),
			req_id = %req.req_id,
			ray_id = %req.ray_id
		)
	)]
	async fn handle_req(self: Arc<Self>, req: Request<W::Request>) {
		let worker_name = req.op_ctx.name().to_string();

		// Record metrics
		metrics::CHIRP_REQUEST_PENDING
			.with_label_values(&[&worker_name])
			.inc();
		metrics::CHIRP_REQUEST_TOTAL
			.with_label_values(&[&worker_name])
			.inc();

		let start_instant = Instant::now();

		// Process the request
		let summary = self.clone().handle_req_inner(req).await.ok();

		// Record metrics
		{
			// Record error
			let error_code_str = if let Some(summary) = &summary {
				if let Some(error) = &summary.error {
					match &error.kind {
						Some(chirp::response::err::Kind::Internal(error)) => {
							let error_code_str = error.code.to_string();
							metrics::CHIRP_REQUEST_ERRORS
								.with_label_values(&[&worker_name, &error_code_str, &error.ty])
								.inc();

							error_code_str
						}
						Some(chirp::response::err::Kind::BadRequest(error)) => {
							let error_code_str = error.code.to_string();
							metrics::CHIRP_REQUEST_ERRORS
								.with_label_values(&[&worker_name, &error_code_str, "bad_request"])
								.inc();

							error_code_str
						}
						None => String::new(),
					}
				} else {
					// No error
					String::new()
				}
			} else {
				// Internal Chirp error
				"-1".to_owned()
			};

			// Other request metrics
			let dt = start_instant.elapsed().as_secs_f64();
			metrics::CHIRP_REQUEST_PENDING
				.with_label_values(&[&worker_name])
				.dec();
			metrics::CHIRP_REQUEST_DURATION
				.with_label_values(&[&worker_name, error_code_str.as_str()])
				.observe(dt);
		}
	}

	#[tracing::instrument(level = "trace", err, skip_all)]
	async fn handle_req_inner(
		self: Arc<Self>,
		req: Request<W::Request>,
	) -> Result<WorkerResponseSummary, ManagerError> {
		if req.dont_log_body {
			tracing::info!(request = ?req, "handling req");
		} else {
			tracing::info!(request = ?req, body = ?req.op_ctx.body(), "handling req");
		}

		let is_recursive = req
			.op_ctx
			.trace()
			.iter()
			.any(|x| x.context_name == req.op_ctx.name());
		let handle_res = if is_recursive {
			Ok(Err(GlobalError::new(
				ManagerError::RecursiveRequest {
					worker_name: req.op_ctx.name().into(),
				},
				chirp::ErrorCode::RecursiveRequest,
			)))
		} else {
			// Decrease the timeout based on how many callers there are. This
			// prevents a race condition from the chirp client's timeout and
			// this timeout that lets us respond safely with a timeout error.
			//
			// We have to decrease for each item in the trace since if multiple
			// requests have a timeout of 60s, we want the timeout error to
			// safely cascade.
			let reduced_timeout = req.op_ctx.timeout()
				- Duration::from_secs(1) * req.op_ctx.trace().len().min(10) as u32;

			// Handle request normally with timeout
			time::timeout(reduced_timeout, self.clone().handle_req_with_retry(&req)).await
		};

		// Handle the response
		let summary = self.clone().handle_worker_res(req, handle_res).await?;

		Ok(summary)
	}

	/// Enables requests to be retried immediately if needed.
	///
	/// This is executed within a timeout, so the overall timeout is not restarted
	/// when retrying the request.
	#[tracing::instrument(level = "trace", err, skip_all)]
	async fn handle_req_with_retry(
		self: Arc<Self>,
		req: &Request<W::Request>,
	) -> GlobalResult<W::Response> {
		self.worker
			.handle(req.op_ctx())
			.instrument(tracing::info_span!("handle", name = %W::NAME))
			.await

		// // TODO: Add back
		// // Will retry 4 times. This will take a maximum of 15 seconds.
		// let mut backoff = rivet_util::Backoff::new(5, Some(4), 1_000, 1_000);
		// loop {
		// 	let res = self
		// 		.worker
		// 		.handle(req.op_ctx())
		// 		.instrument(tracing::info_span!("handle", name = %W::NAME))
		// 		.await;

		// 	// Attempt to retry the request with backoff
		// 	if matches!(
		// 		res,
		// 		Err(GlobalError::Internal {
		// 			retry_immediately: true,
		// 			..
		// 		})
		// 	) {
		// 		tracing::info!("ticking request retry backoff");

		// 		if backoff.tick().await {
		// 			tracing::warn!("retry request failed too many times");
		// 			return res;
		// 		} else {
		// 			tracing::info!("retrying request");
		// 		}
		// 	} else {
		// 		// Return result immediately
		// 		return res;
		// 	}
		// }
	}

	#[tracing::instrument(level = "trace", skip_all)]
	async fn handle_worker_res(
		self: Arc<Self>,
		req: Request<W::Request>,
		handle_res: Result<GlobalResult<W::Response>, time::error::Elapsed>,
	) -> Result<WorkerResponseSummary, ManagerError> {
		// Log response status and build RPC response if needed
		let (rpc_res, debug_error): (Option<chirp::Response>, Option<chirp::DebugServiceError>) =
			match handle_res {
				Ok(Ok(res)) => {
					if req.dont_log_body {
						tracing::info!("worker success");
					} else {
						tracing::info!(?res, "worker success");
					}

					// Ack the message if needed
					if let Some(msg_meta) = req.redis_message_meta {
						tracing::info!(?msg_meta, "acking message");
						let spawn_res = tokio::task::Builder::new()
							.name("chirp_worker::consumer_ack_ok")
							.spawn(self.clone().consumer_ack(msg_meta));
						if let Err(err) = spawn_res {
							tracing::error!(?err, "failed to spawn consumer_ack_ok task");
						}
					}

					(
						// Serialize response body if a reply is needed
						if let WorkerKind::Rpc { .. } = &self.config.worker_kind {
							let mut body_buf =
								Vec::with_capacity(prost::Message::encoded_len(&res));
							prost::Message::encode(&res, &mut body_buf)
								.map_err(ManagerError::EncodeResponseBody)?;

							Some(chirp::Response {
								kind: Some(chirp::response::Kind::Ok(chirp::response::Ok {
									body: body_buf,
								})),
							})
						} else {
							None
						},
						None,
					)
				}
				Ok(Err(err)) => {
					tracing::error!(?err, "worker error");

					let err_proto = Into::<chirp::response::Err>::into(err);

					if let Some(chirp::response::err::Kind::Internal(error)) = &err_proto.kind {
						// Ack the message if recursive (i.e. this will never succeed
						// again)
						if let (Some(msg_meta), true) = (
							req.redis_message_meta,
							error.code == chirp::ErrorCode::RecursiveRequest as i32,
						) {
							tracing::error!(
								?msg_meta,
								"acking message because we can never recover from this error"
							);
							let spawn_res = tokio::task::Builder::new()
								.name("chirp_worker::consumer_ack_unrecoverable")
								.spawn(self.clone().consumer_ack(msg_meta));
							if let Err(err) = spawn_res {
								tracing::error!(
									?err,
									"failed to spawn consumer_ack_unrecoverable task"
								);
							}
						}
					}

					(
						if let WorkerKind::Rpc { .. } = &self.config.worker_kind {
							Some(chirp::Response {
								kind: Some(chirp::response::Kind::Err(err_proto.clone())),
							})
						} else {
							None
						},
						Some(chirp::DebugServiceError {
							context_name: req.op_ctx.name().into(),
							error: Some(err_proto),
						}),
					)
				}
				Err(err) => {
					tracing::error!(?err, "worker task timed out");

					let err_proto = Into::<chirp::response::Err>::into(GlobalError::new(
						ManagerError::RequestTaskTimedOut,
						chirp::ErrorCode::TimedOut,
					));

					(
						if let WorkerKind::Rpc { .. } = &self.config.worker_kind {
							Some(chirp::Response {
								kind: Some(chirp::response::Kind::Err(err_proto.clone())),
							})
						} else {
							None
						},
						Some(chirp::DebugServiceError {
							context_name: req.op_ctx.name().into(),
							error: Some(err_proto),
						}),
					)
				}
			};

		// Send reply
		if let (Some(nats_message), Some(rpc_res)) = (&req.nats_message, rpc_res) {
			// Serialize RPC res
			let mut res_buf = Vec::with_capacity(prost::Message::encoded_len(&rpc_res));
			prost::Message::encode(&rpc_res, &mut res_buf).map_err(ManagerError::EncodeResponse)?;

			tracing::info!(res_bytes = ?res_buf.len(), "sending rpc nats response");

			let reply = nats_message
				.reply
				.clone()
				.ok_or(ManagerError::MissingNatsReply)?;
			self.nats
				.publish(reply, res_buf.into())
				.await
				.map_err(ManagerError::RequestRespond)?;
		}

		Ok(WorkerResponseSummary {
			error: debug_error.and_then(|x| x.error),
		})
	}

	/// Acknowledges the Redis message from a consumer worker.
	#[tracing::instrument]
	async fn consumer_ack(self: Arc<Self>, msg_meta: RedisMessageMeta) {
		// let mut backoff = rivet_util::Backoff::default();
		// loop {
		// 	if backoff.tick().await {
		// 		tracing::error!("acking stream message failed too many times, aborting");
		// 		return;
		// 	}

		// Acknowledge the messages
		let mut redis_chirp = self.redis_chirp.clone();
		match redis_chirp
			.xack::<_, _, _, ()>(&msg_meta.topic_key, &msg_meta.group, &[&msg_meta.id])
			.await
		{
			Ok(_) => {
				tracing::info!(?msg_meta, "acknowledged stream message");
				// break;
			}
			Err(err) => {
				tracing::error!(?err, "failed to ack message");
			}
		}
		// }
	}
}

enum PullRedisStatus {
	PulledMessages,
	ClaimedPending,
	ConnErr,
}
