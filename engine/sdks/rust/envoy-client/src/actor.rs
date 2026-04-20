use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use rivet_envoy_protocol as protocol;
use rivet_util::async_counter::AsyncCounter;
use rivet_util_serde::HashableMap;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::TryRecvError;
use tokio::task::{JoinError, JoinSet};

use crate::config::{HttpRequest, HttpResponse, WebSocketMessage};
use crate::connection::ws_send;
use crate::context::SharedContext;
use crate::handle::EnvoyHandle;
use crate::stringify::stringify_to_rivet_tunnel_message_kind;
use crate::utils::{BufferMap, id_to_str, wrapping_add_u16, wrapping_lte_u16, wrapping_sub_u16};

pub enum ToActor {
	Intent {
		intent: protocol::ActorIntent,
		error: Option<String>,
	},
	Stop {
		command_idx: i64,
		reason: protocol::StopActorReason,
	},
	Lost,
	SetAlarm {
		alarm_ts: Option<i64>,
	},
	ReqStart {
		message_id: protocol::MessageId,
		req: protocol::ToEnvoyRequestStart,
	},
	ReqChunk {
		message_id: protocol::MessageId,
		chunk: protocol::ToEnvoyRequestChunk,
	},
	ReqAbort {
		message_id: protocol::MessageId,
	},
	WsOpen {
		message_id: protocol::MessageId,
		path: String,
		headers: BTreeMap<String, String>,
	},
	WsMsg {
		message_id: protocol::MessageId,
		msg: protocol::ToEnvoyWebSocketMessage,
	},
	WsClose {
		message_id: protocol::MessageId,
		close: protocol::ToEnvoyWebSocketClose,
	},
	HwsAck {
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
		envoy_message_index: u16,
	},
}

struct PendingRequest {
	envoy_message_index: u16,
	body_tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
}

struct WsEntry {
	is_hibernatable: bool,
	rivet_message_index: u16,
	ws_handler: Option<crate::config::WebSocketHandler>,
	outgoing_tx: mpsc::UnboundedSender<crate::config::WsOutgoing>,
}

struct ActorContext {
	shared: Arc<SharedContext>,
	actor_id: String,
	generation: u32,
	command_idx: i64,
	event_index: i64,
	error: Option<String>,
	pending_requests: BufferMap<PendingRequest>,
	ws_entries: BufferMap<WsEntry>,
	hibernating_requests: Vec<protocol::HibernatingRequest>,
	active_http_request_count: Arc<AsyncCounter>,
}

struct ActiveHttpRequestGuard {
	active_http_request_count: Arc<AsyncCounter>,
}

struct PendingStop {
	completion_rx: oneshot::Receiver<anyhow::Result<()>>,
	stop_code: protocol::StopCode,
	stop_message: Option<String>,
}

enum StopProgress {
	Stopped,
	Pending(PendingStop),
}

impl ActiveHttpRequestGuard {
	fn new(active_http_request_count: Arc<AsyncCounter>) -> Self {
		active_http_request_count.increment();
		Self {
			active_http_request_count,
		}
	}
}

impl Drop for ActiveHttpRequestGuard {
	fn drop(&mut self) {
		self.active_http_request_count.decrement();
	}
}

pub fn create_actor(
	shared: Arc<SharedContext>,
	actor_id: String,
	generation: u32,
	config: protocol::ActorConfig,
	hibernating_requests: Vec<protocol::HibernatingRequest>,
	preloaded_kv: Option<protocol::PreloadedKv>,
	sqlite_schema_version: u32,
	sqlite_startup_data: Option<protocol::SqliteStartupData>,
) -> (mpsc::UnboundedSender<ToActor>, Arc<AsyncCounter>) {
	let (tx, rx) = mpsc::unbounded_channel();
	let active_http_request_count = Arc::new(AsyncCounter::new());
	tokio::spawn(actor_inner(
		shared,
		actor_id,
		generation,
		config,
		hibernating_requests,
		preloaded_kv,
		sqlite_schema_version,
		sqlite_startup_data,
		rx,
		active_http_request_count.clone(),
	));
	(tx, active_http_request_count)
}

async fn actor_inner(
	shared: Arc<SharedContext>,
	actor_id: String,
	generation: u32,
	config: protocol::ActorConfig,
	hibernating_requests: Vec<protocol::HibernatingRequest>,
	preloaded_kv: Option<protocol::PreloadedKv>,
	sqlite_schema_version: u32,
	sqlite_startup_data: Option<protocol::SqliteStartupData>,
	mut rx: mpsc::UnboundedReceiver<ToActor>,
	active_http_request_count: Arc<AsyncCounter>,
) {
	let handle = EnvoyHandle {
		shared: shared.clone(),
		// Fake channel, don't care
		started_rx: tokio::sync::watch::channel(()).1,
	};

	let mut ctx = ActorContext {
		shared: shared.clone(),
		actor_id: actor_id.clone(),
		generation,
		command_idx: 0,
		event_index: 0,
		error: None,
		pending_requests: BufferMap::new(),
		ws_entries: BufferMap::new(),
		hibernating_requests,
		active_http_request_count,
	};
	let mut http_request_tasks = JoinSet::new();
	let mut pending_stop: Option<PendingStop> = None;
	let mut rx_closed = false;

	// Call on_actor_start
	let start_result = shared
		.config
		.callbacks
		.on_actor_start(
			handle.clone(),
			actor_id.clone(),
			generation,
			config,
			preloaded_kv,
			sqlite_schema_version,
			sqlite_startup_data,
		)
		.await;

	if let Err(error) = start_result {
		tracing::error!(actor_id = %ctx.actor_id, ?error, "actor start failed");
		send_event(
			&mut ctx,
			protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
				state: protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
					code: protocol::StopCode::Error,
					message: Some(format!("{error:#}")),
				}),
			}),
		);
		return;
	}

	if let Some(meta_entries) = handle.take_pending_hibernation_restore(&actor_id) {
		if let Err(error) = handle_hws_restore(&mut ctx, &handle, meta_entries).await {
			tracing::error!(actor_id = %ctx.actor_id, ?error, "actor hibernation restore failed");
			send_event(
				&mut ctx,
				protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
					state: protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
						code: protocol::StopCode::Error,
						message: Some(format!("{error:#}")),
					}),
				}),
			);
			return;
		}
	}

	// Send running state
	send_event(
		&mut ctx,
		protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
			state: protocol::ActorState::ActorStateRunning,
		}),
	);

	loop {
		tokio::select! {
			maybe_task = async {
				if http_request_tasks.is_empty() {
					std::future::pending().await
				} else {
					http_request_tasks.join_next().await
				}
			} => {
				if let Some(result) = maybe_task {
					handle_http_request_task_result(&ctx, result);
				}
			}
			msg = async {
				if rx_closed {
					std::future::pending::<Option<ToActor>>().await
				} else {
					rx.recv().await
				}
			} => {
				let Some(msg) = msg else {
					if pending_stop.is_some() {
						rx_closed = true;
						continue;
					}
					break;
				};

				match msg {
					ToActor::Intent { intent, error } => {
						send_event(
							&mut ctx,
							protocol::Event::EventActorIntent(protocol::EventActorIntent { intent }),
						);
						if error.is_some() {
							ctx.error = error;
						}
					}
					ToActor::Stop {
						command_idx,
						reason,
					} => {
						if pending_stop.is_some() {
							tracing::warn!(
								actor_id = %ctx.actor_id,
								command_idx,
								"ignoring duplicate stop while actor teardown is in progress"
							);
							continue;
						}
						if command_idx <= ctx.command_idx {
							tracing::warn!(command_idx, "ignoring already seen command");
							continue;
						}
						ctx.command_idx = command_idx;
						match begin_stop(&mut ctx, &handle, &mut http_request_tasks, reason).await {
							StopProgress::Stopped => break,
							StopProgress::Pending(stop) => pending_stop = Some(stop),
						}
					}
					ToActor::Lost => {
						if pending_stop.is_some() {
							tracing::warn!(
								actor_id = %ctx.actor_id,
								"ignoring lost signal while actor teardown is in progress"
							);
							continue;
						}
						match begin_stop(
							&mut ctx,
							&handle,
							&mut http_request_tasks,
							protocol::StopActorReason::Lost,
						)
						.await
						{
							StopProgress::Stopped => break,
							StopProgress::Pending(stop) => pending_stop = Some(stop),
						}
					}
					ToActor::SetAlarm { alarm_ts } => {
						send_event(
							&mut ctx,
							protocol::Event::EventActorSetAlarm(protocol::EventActorSetAlarm { alarm_ts }),
						);
					}
					ToActor::ReqStart { message_id, req } => {
						handle_req_start(&mut ctx, &handle, &mut http_request_tasks, message_id, req);
					}
					ToActor::ReqChunk { message_id, chunk } => {
						handle_req_chunk(&mut ctx, message_id, chunk);
					}
					ToActor::ReqAbort { message_id } => {
						handle_req_abort(&mut ctx, message_id);
					}
					ToActor::WsOpen {
						message_id,
						path,
						headers,
					} => {
						handle_ws_open(&mut ctx, &handle, message_id, path, headers).await;
					}
					ToActor::WsMsg { message_id, msg } => {
						handle_ws_message(&mut ctx, message_id, msg).await;
					}
					ToActor::WsClose { message_id, close } => {
						handle_ws_close(&mut ctx, message_id, close).await;
					}
					ToActor::HwsAck {
						gateway_id,
						request_id,
						envoy_message_index,
					} => {
						handle_hws_ack(&mut ctx, gateway_id, request_id, envoy_message_index).await;
					}
				}
			}
			stop_result = async {
				let pending = pending_stop
					.as_mut()
					.expect("pending stop must exist when waiting for stop completion");
				(&mut pending.completion_rx).await
			}, if pending_stop.is_some() => {
				let pending = pending_stop
					.take()
					.expect("pending stop must exist when stop completion resolves");
				abort_and_join_http_request_tasks(&mut ctx, &mut http_request_tasks).await;
				finalize_stop(&mut ctx, pending, stop_result);
				break;
			}
		}
	}

	abort_and_join_http_request_tasks(&mut ctx, &mut http_request_tasks).await;
	tracing::debug!(actor_id = %ctx.actor_id, "envoy actor stopped");
}

fn send_event(ctx: &mut ActorContext, inner: protocol::Event) {
	let checkpoint = increment_checkpoint(ctx);
	let _ = ctx
		.shared
		.envoy_tx
		.send(crate::envoy::ToEnvoyMessage::SendEvents {
			events: vec![protocol::EventWrapper { checkpoint, inner }],
		});
}

async fn begin_stop(
	ctx: &mut ActorContext,
	handle: &EnvoyHandle,
	_http_request_tasks: &mut JoinSet<()>,
	reason: protocol::StopActorReason,
) -> StopProgress {
	let mut stop_code = if ctx.error.is_some() {
		protocol::StopCode::Error
	} else {
		protocol::StopCode::Ok
	};
	let mut stop_message = ctx.error.clone();
	let (stop_tx, mut stop_rx) = oneshot::channel();

	let stop_result = ctx
		.shared
		.config
		.callbacks
		.on_actor_stop_with_completion(
			handle.clone(),
			ctx.actor_id.clone(),
			ctx.generation,
			reason,
			crate::config::ActorStopHandle::new(stop_tx),
		)
		.await;

	if let Err(error) = stop_result {
		tracing::error!(actor_id = %ctx.actor_id, ?error, "actor stop failed");
		stop_code = protocol::StopCode::Error;
		if stop_message.is_none() {
			stop_message = Some(format!("{error:#}"));
		}
		send_stopped_event(ctx, stop_code, stop_message);
		return StopProgress::Stopped;
	}

	match stop_rx.try_recv() {
		Ok(stop_result) => {
			send_stopped_event_for_result(ctx, stop_code, stop_message, stop_result);
			StopProgress::Stopped
		}
		Err(TryRecvError::Empty) => StopProgress::Pending(PendingStop {
			completion_rx: stop_rx,
			stop_code,
			stop_message,
		}),
		Err(TryRecvError::Closed) => {
			send_stopped_event(ctx, stop_code, stop_message);
			StopProgress::Stopped
		}
	}
}

fn finalize_stop(
	ctx: &mut ActorContext,
	pending: PendingStop,
	stop_result: Result<anyhow::Result<()>, oneshot::error::RecvError>,
) {
	match stop_result {
		Ok(stop_result) => {
			send_stopped_event_for_result(ctx, pending.stop_code, pending.stop_message, stop_result);
		}
		Err(error) => {
			tracing::warn!(
				actor_id = %ctx.actor_id,
				?error,
				"actor stop completion handle dropped before signaling teardown result"
			);
			send_stopped_event(ctx, pending.stop_code, pending.stop_message);
		}
	}
}

fn send_stopped_event_for_result(
	ctx: &mut ActorContext,
	mut stop_code: protocol::StopCode,
	mut stop_message: Option<String>,
	stop_result: anyhow::Result<()>,
) {
	if let Err(error) = stop_result {
		tracing::error!(actor_id = %ctx.actor_id, ?error, "actor stop completion failed");
		stop_code = protocol::StopCode::Error;
		if stop_message.is_none() {
			stop_message = Some(format!("{error:#}"));
		}
	}

	send_stopped_event(ctx, stop_code, stop_message);
}

fn send_stopped_event(
	ctx: &mut ActorContext,
	stop_code: protocol::StopCode,
	stop_message: Option<String>,
) {
	send_event(
		ctx,
		protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
			state: protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
				code: stop_code,
				message: stop_message,
			}),
		}),
	);
}

fn handle_req_start(
	ctx: &mut ActorContext,
	handle: &EnvoyHandle,
	http_request_tasks: &mut JoinSet<()>,
	message_id: protocol::MessageId,
	req: protocol::ToEnvoyRequestStart,
) {
	let pending = PendingRequest {
		envoy_message_index: 0,
		body_tx: None,
	};
	ctx.pending_requests
		.insert(&[&message_id.gateway_id, &message_id.request_id], pending);

	let headers: HashMap<String, String> = req
		.headers
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();

	let body_stream = if req.stream {
		let (body_tx, body_rx) = mpsc::unbounded_channel::<Vec<u8>>();
		if let Some(pending) = ctx
			.pending_requests
			.get_mut(&[&message_id.gateway_id, &message_id.request_id])
		{
			pending.body_tx = Some(body_tx);
		}
		Some(body_rx)
	} else {
		None
	};

	let request = HttpRequest {
		method: req.method,
		path: req.path,
		headers,
		body: req.body,
		body_stream,
	};

	let shared = ctx.shared.clone();
	let handle_clone = handle.clone();
	let actor_id = ctx.actor_id.clone();
	let gateway_id = message_id.gateway_id;
	let request_id = message_id.request_id;
	let request_guard = ActiveHttpRequestGuard::new(ctx.active_http_request_count.clone());

	http_request_tasks.spawn(async move {
		let _request_guard = request_guard;
		let response = shared
			.config
			.callbacks
			.fetch(handle_clone, actor_id, gateway_id, request_id, request)
			.await;

		match response {
			Ok(response) => {
				send_response(&shared, gateway_id, request_id, response).await;
			}
			Err(error) => {
				tracing::error!(?error, "fetch failed");
			}
		}
	});

	if !req.stream {
		ctx.pending_requests
			.remove(&[&message_id.gateway_id, &message_id.request_id]);
	}
}

fn handle_http_request_task_result(ctx: &ActorContext, result: Result<(), JoinError>) {
	if let Err(error) = result {
		if error.is_cancelled() {
			return;
		}

		tracing::error!(actor_id = %ctx.actor_id, ?error, "http request task failed");
	}
}

async fn abort_and_join_http_request_tasks(
	ctx: &mut ActorContext,
	http_request_tasks: &mut JoinSet<()>,
) {
	if http_request_tasks.is_empty() {
		return;
	}

	let active_http_request_count = ctx.active_http_request_count.load();
	tracing::debug!(
		actor_id = %ctx.actor_id,
		active_http_request_count,
		"aborting in-flight http request tasks"
	);

	http_request_tasks.abort_all();

	while let Some(result) = http_request_tasks.join_next().await {
		handle_http_request_task_result(ctx, result);
	}
}

fn handle_req_chunk(
	ctx: &mut ActorContext,
	message_id: protocol::MessageId,
	chunk: protocol::ToEnvoyRequestChunk,
) {
	let finish = chunk.finish;
	let pending = ctx
		.pending_requests
		.get(&[&message_id.gateway_id, &message_id.request_id]);
	if let Some(pending) = pending {
		if let Some(body_tx) = &pending.body_tx {
			let _ = body_tx.send(chunk.body);
		} else {
			tracing::warn!("received chunk for pending request without stream controller");
		}
	} else {
		tracing::warn!("received chunk for unknown pending request");
	}

	if finish {
		ctx.pending_requests
			.remove(&[&message_id.gateway_id, &message_id.request_id]);
	}
}

fn handle_req_abort(ctx: &mut ActorContext, message_id: protocol::MessageId) {
	ctx.pending_requests
		.remove(&[&message_id.gateway_id, &message_id.request_id]);
}

fn spawn_ws_outgoing_task(
	shared: Arc<SharedContext>,
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
	mut outgoing_rx: mpsc::UnboundedReceiver<crate::config::WsOutgoing>,
) {
	tokio::spawn(async move {
		let mut idx: u16 = 0;
		while let Some(msg) = outgoing_rx.recv().await {
			idx += 1;
			match msg {
				crate::config::WsOutgoing::Message { data, binary } => {
					ws_send(
						&shared,
						protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
							message_id: protocol::MessageId {
								gateway_id,
								request_id,
								message_index: idx,
							},
							message_kind: protocol::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
								protocol::ToRivetWebSocketMessage { data, binary },
							),
						}),
					)
					.await;
				}
				crate::config::WsOutgoing::Close { code, reason } => {
					ws_send(
						&shared,
						protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
							message_id: protocol::MessageId {
								gateway_id,
								request_id,
								message_index: 0,
							},
							message_kind: protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
								protocol::ToRivetWebSocketClose {
									code,
									reason,
									hibernate: false,
								},
							),
						}),
					)
					.await;
					break;
				}
			}
		}
	});
}

async fn handle_ws_open(
	ctx: &mut ActorContext,
	handle: &EnvoyHandle,
	message_id: protocol::MessageId,
	path: String,
	headers: BTreeMap<String, String>,
) {
	let restored_ws = ctx
		.ws_entries
		.remove(&[&message_id.gateway_id, &message_id.request_id]);
	let is_restoring_hibernatable = restored_ws
		.as_ref()
		.map(|ws| ws.is_hibernatable)
		.unwrap_or(false);

	if !is_restoring_hibernatable {
		ctx.pending_requests.insert(
			&[&message_id.gateway_id, &message_id.request_id],
			PendingRequest {
				envoy_message_index: 0,
				body_tx: None,
			},
		);
	}

	let mut full_headers: HashMap<String, String> = headers.into_iter().collect();
	full_headers.insert("Upgrade".to_string(), "websocket".to_string());
	full_headers.insert("Connection".to_string(), "Upgrade".to_string());

	let request = HttpRequest {
		method: "GET".to_string(),
		path: path.clone(),
		headers: full_headers.clone(),
		body: None,
		body_stream: None,
	};

	let is_hibernatable = if is_restoring_hibernatable {
		true
	} else {
		match ctx
			.shared
			.config
			.callbacks
			.can_hibernate(
				&ctx.actor_id,
				&message_id.gateway_id,
				&message_id.request_id,
				&request,
			)
			.await
		{
			Ok(is_hibernatable) => is_hibernatable,
			Err(error) => {
				tracing::error!(?error, "error checking websocket hibernation");

				send_actor_message(
					ctx,
					message_id.gateway_id,
					message_id.request_id,
					protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
						protocol::ToRivetWebSocketClose {
							code: Some(1011),
							reason: Some("Server Error".to_string()),
							hibernate: false,
						},
					),
				)
				.await;

				ctx.pending_requests
					.remove(&[&message_id.gateway_id, &message_id.request_id]);
				return;
			}
		}
	};

	// Create outgoing channel BEFORE calling websocket() so the sender is available immediately
	let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel::<crate::config::WsOutgoing>();
	let sender = crate::config::WebSocketSender {
		tx: outgoing_tx.clone(),
	};

	let ws_result = if let Some(mut restored_ws) = restored_ws {
		match restored_ws.ws_handler.take() {
			Some(ws_handler) => Ok(ws_handler),
			None => Err(anyhow!(
				"missing websocket handler for restored hibernatable websocket"
			)),
		}
	} else {
		ctx
			.shared
			.config
			.callbacks
			.websocket(
				handle.clone(),
				ctx.actor_id.clone(),
				message_id.gateway_id,
				message_id.request_id,
				request,
				path,
				full_headers,
				is_hibernatable,
				false,
				sender,
			)
			.await
	};

	match ws_result {
		Ok(ws_handler) => {
			ctx.ws_entries.insert(
				&[&message_id.gateway_id, &message_id.request_id],
				WsEntry {
					is_hibernatable,
					rivet_message_index: message_id.message_index,
					ws_handler: Some(ws_handler),
					outgoing_tx,
				},
			);

			spawn_ws_outgoing_task(
				ctx.shared.clone(),
				message_id.gateway_id,
				message_id.request_id,
				outgoing_rx,
			);

			// Gateway wake flows still wait for a websocket-open ack before they
			// resume forwarding buffered client messages, even when the request is
			// being restored after actor hibernation.
			send_actor_message(
				ctx,
				message_id.gateway_id,
				message_id.request_id,
				protocol::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
					protocol::ToRivetWebSocketOpen {
						can_hibernate: is_hibernatable,
					},
				),
			)
			.await;

			// Call on_open if provided
			if let Some(ws) = ctx
				.ws_entries
				.get_mut(&[&message_id.gateway_id, &message_id.request_id])
			{
				if let Some(handler) = &mut ws.ws_handler {
					if let Some(on_open) = handler.on_open.take() {
						let sender = crate::config::WebSocketSender {
							tx: ws.outgoing_tx.clone(),
						};

						on_open(sender).await;
					}
				}
			}
		}
		Err(error) => {
			tracing::error!(?error, "error handling websocket open");

			send_actor_message(
				ctx,
				message_id.gateway_id,
				message_id.request_id,
				protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
					protocol::ToRivetWebSocketClose {
						code: Some(1011),
						reason: Some("Server Error".to_string()),
						hibernate: false,
					},
				),
			)
			.await;

			ctx.pending_requests
				.remove(&[&message_id.gateway_id, &message_id.request_id]);
			ctx.ws_entries
				.remove(&[&message_id.gateway_id, &message_id.request_id]);
		}
	}
}

async fn handle_ws_message(
	ctx: &mut ActorContext,
	message_id: protocol::MessageId,
	msg: protocol::ToEnvoyWebSocketMessage,
) {
	let ws = ctx
		.ws_entries
		.get_mut(&[&message_id.gateway_id, &message_id.request_id]);

	if let Some(ws) = ws {
		// Validate message index for hibernatable websockets
		if ws.is_hibernatable {
			let previous_index = ws.rivet_message_index;
			let received_index = message_id.message_index;

			if wrapping_lte_u16(received_index, previous_index) {
				tracing::info!(
					request_id = id_to_str(&message_id.request_id),
					actor_id = %ctx.actor_id,
					previous_index,
					received_index,
					"received duplicate hibernating websocket message"
				);
				return;
			}

			let expected_index = wrapping_add_u16(previous_index, 1);
			if received_index != expected_index {
				tracing::warn!(
					request_id = id_to_str(&message_id.request_id),
					actor_id = %ctx.actor_id,
					previous_index,
					expected_index,
					received_index,
					gap = wrapping_sub_u16(wrapping_sub_u16(received_index, previous_index), 1),
					"hibernatable websocket message index out of sequence, closing connection"
				);

				send_actor_message(
					ctx,
					message_id.gateway_id,
					message_id.request_id,
					protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
						protocol::ToRivetWebSocketClose {
							code: Some(1008),
							reason: Some("ws.message_index_skip".to_string()),
							hibernate: false,
						},
					),
				)
				.await;
				return;
			}

			ws.rivet_message_index = received_index;
		}

		if let Some(handler) = &ws.ws_handler {
			let sender = crate::config::WebSocketSender {
				tx: ws.outgoing_tx.clone(),
			};
			let ws_msg = WebSocketMessage {
				data: msg.data,
				binary: msg.binary,
				gateway_id: message_id.gateway_id,
				request_id: message_id.request_id,
				message_index: message_id.message_index,
				sender,
			};
			(handler.on_message)(ws_msg).await;
		}
	} else {
		tracing::warn!("received message for unknown ws");
	}
}

async fn handle_ws_close(
	ctx: &mut ActorContext,
	message_id: protocol::MessageId,
	close: protocol::ToEnvoyWebSocketClose,
) {
	let ws = ctx
		.ws_entries
		.remove(&[&message_id.gateway_id, &message_id.request_id]);

	if let Some(ws) = ws {
		if let Some(handler) = &ws.ws_handler {
			let code = close.code.unwrap_or(1000);
			let reason = close.reason.unwrap_or_default();
			(handler.on_close)(code, reason).await;
		}
		ctx.pending_requests
			.remove(&[&message_id.gateway_id, &message_id.request_id]);
	} else {
		tracing::warn!("received close for unknown ws");
	}
}

async fn handle_hws_restore(
	ctx: &mut ActorContext,
	handle: &EnvoyHandle,
	meta_entries: Vec<crate::tunnel::HibernatingWebSocketMetadata>,
) -> anyhow::Result<()> {
	tracing::debug!(
		requests = ctx.hibernating_requests.len(),
		"restoring hibernating requests"
	);

	let hibernating_requests = std::mem::take(&mut ctx.hibernating_requests);

	for hib_req in &hibernating_requests {
		let meta = meta_entries.iter().find(|entry| {
			entry.gateway_id == hib_req.gateway_id && entry.request_id == hib_req.request_id
		});

		if let Some(meta) = meta {
			ctx.pending_requests.insert(
				&[&hib_req.gateway_id, &hib_req.request_id],
				PendingRequest {
					envoy_message_index: meta.envoy_message_index,
					body_tx: None,
				},
			);

			let mut full_headers = meta.headers.clone();
			full_headers.insert("Upgrade".to_string(), "websocket".to_string());
			full_headers.insert("Connection".to_string(), "Upgrade".to_string());

			let request = HttpRequest {
				method: "GET".to_string(),
				path: meta.path.clone(),
				headers: full_headers.clone(),
				body: None,
				body_stream: None,
			};

			let (hws_outgoing_tx, hws_outgoing_rx) = mpsc::unbounded_channel();
			let hws_sender = crate::config::WebSocketSender {
				tx: hws_outgoing_tx.clone(),
			};

			let ws_result = ctx
				.shared
				.config
				.callbacks
				.websocket(
					handle.clone(),
					ctx.actor_id.clone(),
					hib_req.gateway_id,
					hib_req.request_id,
					request,
					meta.path.clone(),
					full_headers,
					true,
					true,
					hws_sender,
				)
				.await;

			match ws_result {
				Ok(ws_handler) => {
					spawn_ws_outgoing_task(
						ctx.shared.clone(),
						hib_req.gateway_id,
						hib_req.request_id,
						hws_outgoing_rx,
					);
					ctx.ws_entries.insert(
						&[&hib_req.gateway_id, &hib_req.request_id],
						WsEntry {
							is_hibernatable: true,
							rivet_message_index: meta.rivet_message_index,
							ws_handler: Some(ws_handler),
							outgoing_tx: hws_outgoing_tx,
						},
					);
					// Gateway wake flows wait for the websocket-open ack before
					// they resume forwarding buffered client messages.
					send_actor_message(
						ctx,
						hib_req.gateway_id,
						hib_req.request_id,
						protocol::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
							protocol::ToRivetWebSocketOpen {
								can_hibernate: true,
							},
						),
					)
					.await;
					tracing::info!(
						request_id = id_to_str(&hib_req.request_id),
						"connection successfully restored"
					);
				}
				Err(error) => {
					tracing::error!(
						request_id = id_to_str(&hib_req.request_id),
						?error,
						"error creating websocket during restore"
					);

					send_actor_message(
						ctx,
						hib_req.gateway_id,
						hib_req.request_id,
						protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
							protocol::ToRivetWebSocketClose {
								code: Some(1011),
								reason: Some("ws.restore_error".to_string()),
								hibernate: false,
							},
						),
					)
					.await;

					ctx.pending_requests
						.remove(&[&hib_req.gateway_id, &hib_req.request_id]);
				}
			}
		} else {
			tracing::warn!(
				request_id = id_to_str(&hib_req.request_id),
				"closing websocket that is not persisted"
			);

			send_actor_message(
				ctx,
				hib_req.gateway_id,
				hib_req.request_id,
				protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
					protocol::ToRivetWebSocketClose {
						code: Some(1000),
						reason: Some("ws.meta_not_found_during_restore".to_string()),
						hibernate: false,
					},
				),
			)
			.await;
		}
	}

	// Process loaded but not connected (stale)
	for meta in &meta_entries {
		let is_connected = hibernating_requests
			.iter()
			.any(|req| req.gateway_id == meta.gateway_id && req.request_id == meta.request_id);

		if !is_connected {
			tracing::warn!(
				request_id = id_to_str(&meta.request_id),
				"removing stale persisted websocket"
			);

			let full_headers = meta.headers.clone();
			let request = HttpRequest {
				method: "GET".to_string(),
				path: meta.path.clone(),
				headers: full_headers.clone(),
				body: None,
				body_stream: None,
			};

			let (stale_tx, _) = mpsc::unbounded_channel();
			let stale_sender = crate::config::WebSocketSender { tx: stale_tx };

			let ws_result = ctx
				.shared
				.config
				.callbacks
				.websocket(
					handle.clone(),
					ctx.actor_id.clone(),
					meta.gateway_id,
					meta.request_id,
					request,
					meta.path.clone(),
					full_headers,
					true,
					true,
					stale_sender,
				)
				.await;

			if let Ok(handler) = ws_result {
				(handler.on_close)(1000, "ws.stale_metadata".to_string()).await;
			}
		}
	}

	ctx.hibernating_requests = hibernating_requests;
	tracing::info!("restored hibernatable websockets");
	Ok(())
}

async fn handle_hws_ack(
	ctx: &mut ActorContext,
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
	envoy_message_index: u16,
) {
	tracing::debug!(
		request_id = id_to_str(&request_id),
		index = envoy_message_index,
		"ack ws msg"
	);

	send_actor_message(
		ctx,
		gateway_id,
		request_id,
		protocol::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
			protocol::ToRivetWebSocketMessageAck {
				index: envoy_message_index,
			},
		),
	)
	.await;
}

fn increment_checkpoint(ctx: &mut ActorContext) -> protocol::ActorCheckpoint {
	let index = ctx.event_index;
	ctx.event_index += 1;
	protocol::ActorCheckpoint {
		actor_id: ctx.actor_id.clone(),
		generation: ctx.generation,
		index,
	}
}

async fn send_actor_message(
	ctx: &mut ActorContext,
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
	message_kind: protocol::ToRivetTunnelMessageKind,
) {
	let req = ctx.pending_requests.get_mut(&[&gateway_id, &request_id]);
	let envoy_message_index = if let Some(req) = req {
		let idx = req.envoy_message_index;
		req.envoy_message_index += 1;
		idx
	} else {
		tracing::warn!(
			gateway_id = id_to_str(&gateway_id),
			request_id = id_to_str(&request_id),
			"missing pending request for send message"
		);
		return;
	};

	let msg = protocol::ToRivetTunnelMessage {
		message_id: protocol::MessageId {
			gateway_id,
			request_id,
			message_index: envoy_message_index,
		},
		message_kind: message_kind.clone(),
	};

	let buffer_msg = msg.clone();
	let failed = ws_send(&ctx.shared, protocol::ToRivet::ToRivetTunnelMessage(msg)).await;

	if failed {
		if tracing::enabled!(tracing::Level::DEBUG) {
			tracing::debug!(
				request_id = id_to_str(&request_id),
				message = stringify_to_rivet_tunnel_message_kind(&message_kind),
				"buffering tunnel message, socket not connected to engine"
			);
		}
		let _ = ctx
			.shared
			.envoy_tx
			.send(crate::envoy::ToEnvoyMessage::BufferTunnelMsg { msg: buffer_msg });
	}
}

async fn send_response(
	shared: &SharedContext,
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
	mut response: HttpResponse,
) {
	let mut headers = HashableMap::new();
	for (k, v) in response.headers {
		headers.insert(k, v);
	}

	let is_streaming = response.body_stream.is_some();

	if !is_streaming {
		if let Some(body) = &response.body {
			if !headers.contains_key("content-length") {
				headers.insert("content-length".to_string(), body.len().to_string());
			}
		}
	}

	// Send the response start
	ws_send(
		shared,
		protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
			message_id: protocol::MessageId {
				gateway_id,
				request_id,
				message_index: 0,
			},
			message_kind: protocol::ToRivetTunnelMessageKind::ToRivetResponseStart(
				protocol::ToRivetResponseStart {
					status: response.status,
					headers,
					body: response.body,
					stream: is_streaming,
				},
			),
		}),
	)
	.await;

	// If streaming, read chunks from the body_stream and forward them
	if let Some(ref mut body_stream) = response.body_stream {
		let mut message_index: u16 = 1;
		while let Some(chunk) = body_stream.recv().await {
			let finish = chunk.finish;
			ws_send(
				shared,
				protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
					message_id: protocol::MessageId {
						gateway_id,
						request_id,
						message_index,
					},
					message_kind: protocol::ToRivetTunnelMessageKind::ToRivetResponseChunk(
						protocol::ToRivetResponseChunk {
							body: chunk.data,
							finish,
						},
					),
				}),
			)
			.await;
			message_index = message_index.wrapping_add(1);
			if finish {
				break;
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;
	use std::future::pending;
	use std::sync::Mutex;
	use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
	use std::time::Duration;

	use tokio::sync::Notify;
	use tokio::sync::oneshot;
	use tokio::task::yield_now;
	use tokio::time::Instant;

	use super::*;
	use crate::config::{BoxFuture, EnvoyCallbacks, WebSocketHandler, WebSocketSender};
	use crate::context::{SharedActorEntry, WsTxMessage};
	use crate::envoy::ToEnvoyMessage;

	struct DropSignal(Option<oneshot::Sender<()>>);

	impl Drop for DropSignal {
		fn drop(&mut self) {
			if let Some(tx) = self.0.take() {
				let _ = tx.send(());
			}
		}
	}

	struct TestCallbacks {
		fetch_started_tx: Mutex<Option<oneshot::Sender<()>>>,
		fetch_dropped_tx: Mutex<Option<oneshot::Sender<()>>>,
		release_fetch: Arc<Notify>,
		complete_fetch: AtomicBool,
	}

	impl TestCallbacks {
		fn idle() -> Self {
			Self {
				fetch_started_tx: Mutex::new(None),
				fetch_dropped_tx: Mutex::new(None),
				release_fetch: Arc::new(Notify::new()),
				complete_fetch: AtomicBool::new(true),
			}
		}

		fn completing(
			fetch_started_tx: oneshot::Sender<()>,
			release_fetch: Arc<Notify>,
		) -> Self {
			Self {
				fetch_started_tx: Mutex::new(Some(fetch_started_tx)),
				fetch_dropped_tx: Mutex::new(None),
				release_fetch,
				complete_fetch: AtomicBool::new(true),
			}
		}

		fn hanging(
			fetch_started_tx: oneshot::Sender<()>,
			fetch_dropped_tx: oneshot::Sender<()>,
		) -> Self {
			Self {
				fetch_started_tx: Mutex::new(Some(fetch_started_tx)),
				fetch_dropped_tx: Mutex::new(Some(fetch_dropped_tx)),
				release_fetch: Arc::new(Notify::new()),
				complete_fetch: AtomicBool::new(false),
			}
		}
	}

	struct DeferredStopCallbacks {
		stop_handle_tx: Mutex<Option<oneshot::Sender<crate::config::ActorStopHandle>>>,
	}

	impl EnvoyCallbacks for TestCallbacks {
		fn on_actor_start(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_generation: u32,
			_config: protocol::ActorConfig,
			_preloaded_kv: Option<protocol::PreloadedKv>,
			_sqlite_schema_version: u32,
			_sqlite_startup_data: Option<protocol::SqliteStartupData>,
		) -> BoxFuture<anyhow::Result<()>> {
			Box::pin(async { Ok(()) })
		}

		fn on_actor_stop(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_generation: u32,
			_reason: protocol::StopActorReason,
		) -> BoxFuture<anyhow::Result<()>> {
			Box::pin(async { Ok(()) })
		}

		fn on_shutdown(&self) {}

		fn fetch(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_gateway_id: protocol::GatewayId,
			_request_id: protocol::RequestId,
			_request: HttpRequest,
		) -> BoxFuture<anyhow::Result<HttpResponse>> {
			let fetch_started_tx = self
				.fetch_started_tx
				.lock()
				.expect("fetch_started mutex poisoned")
				.take();
			let fetch_dropped_tx = self
				.fetch_dropped_tx
				.lock()
				.expect("fetch_dropped mutex poisoned")
				.take();
			let release_fetch = self.release_fetch.clone();
			let complete_fetch = self.complete_fetch.load(Ordering::Acquire);

			Box::pin(async move {
				if let Some(tx) = fetch_started_tx {
					let _ = tx.send(());
				}

				let _drop_signal = DropSignal(fetch_dropped_tx);

				if complete_fetch {
					release_fetch.notified().await;
					Ok(HttpResponse {
						status: 200,
						headers: HashMap::new(),
						body: Some(Vec::new()),
						body_stream: None,
					})
				} else {
					pending::<()>().await;
					unreachable!("pending future should never resolve");
				}
			})
		}

		fn websocket(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_gateway_id: protocol::GatewayId,
			_request_id: protocol::RequestId,
			_request: HttpRequest,
			_path: String,
			_headers: HashMap<String, String>,
			_is_hibernatable: bool,
			_is_restoring_hibernatable: bool,
			_sender: WebSocketSender,
		) -> BoxFuture<anyhow::Result<WebSocketHandler>> {
			Box::pin(async {
				Ok(WebSocketHandler {
					on_message: Box::new(|_| Box::pin(async {})),
					on_close: Box::new(|_, _| Box::pin(async {})),
					on_open: None,
				})
			})
		}

		fn can_hibernate(
			&self,
			_actor_id: &str,
			_gateway_id: &protocol::GatewayId,
			_request_id: &protocol::RequestId,
			_request: &HttpRequest,
		) -> BoxFuture<anyhow::Result<bool>> {
			Box::pin(async { Ok(false) })
		}
	}

	impl EnvoyCallbacks for DeferredStopCallbacks {
		fn on_actor_start(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_generation: u32,
			_config: protocol::ActorConfig,
			_preloaded_kv: Option<protocol::PreloadedKv>,
			_sqlite_schema_version: u32,
			_sqlite_startup_data: Option<protocol::SqliteStartupData>,
		) -> BoxFuture<anyhow::Result<()>> {
			Box::pin(async { Ok(()) })
		}

		fn on_actor_stop_with_completion(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_generation: u32,
			_reason: protocol::StopActorReason,
			stop_handle: crate::config::ActorStopHandle,
		) -> BoxFuture<anyhow::Result<()>> {
			let stop_handle_tx = self
				.stop_handle_tx
				.lock()
				.expect("stop handle mutex poisoned")
				.take();

			Box::pin(async move {
				let Some(tx) = stop_handle_tx else {
					anyhow::bail!("stop handle sender missing");
				};

				tx.send(stop_handle)
					.map_err(|_| anyhow::anyhow!("failed to publish stop handle"))?;
				Ok(())
			})
		}

		fn on_shutdown(&self) {}

		fn fetch(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_gateway_id: protocol::GatewayId,
			_request_id: protocol::RequestId,
			_request: HttpRequest,
		) -> BoxFuture<anyhow::Result<HttpResponse>> {
			Box::pin(async { anyhow::bail!("fetch should not be called in deferred stop test") })
		}

		fn websocket(
			&self,
			_handle: EnvoyHandle,
			_actor_id: String,
			_gateway_id: protocol::GatewayId,
			_request_id: protocol::RequestId,
			_request: HttpRequest,
			_path: String,
			_headers: HashMap<String, String>,
			_is_hibernatable: bool,
			_is_restoring_hibernatable: bool,
			_sender: WebSocketSender,
		) -> BoxFuture<anyhow::Result<WebSocketHandler>> {
			Box::pin(async { anyhow::bail!("websocket should not be called in deferred stop test") })
		}

		fn can_hibernate(
			&self,
			_actor_id: &str,
			_gateway_id: &protocol::GatewayId,
			_request_id: &protocol::RequestId,
			_request: &HttpRequest,
		) -> BoxFuture<anyhow::Result<bool>> {
			Box::pin(async { Ok(false) })
		}
	}

	fn build_shared_context(
		callbacks: Arc<dyn EnvoyCallbacks>,
	) -> (Arc<SharedContext>, mpsc::UnboundedReceiver<ToEnvoyMessage>) {
		let (envoy_tx, envoy_rx) = mpsc::unbounded_channel();
		let shared = Arc::new(SharedContext {
			config: crate::config::EnvoyConfig {
				version: 1,
				endpoint: "http://127.0.0.1:1".to_string(),
				token: None,
				namespace: "test".to_string(),
				pool_name: "test".to_string(),
				prepopulate_actor_names: HashMap::new(),
				metadata: None,
				not_global: true,
				debug_latency_ms: None,
				callbacks,
			},
			envoy_key: "test-envoy".to_string(),
			envoy_tx,
			actors: Arc::new(std::sync::Mutex::new(HashMap::new())),
			live_tunnel_requests: Arc::new(std::sync::Mutex::new(HashMap::new())),
			pending_hibernation_restores: Arc::new(std::sync::Mutex::new(HashMap::new())),
			ws_tx: Arc::new(tokio::sync::Mutex::new(None::<mpsc::UnboundedSender<WsTxMessage>>)),
			protocol_metadata: Arc::new(tokio::sync::Mutex::new(None)),
			shutting_down: std::sync::atomic::AtomicBool::new(false),
		});
		(shared, envoy_rx)
	}

	fn actor_config() -> protocol::ActorConfig {
		protocol::ActorConfig {
			name: "test".to_string(),
			key: Some("test-key".to_string()),
			create_ts: 0,
			input: None,
		}
	}

	fn request_start() -> protocol::ToEnvoyRequestStart {
		protocol::ToEnvoyRequestStart {
			actor_id: "test-actor".to_string(),
			method: "GET".to_string(),
			path: "/test".to_string(),
			headers: HashableMap::new(),
			body: None,
			stream: false,
		}
	}

	fn message_id() -> protocol::MessageId {
		protocol::MessageId {
			gateway_id: [1, 2, 3, 4],
			request_id: [5, 6, 7, 8],
			message_index: 0,
		}
	}

	async fn wait_for_zero(active_http_request_count: &Arc<AsyncCounter>) {
		assert!(
			active_http_request_count
				.wait_zero(Instant::now() + Duration::from_secs(2))
				.await,
			"timed out waiting for active HTTP request count to reach zero"
		);
	}

	async fn wait_for_stopped_event(envoy_rx: &mut mpsc::UnboundedReceiver<ToEnvoyMessage>) {
		tokio::time::timeout(Duration::from_secs(2), async {
			loop {
				let Some(msg) = envoy_rx.recv().await else {
					panic!("envoy channel closed before stopped event");
				};

				if let ToEnvoyMessage::SendEvents { events } = msg {
					if events.iter().any(|event| {
						matches!(
							event.inner,
							protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
								state: protocol::ActorState::ActorStateStopped(_),
							})
						)
					}) {
						return;
					}
				}
			}
		})
		.await
		.expect("timed out waiting for stopped event");
	}

	async fn assert_no_stopped_event(envoy_rx: &mut mpsc::UnboundedReceiver<ToEnvoyMessage>) {
		let result = tokio::time::timeout(Duration::from_millis(100), async {
			loop {
				let Some(msg) = envoy_rx.recv().await else {
					panic!("envoy channel closed while waiting for non-stopped event");
				};

				if let ToEnvoyMessage::SendEvents { events } = msg {
					if events.iter().any(|event| {
						matches!(
							event.inner,
							protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
								state: protocol::ActorState::ActorStateStopped(_),
							})
						)
					}) {
						panic!("received stopped event before teardown completion");
					}
				}
			}
		})
		.await;

		assert!(result.is_err(), "stopped event arrived before teardown completion");
	}

	#[tokio::test]
	async fn active_http_request_count_tracks_in_flight_fetches() {
		let (fetch_started_tx, fetch_started_rx) = oneshot::channel();
		let release_fetch = Arc::new(Notify::new());
		let callbacks = Arc::new(TestCallbacks::completing(
			fetch_started_tx,
			release_fetch.clone(),
		));
		let (shared, mut envoy_rx) = build_shared_context(callbacks);
		let (actor_tx, active_http_request_count) = create_actor(
			shared,
			"actor-1".to_string(),
			1,
			actor_config(),
			Vec::new(),
			None,
			0,
			None,
		);

		actor_tx
			.send(ToActor::ReqStart {
				message_id: message_id(),
				req: request_start(),
			})
			.expect("failed to send request start");

		tokio::time::timeout(Duration::from_secs(2), fetch_started_rx)
			.await
			.expect("timed out waiting for fetch start")
			.expect("fetch start sender dropped");
		assert_eq!(active_http_request_count.load(), 1);

		release_fetch.notify_waiters();
		wait_for_zero(&active_http_request_count).await;

		actor_tx
			.send(ToActor::Stop {
				command_idx: 1,
				reason: protocol::StopActorReason::StopIntent,
			})
			.expect("failed to send stop");
		wait_for_stopped_event(&mut envoy_rx).await;
	}

	#[tokio::test]
	async fn actor_stop_aborts_in_flight_http_requests_before_stopped_event() {
		let (fetch_started_tx, fetch_started_rx) = oneshot::channel();
		let (fetch_dropped_tx, fetch_dropped_rx) = oneshot::channel();
		let callbacks = Arc::new(TestCallbacks::hanging(fetch_started_tx, fetch_dropped_tx));
		let (shared, mut envoy_rx) = build_shared_context(callbacks);
		let (actor_tx, active_http_request_count) = create_actor(
			shared,
			"actor-2".to_string(),
			1,
			actor_config(),
			Vec::new(),
			None,
			0,
			None,
		);

		actor_tx
			.send(ToActor::ReqStart {
				message_id: message_id(),
				req: request_start(),
			})
			.expect("failed to send request start");

		tokio::time::timeout(Duration::from_secs(2), fetch_started_rx)
			.await
			.expect("timed out waiting for fetch start")
			.expect("fetch start sender dropped");
		assert_eq!(active_http_request_count.load(), 1);

		actor_tx
			.send(ToActor::Stop {
				command_idx: 1,
				reason: protocol::StopActorReason::StopIntent,
			})
			.expect("failed to send stop");

		tokio::time::timeout(Duration::from_secs(2), fetch_dropped_rx)
			.await
			.expect("timed out waiting for fetch abort")
			.expect("fetch drop sender dropped");
		wait_for_stopped_event(&mut envoy_rx).await;
		assert_eq!(active_http_request_count.load(), 0);
	}

	#[tokio::test]
	async fn actor_stop_waits_for_completion_handle_before_stopped_event() {
		let (stop_handle_tx, stop_handle_rx) = oneshot::channel();
		let callbacks = Arc::new(DeferredStopCallbacks {
			stop_handle_tx: Mutex::new(Some(stop_handle_tx)),
		});
		let (shared, mut envoy_rx) = build_shared_context(callbacks);
		let (actor_tx, _active_http_request_count) = create_actor(
			shared,
			"actor-3".to_string(),
			1,
			actor_config(),
			Vec::new(),
			None,
			0,
			None,
		);

		actor_tx
			.send(ToActor::Stop {
				command_idx: 1,
				reason: protocol::StopActorReason::StopIntent,
			})
			.expect("failed to send stop");

		let stop_handle = tokio::time::timeout(Duration::from_secs(2), stop_handle_rx)
			.await
			.expect("timed out waiting for stop handle")
			.expect("stop handle sender dropped");
		assert_no_stopped_event(&mut envoy_rx).await;

		assert!(stop_handle.complete(), "stop handle should complete once");
		wait_for_stopped_event(&mut envoy_rx).await;
	}

	#[tokio::test]
	async fn http_request_guard_counter_is_visible_through_envoy_handle() {
		let (shared, _envoy_rx) = build_shared_context(Arc::new(TestCallbacks::idle()));
		let handle = EnvoyHandle {
			shared: shared.clone(),
			started_rx: tokio::sync::watch::channel(()).1,
		};
		let counter = Arc::new(AsyncCounter::new());
		shared
			.actors
			.lock()
			.expect("shared actor registry poisoned")
			.entry("actor-4".to_string())
			.or_insert_with(HashMap::new)
			.insert(
				4,
				SharedActorEntry {
					handle: mpsc::unbounded_channel().0,
					active_http_request_count: counter.clone(),
				},
			);

		let request_guard = ActiveHttpRequestGuard::new(counter);
		let handle_counter = handle
			.http_request_counter("actor-4", Some(4))
			.expect("counter should be returned");
		assert_eq!(handle_counter.load(), 1);

		drop(request_guard);
		assert_eq!(handle_counter.load(), 0);
		assert!(
			handle_counter
				.wait_zero(Instant::now() + Duration::from_secs(2))
				.await
		);
	}

	#[tokio::test]
	async fn active_http_request_counter_waiter_wakes_only_after_final_drop() {
		let counter = Arc::new(AsyncCounter::new());
		let guard_a = ActiveHttpRequestGuard::new(counter.clone());
		let guard_b = ActiveHttpRequestGuard::new(counter.clone());
		let wake_count = Arc::new(AtomicUsize::new(0));

		let waiter = tokio::spawn({
			let counter = counter.clone();
			let wake_count = wake_count.clone();
			async move {
				let woke = counter
					.wait_zero(Instant::now() + Duration::from_secs(2))
					.await;
				if woke {
					wake_count.fetch_add(1, Ordering::SeqCst);
				}
				woke
			}
		});

		yield_now().await;
		drop(guard_a);
		yield_now().await;
		assert_eq!(wake_count.load(Ordering::SeqCst), 0);
		assert!(
			!waiter.is_finished(),
			"waiter should stay pending until the final in-flight request completes"
		);

		drop(guard_b);
		assert!(waiter.await.expect("waiter should join"));
		assert_eq!(wake_count.load(Ordering::SeqCst), 1);
	}
}
