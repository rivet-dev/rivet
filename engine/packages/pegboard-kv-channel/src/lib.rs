//! KV channel WebSocket handler for the engine.
//!
//! Serves the KV channel protocol at /kv/connect for native SQLite to route
//! page-level KV operations over WebSocket. See
//! docs-internal/engine/NATIVE_SQLITE_DATA_CHANNEL.md for the full spec.

mod metrics;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::TryStreamExt;
use gas::prelude::*;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use hyper_tungstenite::tungstenite::Message;
use pegboard::actor_kv;
use rivet_guard_core::{
	ResponseBody, WebSocketHandle, custom_serve::CustomServeTrait,
	request_context::RequestContext,
};
use tokio::sync::{Mutex, mpsc, watch};
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use uuid::Uuid;

pub use rivet_kv_channel_protocol as protocol;

use actor_kv::{MAX_KEY_SIZE, MAX_KEYS, MAX_PUT_PAYLOAD_SIZE, MAX_VALUE_SIZE};

/// Overhead added by KeyWrapper tuple packing (NESTED prefix byte + NIL suffix
/// byte). Must match `KeyWrapper::tuple_len` in
/// `engine/packages/pegboard/src/keys/actor_kv.rs`.
const KEY_WRAPPER_OVERHEAD: usize = 2;

/// Maximum number of actors a single connection can have open simultaneously.
/// Prevents a malicious client from exhausting memory via unbounded actor_channels.
const MAX_ACTORS_PER_CONNECTION: usize = 1000;

/// Shared state across all KV channel connections.
pub struct KvChannelState {
	/// Maps actor_id string to the connection_id holding the single-writer lock and a reference
	/// to that connection's open_actors set. The Arc reference allows lock eviction to remove the
	/// actor from the old connection's set without acquiring the global lock on the KV hot path.
	actor_locks: Mutex<HashMap<String, (Uuid, Arc<Mutex<HashSet<String>>>)>>,
}

pub struct PegboardKvChannelCustomServe {
	ctx: StandaloneCtx,
	state: Arc<KvChannelState>,
}

impl PegboardKvChannelCustomServe {
	pub fn new(ctx: StandaloneCtx) -> Self {
		Self {
			ctx,
			state: Arc::new(KvChannelState {
				actor_locks: Mutex::new(HashMap::new()),
			}),
		}
	}
}

#[async_trait]
impl CustomServeTrait for PegboardKvChannelCustomServe {
	#[tracing::instrument(skip_all)]
	async fn handle_request(
		&self,
		_req: hyper::Request<Full<Bytes>>,
		_req_ctx: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		let response = Response::builder()
			.status(StatusCode::OK)
			.header("Content-Type", "text/plain")
			.body(ResponseBody::Full(Full::new(Bytes::from(
				"kv-channel WebSocket endpoint",
			))))?;
		Ok(response)
	}

	#[tracing::instrument(skip_all)]
	async fn handle_websocket(
		&self,
		req_ctx: &mut RequestContext,
		ws_handle: WebSocketHandle,
		_after_hibernation: bool,
	) -> Result<Option<CloseFrame>> {
		let ctx = self.ctx.with_ray(req_ctx.ray_id(), req_ctx.req_id())?;
		let state = self.state.clone();

		// Parse URL params.
		let url = url::Url::parse(&format!("ws://placeholder{}", req_ctx.path()))
			.context("failed to parse WebSocket URL")?;
		let params: HashMap<String, String> = url
			.query_pairs()
			.map(|(k, v)| (k.to_string(), v.to_string()))
			.collect();

		// Validate protocol version.
		let protocol_version: u32 = params
			.get("protocol_version")
			.context("missing protocol_version query param")?
			.parse()
			.context("invalid protocol_version")?;
		anyhow::ensure!(
			protocol_version == protocol::PROTOCOL_VERSION,
			"unsupported protocol version: {protocol_version}, expected {}",
			protocol::PROTOCOL_VERSION
		);

		// Resolve namespace.
		let namespace_name = params
			.get("namespace")
			.context("missing namespace query param")?
			.clone();
		let namespace = ctx
			.op(namespace::ops::resolve_for_name_global::Input {
				name: namespace_name.clone(),
			})
			.await
			.with_context(|| format!("failed to resolve namespace: {namespace_name}"))?
			.ok_or_else(|| namespace::errors::Namespace::NotFound.build())
			.with_context(|| format!("namespace not found: {namespace_name}"))?;

		// Assign connection ID. Uses UUID to eliminate any possibility of ID collision.
		let conn_id = Uuid::new_v4();
		let namespace_id = namespace.namespace_id;

		tracing::info!(%conn_id, %namespace_id, "kv channel connection established");

		// Track actors opened by this connection for cleanup on disconnect.
		let open_actors: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
		let last_pong_ts = Arc::new(AtomicI64::new(util::timestamp::now()));

		// Run the connection loop. Any error triggers cleanup below.
		let result = run_connection(
			ctx.clone(),
			state.clone(),
			ws_handle,
			conn_id,
			namespace_id,
			open_actors.clone(),
			last_pong_ts,
		)
		.await;

		// Release all locks held by this connection. Only remove entries where the lock is still
		// held by this conn_id, since another connection may have evicted it via ActorOpenRequest.
		{
			let open = open_actors.lock().await;
			let mut locks = state.actor_locks.lock().await;
			for actor_id in open.iter() {
				if let Some((lock_conn, _)) = locks.get(actor_id) {
					if *lock_conn == conn_id {
						locks.remove(actor_id);
						tracing::debug!(%conn_id, %actor_id, "released actor lock on disconnect");
					}
				}
			}
		}

		tracing::info!(%conn_id, "kv channel connection closed");

		result.map(|_| None)
	}
}

// MARK: Connection lifecycle

async fn run_connection(
	ctx: StandaloneCtx,
	state: Arc<KvChannelState>,
	ws_handle: WebSocketHandle,
	conn_id: Uuid,
	namespace_id: Id,
	open_actors: Arc<Mutex<HashSet<String>>>,
	last_pong_ts: Arc<AtomicI64>,
) -> Result<()> {
	let ping_interval =
		Duration::from_millis(ctx.config().pegboard().runner_update_ping_interval_ms());
	let ping_timeout_ms = ctx.config().pegboard().runner_ping_timeout_ms();

	let (ping_abort_tx, ping_abort_rx) = watch::channel(());

	// Spawn ping task.
	let ping_ws = ws_handle.clone();
	let ping_last_pong = last_pong_ts.clone();
	let ping = tokio::spawn(async move {
		ping_task(
			ping_ws,
			ping_last_pong,
			ping_abort_rx,
			ping_interval,
			ping_timeout_ms,
		)
		.await
	});

	// Run message loop.
	let msg_result = message_loop(
		&ctx,
		&state,
		&ws_handle,
		conn_id,
		namespace_id,
		&open_actors,
		&last_pong_ts,
	)
	.await;

	// Signal ping task to stop and wait for it.
	let _ = ping_abort_tx.send(());
	let _ = ping.await;

	msg_result
}

// MARK: Ping task

async fn ping_task(
	ws_handle: WebSocketHandle,
	last_pong_ts: Arc<AtomicI64>,
	mut abort_rx: watch::Receiver<()>,
	interval: Duration,
	timeout_ms: i64,
) -> Result<()> {
	loop {
		tokio::select! {
			_ = tokio::time::sleep(interval) => {}
			_ = abort_rx.changed() => return Ok(()),
		}

		// Check pong timeout.
		let last = last_pong_ts.load(Ordering::Relaxed);
		let now = util::timestamp::now();
		if now - last > timeout_ms {
			tracing::warn!("kv channel ping timed out, closing connection");
			return Err(anyhow::anyhow!("ping timed out"));
		}

		// Send ping.
		let ping = protocol::ToClient::ToClientPing(protocol::ToClientPing { ts: now });
		let data = protocol::encode_to_client(&ping)?;
		ws_handle.send(Message::Binary(data.into())).await?;
	}
}

// MARK: Message loop

async fn message_loop(
	ctx: &StandaloneCtx,
	state: &Arc<KvChannelState>,
	ws_handle: &WebSocketHandle,
	conn_id: Uuid,
	namespace_id: Id,
	open_actors: &Arc<Mutex<HashSet<String>>>,
	last_pong_ts: &AtomicI64,
) -> Result<()> {
	let ws_rx = ws_handle.recv();
	let mut ws_rx = ws_rx.lock().await;
	let mut term_signal = rivet_runtime::TermSignal::get();

	// Per-actor channel routing for concurrent cross-actor request processing.
	// Each actor gets its own mpsc channel and a spawned task that drains it
	// sequentially, preserving intra-actor ordering while allowing inter-actor
	// parallelism. Do not use tokio::spawn per request as that would break
	// optimistic pipelining and journal write ordering.
	// See docs-internal/engine/NATIVE_SQLITE_REVIEW_FINDINGS.md Finding 2.
	let mut actor_channels: HashMap<String, mpsc::Sender<protocol::ToServerRequest>> =
		HashMap::new();
	let mut actor_tasks = tokio::task::JoinSet::new();

	// Use an async block so that early returns (via ?) still run cleanup below.
	let result = async {
		loop {
			let msg = tokio::select! {
				res = ws_rx.try_next() => {
					match res? {
						Some(msg) => msg,
						None => {
							tracing::debug!("websocket closed");
							return Ok(());
						}
					}
				}
				_ = term_signal.recv() => {
					// Send ToClientClose before shutting down.
					let close_msg = protocol::ToClient::ToClientClose;
					let data = protocol::encode_to_client(&close_msg)?;
					let _ = ws_handle.send(Message::Binary(data.into())).await;
					return Ok(());
				}
			};

			match msg {
				Message::Binary(data) => {
					handle_binary_message(
						ctx,
						state,
						ws_handle,
						conn_id,
						namespace_id,
						open_actors,
						last_pong_ts,
						&data,
						&mut actor_channels,
						&mut actor_tasks,
					)
					.await?;
				}
				Message::Close(_) => {
					tracing::debug!("websocket close frame received");
					return Ok(());
				}
				_ => {}
			}
		}
	}
	.await;

	// Drop all senders to signal per-actor tasks to stop, then wait for them
	// to finish draining any in-flight requests.
	actor_channels.clear();
	while actor_tasks.join_next().await.is_some() {}

	result
}

async fn handle_binary_message(
	ctx: &StandaloneCtx,
	state: &Arc<KvChannelState>,
	ws_handle: &WebSocketHandle,
	conn_id: Uuid,
	namespace_id: Id,
	open_actors: &Arc<Mutex<HashSet<String>>>,
	last_pong_ts: &AtomicI64,
	data: &[u8],
	actor_channels: &mut HashMap<String, mpsc::Sender<protocol::ToServerRequest>>,
	actor_tasks: &mut tokio::task::JoinSet<()>,
) -> Result<()> {
	let msg = match protocol::decode_to_server(data) {
		Ok(msg) => msg,
		Err(err) => {
			tracing::warn!(
				?err,
				data_len = data.len(),
				"failed to deserialize kv channel message"
			);
			return Ok(());
		}
	};

	match msg {
		protocol::ToServer::ToServerPong(pong) => {
			last_pong_ts.store(util::timestamp::now(), Ordering::Relaxed);
			tracing::trace!(ts = pong.ts, "received pong");
		}
		protocol::ToServer::ToServerRequest(req) => {
			let is_close = matches!(req.data, protocol::RequestData::ActorCloseRequest);
			let actor_id = req.actor_id.clone();
			let request_id = req.request_id;

			// Create a per-actor channel and task on first request for this actor.
			if !actor_channels.contains_key(&actor_id) {
				let (tx, rx) = mpsc::channel(64);
				actor_tasks.spawn(actor_request_task(
					Clone::clone(ctx),
					Clone::clone(state),
					Clone::clone(ws_handle),
					conn_id,
					namespace_id,
					Clone::clone(open_actors),
					rx,
				));
				actor_channels.insert(actor_id.clone(), tx);
			}

			// Route request to the actor's channel for sequential processing.
			if let Some(tx) = actor_channels.get(&actor_id) {
				match tx.try_send(req) {
					Ok(()) => {}
					Err(mpsc::error::TrySendError::Full(_)) => {
						tracing::warn!(%actor_id, "per-actor channel full, applying backpressure");
						send_response(
							ws_handle,
							request_id,
							error_response(
								"backpressure",
								"too many in-flight requests for this actor",
							),
						)
						.await;
					}
					Err(mpsc::error::TrySendError::Closed(_)) => {
						tracing::warn!(%actor_id, "per-actor task channel closed, removing dead entry");
						actor_channels.remove(&actor_id);
						send_response(
							ws_handle,
							request_id,
							error_response(
								"internal_error",
								"internal error",
							),
						)
						.await;
					}
				}
			}

			// Remove the channel entry on close so the task exits after draining
			// remaining requests and resources are freed.
			if is_close {
				actor_channels.remove(&actor_id);
			}
		}
	}

	Ok(())
}

/// Processes requests for a single actor sequentially, preserving intra-actor
/// ordering. Spawned once per actor per connection. Exits when the sender is
/// dropped (connection end) or after processing an ActorCloseRequest.
async fn actor_request_task(
	ctx: StandaloneCtx,
	state: Arc<KvChannelState>,
	ws_handle: WebSocketHandle,
	conn_id: Uuid,
	namespace_id: Id,
	open_actors: Arc<Mutex<HashSet<String>>>,
	mut rx: mpsc::Receiver<protocol::ToServerRequest>,
) {
	// Cached actor resolution. Populated on first KV request, reused for all
	// subsequent requests. Actor name is immutable so this never goes stale.
	let mut cached_actor: Option<(Id, String)> = None;

	while let Some(req) = rx.recv().await {
		let is_close = matches!(req.data, protocol::RequestData::ActorCloseRequest);

		let response_data = match &req.data {
			// Open/close are lifecycle ops that don't need a resolved actor.
			protocol::RequestData::ActorOpenRequest
			| protocol::RequestData::ActorCloseRequest => {
				handle_request(&ctx, &state, conn_id, namespace_id, &open_actors, &req).await
			}
			// KV ops: resolve once, cache, reuse.
			_ => {
				let is_open = open_actors.lock().await.contains(&req.actor_id);
				if !is_open {
					let locks = state.actor_locks.lock().await;
					if locks.contains_key(&req.actor_id) {
						error_response(
							"actor_locked",
							"actor is locked by another connection",
						)
					} else {
						error_response(
							"actor_not_open",
							"actor is not opened on this connection",
						)
					}
				} else {
					// Lazy-resolve and cache.
					if cached_actor.is_none() {
						match resolve_actor(&ctx, &req.actor_id, namespace_id).await {
							Ok(v) => {
								cached_actor = Some(v);
							}
							Err(resp) => {
								// Don't cache failures. Next request will retry.
								send_response(&ws_handle, req.request_id, resp).await;
								if is_close {
									break;
								}
								continue;
							}
						}
					}
					let (parsed_id, actor_name) = cached_actor.as_ref().unwrap();

					let recipient = actor_kv::Recipient {
						actor_id: *parsed_id,
						namespace_id,
						name: actor_name.clone(),
					};

					match &req.data {
						protocol::RequestData::KvGetRequest(body) => {
							handle_kv_get(&ctx, &recipient, body).await
						}
						protocol::RequestData::KvPutRequest(body) => {
							handle_kv_put(&ctx, &recipient, body).await
						}
						protocol::RequestData::KvDeleteRequest(body) => {
							handle_kv_delete(&ctx, &recipient, body).await
						}
						protocol::RequestData::KvDeleteRangeRequest(body) => {
							handle_kv_delete_range(&ctx, &recipient, body).await
						}
						_ => unreachable!(),
					}
				}
			}
		};

		send_response(&ws_handle, req.request_id, response_data).await;

		// Stop processing after a close request. The sender is also removed
		// from actor_channels by the message loop so no new requests arrive.
		if is_close {
			break;
		}
	}
}

/// Encode and send a response to the client. Logs warnings on failure.
async fn send_response(
	ws_handle: &WebSocketHandle,
	request_id: u32,
	data: protocol::ResponseData,
) {
	let response = protocol::ToClient::ToClientResponse(protocol::ToClientResponse {
		request_id,
		data,
	});

	match protocol::encode_to_client(&response) {
		Ok(encoded) => {
			if let Err(err) = ws_handle.send(Message::Binary(encoded.into())).await {
				tracing::warn!(?err, "failed to send kv channel response from actor task");
			}
		}
		Err(err) => {
			tracing::warn!(?err, "failed to encode kv channel response");
		}
	}
}

// MARK: Request handling

/// Handles actor lifecycle requests (open/close). KV operations are handled
/// directly in `actor_request_task` with cached actor resolution.
async fn handle_request(
	_ctx: &StandaloneCtx,
	state: &KvChannelState,
	conn_id: Uuid,
	_namespace_id: Id,
	open_actors: &Arc<Mutex<HashSet<String>>>,
	req: &protocol::ToServerRequest,
) -> protocol::ResponseData {
	match &req.data {
		protocol::RequestData::ActorOpenRequest => {
			handle_actor_open(state, conn_id, open_actors, &req.actor_id).await
		}
		protocol::RequestData::ActorCloseRequest => {
			handle_actor_close(state, conn_id, open_actors, &req.actor_id).await
		}
		_ => unreachable!("KV operations are handled in actor_request_task"),
	}
}

// MARK: Actor open/close

async fn handle_actor_open(
	state: &KvChannelState,
	conn_id: Uuid,
	open_actors: &Arc<Mutex<HashSet<String>>>,
	actor_id: &str,
) -> protocol::ResponseData {
	// Reject if this connection already has too many actors open.
	{
		let current_count = open_actors.lock().await.len();
		if current_count >= MAX_ACTORS_PER_CONNECTION {
			return error_response(
				"too_many_actors",
				&format!(
					"connection has too many open actors (max {MAX_ACTORS_PER_CONNECTION})"
				),
			);
		}
	}

	let mut locks = state.actor_locks.lock().await;

	// If the actor is locked by a different connection, unconditionally evict the old lock.
	// This handles reconnection scenarios where the server hasn't detected the old connection's
	// disconnect yet. The old connection's next KV request will fail the fast-path check
	// (open_actors.contains) and return actor_not_open.
	// See docs-internal/engine/NATIVE_SQLITE_REVIEW_FINDINGS.md Finding 4.
	if let Some((existing_conn, old_open_actors)) = locks.get(actor_id) {
		if *existing_conn != conn_id {
			old_open_actors.lock().await.remove(actor_id);
			tracing::info!(
				%conn_id,
				old_conn_id = %existing_conn,
				%actor_id,
				"evicted stale actor lock from old connection"
			);
		}
	}

	locks.insert(actor_id.to_string(), (conn_id, open_actors.clone()));
	open_actors.lock().await.insert(actor_id.to_string());
	tracing::debug!(%conn_id, %actor_id, "actor lock acquired");
	protocol::ResponseData::ActorOpenResponse
}

async fn handle_actor_close(
	state: &KvChannelState,
	conn_id: Uuid,
	open_actors: &Arc<Mutex<HashSet<String>>>,
	actor_id: &str,
) -> protocol::ResponseData {
	let mut locks = state.actor_locks.lock().await;

	if let Some((lock_conn, _)) = locks.get(actor_id) {
		if *lock_conn == conn_id {
			locks.remove(actor_id);
			open_actors.lock().await.remove(actor_id);
			tracing::debug!(%conn_id, %actor_id, "actor lock released");
		}
	}

	protocol::ResponseData::ActorCloseResponse
}

// MARK: KV operations

async fn handle_kv_get(
	ctx: &StandaloneCtx,
	recipient: &actor_kv::Recipient,
	body: &protocol::KvGetRequest,
) -> protocol::ResponseData {
	let start = Instant::now();
	metrics::KV_CHANNEL_REQUESTS_TOTAL.with_label_values(&["get"]).inc();
	metrics::KV_CHANNEL_REQUEST_KEYS.with_label_values(&["get"]).observe(body.keys.len() as f64);

	if let Err(resp) = validate_keys(&body.keys) {
		return resp;
	}

	let udb = match ctx.udb() {
		Ok(udb) => udb,
		Err(err) => return internal_error(&err),
	};

	let result = match actor_kv::get(&*udb, recipient, body.keys.clone()).await {
		Ok((keys, values, _metadata)) => {
			protocol::ResponseData::KvGetResponse(protocol::KvGetResponse { keys, values })
		}
		Err(err) => internal_error(&err),
	};
	metrics::KV_CHANNEL_REQUEST_DURATION.with_label_values(&["get"]).observe(start.elapsed().as_secs_f64());
	result
}

async fn handle_kv_put(
	ctx: &StandaloneCtx,
	recipient: &actor_kv::Recipient,
	body: &protocol::KvPutRequest,
) -> protocol::ResponseData {
	let start = Instant::now();
	metrics::KV_CHANNEL_REQUESTS_TOTAL.with_label_values(&["put"]).inc();
	metrics::KV_CHANNEL_REQUEST_KEYS.with_label_values(&["put"]).observe(body.keys.len() as f64);

	// Validate keys/values length match.
	if body.keys.len() != body.values.len() {
		return error_response(
			"keys_values_length_mismatch",
			"keys and values must have the same length",
		);
	}

	// Validate batch size.
	if body.keys.len() > MAX_KEYS {
		return error_response(
			"batch_too_large",
			&format!("a maximum of {MAX_KEYS} entries is allowed"),
		);
	}

	for key in &body.keys {
		if key.len() + KEY_WRAPPER_OVERHEAD > MAX_KEY_SIZE {
			return error_response(
				"key_too_large",
				&format!("key is too long (max {} bytes)", MAX_KEY_SIZE - KEY_WRAPPER_OVERHEAD),
			);
		}
	}
	for value in &body.values {
		if value.len() > MAX_VALUE_SIZE {
			return error_response(
				"value_too_large",
				&format!("value is too large (max {} KiB)", MAX_VALUE_SIZE / 1024),
			);
		}
	}

	let payload_size: usize = body.keys.iter().map(|k| k.len() + KEY_WRAPPER_OVERHEAD).sum::<usize>()
		+ body.values.iter().map(|v| v.len()).sum::<usize>();
	if payload_size > MAX_PUT_PAYLOAD_SIZE {
		return error_response(
			"payload_too_large",
			&format!(
				"total payload is too large (max {} KiB)",
				MAX_PUT_PAYLOAD_SIZE / 1024
			),
		);
	}

	let udb = match ctx.udb() {
		Ok(udb) => udb,
		Err(err) => return internal_error(&err),
	};

	let result = match actor_kv::put(&*udb, recipient, body.keys.clone(), body.values.clone()).await {
		Ok(()) => protocol::ResponseData::KvPutResponse,
		Err(err) => {
			let rivet_err = rivet_error::RivetError::extract(&err);
			if rivet_err.code() == "kv_storage_quota_exceeded" {
				error_response("storage_quota_exceeded", rivet_err.message())
			} else {
				internal_error(&err)
			}
		}
	};
	metrics::KV_CHANNEL_REQUEST_DURATION.with_label_values(&["put"]).observe(start.elapsed().as_secs_f64());
	result
}

async fn handle_kv_delete(
	ctx: &StandaloneCtx,
	recipient: &actor_kv::Recipient,
	body: &protocol::KvDeleteRequest,
) -> protocol::ResponseData {
	let start = Instant::now();
	metrics::KV_CHANNEL_REQUESTS_TOTAL.with_label_values(&["delete"]).inc();
	metrics::KV_CHANNEL_REQUEST_KEYS.with_label_values(&["delete"]).observe(body.keys.len() as f64);

	if let Err(resp) = validate_keys(&body.keys) {
		return resp;
	}

	let udb = match ctx.udb() {
		Ok(udb) => udb,
		Err(err) => return internal_error(&err),
	};

	let result = match actor_kv::delete(&*udb, recipient, body.keys.clone()).await {
		Ok(()) => protocol::ResponseData::KvDeleteResponse,
		Err(err) => internal_error(&err),
	};
	metrics::KV_CHANNEL_REQUEST_DURATION.with_label_values(&["delete"]).observe(start.elapsed().as_secs_f64());
	result
}

async fn handle_kv_delete_range(
	ctx: &StandaloneCtx,
	recipient: &actor_kv::Recipient,
	body: &protocol::KvDeleteRangeRequest,
) -> protocol::ResponseData {
	let start = Instant::now();
	metrics::KV_CHANNEL_REQUESTS_TOTAL.with_label_values(&["delete_range"]).inc();
	if body.start.len() + KEY_WRAPPER_OVERHEAD > MAX_KEY_SIZE {
		return error_response(
			"key_too_large",
			&format!("start key is too long (max {} bytes)", MAX_KEY_SIZE - KEY_WRAPPER_OVERHEAD),
		);
	}
	if body.end.len() + KEY_WRAPPER_OVERHEAD > MAX_KEY_SIZE {
		return error_response(
			"key_too_large",
			&format!("end key is too long (max {} bytes)", MAX_KEY_SIZE - KEY_WRAPPER_OVERHEAD),
		);
	}

	let udb = match ctx.udb() {
		Ok(udb) => udb,
		Err(err) => return internal_error(&err),
	};

	let result = match actor_kv::delete_range(&*udb, recipient, body.start.clone(), body.end.clone()).await {
		Ok(()) => protocol::ResponseData::KvDeleteResponse,
		Err(err) => internal_error(&err),
	};
	metrics::KV_CHANNEL_REQUEST_DURATION.with_label_values(&["delete_range"]).observe(start.elapsed().as_secs_f64());
	result
}

// MARK: Helpers

/// Look up an actor by ID and return the parsed ID and actor name.
///
/// Defense-in-depth: verifies the actor belongs to the authenticated namespace.
/// The admin_token is a global credential, so this is not strictly necessary
/// today, but prevents cross-namespace access if a less-privileged auth
/// mechanism is introduced in the future.
async fn resolve_actor(
	ctx: &StandaloneCtx,
	actor_id: &str,
	expected_namespace_id: Id,
) -> std::result::Result<(Id, String), protocol::ResponseData> {
	let parsed_id = Id::parse(actor_id).map_err(|err| {
		error_response(
			"actor_not_found",
			&format!("invalid actor id: {err}"),
		)
	})?;

	let actor = ctx
		.op(pegboard::ops::actor::get_for_runner::Input {
			actor_id: parsed_id,
		})
		.await
		.map_err(|err| internal_error(&err))?;

	match actor {
		Some(actor) => {
			if actor.namespace_id != expected_namespace_id {
				return Err(error_response(
					"actor_not_found",
					"actor does not exist or is not running",
				));
			}
			Ok((parsed_id, actor.name))
		}
		None => Err(error_response(
			"actor_not_found",
			"actor does not exist or is not running",
		)),
	}
}

/// Validate a list of KV keys against size and count limits.
fn validate_keys(keys: &[protocol::KvKey]) -> std::result::Result<(), protocol::ResponseData> {
	if keys.len() > MAX_KEYS {
		return Err(error_response(
			"batch_too_large",
			&format!("a maximum of {MAX_KEYS} keys is allowed"),
		));
	}
	for key in keys {
		if key.len() + KEY_WRAPPER_OVERHEAD > MAX_KEY_SIZE {
			return Err(error_response(
				"key_too_large",
				&format!("key is too long (max {} bytes)", MAX_KEY_SIZE - KEY_WRAPPER_OVERHEAD),
			));
		}
	}
	Ok(())
}

fn error_response(code: &str, message: &str) -> protocol::ResponseData {
	protocol::ResponseData::ErrorResponse(protocol::ErrorResponse {
		code: code.to_string(),
		message: message.to_string(),
	})
}

/// Log an internal error with full details server-side and return a generic
/// error message to the client. Prevents leaking stack traces, database errors,
/// or other internal state over the wire.
fn internal_error(err: &anyhow::Error) -> protocol::ResponseData {
	tracing::error!(?err, "kv channel internal error");
	error_response("internal_error", "internal error")
}
