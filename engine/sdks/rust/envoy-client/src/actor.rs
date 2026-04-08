use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

use rivet_envoy_protocol as protocol;
use rivet_util::serde::HashableMap;
use tokio::sync::mpsc;

use crate::config::{HttpRequest, HttpResponse, WebSocketMessage};
use crate::connection::ws_send;
use crate::context::SharedContext;
use crate::handle::EnvoyHandle;
use crate::stringify::stringify_to_rivet_tunnel_message_kind;
use crate::utils::{id_to_str, wrapping_add_u16, wrapping_lte_u16, wrapping_sub_u16, BufferMap};

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
	HwsRestore {
		meta_entries: Vec<crate::tunnel::HibernatingWebSocketMetadata>,
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
}

pub fn create_actor(
	shared: Arc<SharedContext>,
	actor_id: String,
	generation: u32,
	config: protocol::ActorConfig,
	hibernating_requests: Vec<protocol::HibernatingRequest>,
	preloaded_kv: Option<protocol::PreloadedKv>,
) -> mpsc::UnboundedSender<ToActor> {
	let (tx, rx) = mpsc::unbounded_channel();
	tokio::spawn(actor_inner(
		shared,
		actor_id,
		generation,
		config,
		hibernating_requests,
		preloaded_kv,
		rx,
	));
	tx
}

async fn actor_inner(
	shared: Arc<SharedContext>,
	actor_id: String,
	generation: u32,
	config: protocol::ActorConfig,
	hibernating_requests: Vec<protocol::HibernatingRequest>,
	preloaded_kv: Option<protocol::PreloadedKv>,
	mut rx: mpsc::UnboundedReceiver<ToActor>,
) {
	let handle = EnvoyHandle {
		shared: shared.clone(),
		started_rx: tokio::sync::watch::channel(true).1,
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
	};

	// Call on_actor_start
	let start_result = shared
		.config
		.callbacks
		.on_actor_start(handle.clone(), actor_id.clone(), generation, config, preloaded_kv)
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

	// Send running state
	send_event(
		&mut ctx,
		protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
			state: protocol::ActorState::ActorStateRunning,
		}),
	);

	while let Some(msg) = rx.recv().await {
		match msg {
			ToActor::Intent { intent, error } => {
				send_event(
					&mut ctx,
					protocol::Event::EventActorIntent(protocol::EventActorIntent {
						intent,
					}),
				);
				if error.is_some() {
					ctx.error = error;
				}
			}
			ToActor::Stop {
				command_idx,
				reason,
			} => {
				if command_idx <= ctx.command_idx {
					tracing::warn!(command_idx, "ignoring already seen command");
					continue;
				}
				ctx.command_idx = command_idx;
				handle_stop(&mut ctx, &handle, reason).await;
				break;
			}
			ToActor::Lost => {
				handle_stop(&mut ctx, &handle, protocol::StopActorReason::Lost).await;
				break;
			}
			ToActor::SetAlarm { alarm_ts } => {
				send_event(
					&mut ctx,
					protocol::Event::EventActorSetAlarm(protocol::EventActorSetAlarm {
						alarm_ts,
					}),
				);
			}
			ToActor::ReqStart { message_id, req } => {
				handle_req_start(&mut ctx, &handle, message_id, req);
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
			ToActor::HwsRestore { meta_entries } => {
				handle_hws_restore(&mut ctx, &handle, meta_entries).await;
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

	tracing::debug!(actor_id = %ctx.actor_id, "envoy actor stopped");
}

fn send_event(ctx: &mut ActorContext, inner: protocol::Event) {
	let checkpoint = increment_checkpoint(ctx);
	let _ = ctx.shared.envoy_tx.send(crate::envoy::ToEnvoyMessage::SendEvents {
		events: vec![protocol::EventWrapper { checkpoint, inner }],
	});
}

async fn handle_stop(
	ctx: &mut ActorContext,
	handle: &EnvoyHandle,
	reason: protocol::StopActorReason,
) {
	let mut stop_code = if ctx.error.is_some() {
		protocol::StopCode::Error
	} else {
		protocol::StopCode::Ok
	};
	let mut stop_message = ctx.error.clone();

	let stop_result = ctx
		.shared
		.config
		.callbacks
		.on_actor_stop(
			handle.clone(),
			ctx.actor_id.clone(),
			ctx.generation,
			reason,
		)
		.await;

	if let Err(error) = stop_result {
		tracing::error!(actor_id = %ctx.actor_id, ?error, "actor stop failed");
		stop_code = protocol::StopCode::Error;
		if stop_message.is_none() {
			stop_message = Some(format!("{error:#}"));
		}
	}

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
	message_id: protocol::MessageId,
	req: protocol::ToEnvoyRequestStart,
) {
	let pending = PendingRequest {
		envoy_message_index: 0,
		body_tx: None,
	};
	ctx.pending_requests
		.insert(&[&message_id.gateway_id, &message_id.request_id], pending);

	let headers: HashMap<String, String> = req.headers.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

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

	tokio::spawn(async move {
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

async fn handle_ws_open(
	ctx: &mut ActorContext,
	handle: &EnvoyHandle,
	message_id: protocol::MessageId,
	path: String,
	headers: BTreeMap<String, String>,
) {
	ctx.pending_requests.insert(
		&[&message_id.gateway_id, &message_id.request_id],
		PendingRequest {
			envoy_message_index: 0,
			body_tx: None,
		},
	);

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

	let is_hibernatable = ctx.shared.config.callbacks.can_hibernate(
		&ctx.actor_id,
		&message_id.gateway_id,
		&message_id.request_id,
		&request,
	);

	// Create outgoing channel BEFORE calling websocket() so the sender is available immediately
	let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<crate::config::WsOutgoing>();
	let sender = crate::config::WebSocketSender {
		tx: outgoing_tx.clone(),
	};

	let ws_result = ctx
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
		.await;

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

			// Spawn task to forward outgoing WS messages to the tunnel.
			// Uses a shared counter so message indices don't conflict with send_actor_message.
			{
				let shared = ctx.shared.clone();
				let gateway_id = message_id.gateway_id;
				let request_id = message_id.request_id;
				let ws_msg_counter = std::sync::Arc::new(std::sync::atomic::AtomicU16::new(0));
				// Store counter ref on pending request so send_actor_message can coordinate
				if let Some(req) = ctx.pending_requests.get_mut(&[&gateway_id, &request_id]) {
					// The pending request's envoy_message_index will be managed separately;
					// the outgoing task uses its own counter space starting from a high offset.
				}
				tokio::spawn(async move {
					let mut idx: u16 = 0;
					while let Some(msg) = outgoing_rx.recv().await {
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

			// Send WebSocket open
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
			if let Some(ws_entry) = ctx
				.ws_entries
				.get_mut(&[&message_id.gateway_id, &message_id.request_id])
			{
				if let Some(handler) = &mut ws_entry.ws_handler {
					if let Some(on_open) = handler.on_open.take() {
						on_open().await;
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
) {
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
					envoy_message_index: 0,
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

			let (hws_outgoing_tx, _hws_outgoing_rx) = mpsc::unbounded_channel();
			let hws_sender = crate::config::WebSocketSender { tx: hws_outgoing_tx.clone() };

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
					let (outgoing_tx, _outgoing_rx) = mpsc::unbounded_channel();
					ctx.ws_entries.insert(
						&[&hib_req.gateway_id, &hib_req.request_id],
						WsEntry {
							is_hibernatable: true,
							rivet_message_index: meta.rivet_message_index,
							ws_handler: Some(ws_handler),
							outgoing_tx,
						},
					);
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
		let is_connected = hibernating_requests.iter().any(|req| {
			req.gateway_id == meta.gateway_id && req.request_id == meta.request_id
		});

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
	let req = ctx
		.pending_requests
		.get_mut(&[&gateway_id, &request_id]);
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
	let failed = ws_send(
		&ctx.shared,
		protocol::ToRivet::ToRivetTunnelMessage(msg),
	)
	.await;

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
