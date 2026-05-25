use rivet_envoy_protocol as protocol;

use crate::connection::ws_send;
use crate::envoy::{BufferedActorMessage, EnvoyContext};
use crate::utils::{display_id, tunnel_request_key};

pub struct HibernatingWebSocketMetadata {
	pub gateway_id: protocol::GatewayId,
	pub request_id: protocol::RequestId,
	pub envoy_message_index: u16,
	pub rivet_message_index: u16,
	pub path: String,
	pub headers: std::collections::HashMap<String, String>,
}

pub async fn handle_tunnel_message(ctx: &mut EnvoyContext, msg: protocol::ToEnvoyTunnelMessage) {
	let message_id = msg.message_id;
	let message_index = message_id.message_index;
	let message_kind = to_envoy_tunnel_message_kind_name(&msg.message_kind);
	let inner_data_len = to_envoy_tunnel_message_inner_data_len(&msg.message_kind);
	tracing::trace!(
		gateway_id = %display_id(&message_id.gateway_id),
		request_id = %display_id(&message_id.request_id),
		message_index,
		message_kind,
		inner_data_len,
		"received tunnel message from engine"
	);
	match msg.message_kind {
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(req) => {
			handle_request_start(ctx, message_id, req).await;
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(chunk) => {
			handle_request_chunk(ctx, message_id, chunk);
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => {
			handle_request_abort(ctx, message_id);
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(open) => {
			handle_ws_open(ctx, message_id, open).await;
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(msg) => {
			handle_ws_message(ctx, message_id, msg);
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(close) => {
			handle_ws_close(ctx, message_id, close).await;
		}
	}
}

async fn handle_request_start(
	ctx: &mut EnvoyContext,
	message_id: protocol::MessageId,
	req: protocol::ToEnvoyRequestStart,
) {
	let actor_id = req.actor_id.clone();
	let has_actor = ctx.get_actor(&actor_id, None).is_some();

	if !has_actor {
		tracing::warn!(actor_id = %actor_id, "received request for unknown actor");
		send_error_response(ctx, message_id.gateway_id, message_id.request_id).await;
		return;
	}

	ctx.request_to_actor.insert(
		tunnel_request_key(&message_id.gateway_id, &message_id.request_id),
		actor_id.clone(),
	);

	let actor = ctx.get_actor(&actor_id, None).unwrap();
	let _ = actor
		.handle
		.send(crate::actor::ToActor::ReqStart { message_id, req });
}

fn handle_request_chunk(
	ctx: &mut EnvoyContext,
	message_id: protocol::MessageId,
	chunk: protocol::ToEnvoyRequestChunk,
) {
	let actor_id = ctx
		.request_to_actor
		.get(tunnel_request_key(&message_id.gateway_id, &message_id.request_id))
		.cloned();

	let finish = chunk.finish;

	if let Some(actor_id) = &actor_id {
		if let Some(actor) = ctx.get_actor(actor_id, None) {
			let _ = actor.handle.send(crate::actor::ToActor::ReqChunk {
				message_id: message_id.clone(),
				chunk,
			});
		}
	}

	if finish {
		ctx.request_to_actor
			.remove(tunnel_request_key(&message_id.gateway_id, &message_id.request_id));
	}
}

fn handle_request_abort(ctx: &mut EnvoyContext, message_id: protocol::MessageId) {
	let actor_id = ctx
		.request_to_actor
		.get(tunnel_request_key(&message_id.gateway_id, &message_id.request_id))
		.cloned();
	if let Some(actor_id) = &actor_id {
		if let Some(actor) = ctx.get_actor(actor_id, None) {
			let _ = actor.handle.send(crate::actor::ToActor::ReqAbort {
				message_id: message_id.clone(),
			});
		}
	}

	ctx.request_to_actor
		.remove(tunnel_request_key(&message_id.gateway_id, &message_id.request_id));
}

async fn handle_ws_open(
	ctx: &mut EnvoyContext,
	message_id: protocol::MessageId,
	open: protocol::ToEnvoyWebSocketOpen,
) {
	let actor_id = open.actor_id.clone();
	let has_actor = ctx.get_actor(&actor_id, None).is_some();

	if !has_actor {
		tracing::warn!(actor_id = %actor_id, "received ws open for unknown actor");

		ws_send(
			&ctx.shared,
			protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
				message_id,
				message_kind: protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
					protocol::ToRivetWebSocketClose {
						code: Some(1011),
						reason: Some("Actor not found".to_string()),
						hibernate: false,
					},
				),
			}),
		)
		.await;
		return;
	}

	ctx.request_to_actor.insert(
		tunnel_request_key(&message_id.gateway_id, &message_id.request_id),
		actor_id.clone(),
	);
	ctx.shared
		.live_tunnel_requests
		.upsert_async(
			tunnel_request_key(&message_id.gateway_id, &message_id.request_id),
			actor_id.clone(),
		)
		.await;

	// Convert HashableMap headers to BTreeMap for the actor message
	let headers = open
		.headers
		.iter()
		.map(|(k, v)| (k.clone(), v.clone()))
		.collect();

	let actor = ctx.get_actor(&actor_id, None).unwrap();
	let _ = actor.handle.send(crate::actor::ToActor::WsOpen {
		message_id,
		path: open.path,
		headers,
	});
}

fn handle_ws_message(
	ctx: &mut EnvoyContext,
	message_id: protocol::MessageId,
	msg: protocol::ToEnvoyWebSocketMessage,
) {
	let data_len = msg.data.len();
	let binary = msg.binary;
	let gateway_id = message_id.gateway_id;
	let request_id = message_id.request_id;
	let message_index = message_id.message_index;
	let actor_id = ctx
		.request_to_actor
		.get(tunnel_request_key(&message_id.gateway_id, &message_id.request_id))
		.cloned();
	if let Some(actor_id) = &actor_id {
		if let Some(actor) = ctx.get_actor(actor_id, None) {
			tracing::trace!(
				actor_id = %actor_id,
				gateway_id = %display_id(&gateway_id),
				request_id = %display_id(&request_id),
				message_index,
				data_len,
				binary,
				"dispatching websocket message to actor task"
			);
			if actor
				.handle
				.send(crate::actor::ToActor::WsMsg { message_id, msg })
				.is_ok()
				{
					tracing::trace!(
						actor_id = %actor_id,
						gateway_id = %display_id(&gateway_id),
						request_id = %display_id(&request_id),
						message_index,
						data_len,
						binary,
					"dispatched websocket message to actor task"
				);
				} else {
					tracing::warn!(
						actor_id = %actor_id,
						gateway_id = %display_id(&gateway_id),
						request_id = %display_id(&request_id),
						message_index,
						data_len,
						binary,
					"actor websocket task channel closed"
				);
			}
		} else {
			tracing::trace!(
				actor_id = %actor_id,
				gateway_id = %display_id(&gateway_id),
				request_id = %display_id(&request_id),
				message_index,
				data_len,
				binary,
				"buffering websocket message for actor task"
			);
			ctx.buffered_actor_messages
				.entry(actor_id.clone())
				.or_default()
				.push(BufferedActorMessage::WsMsg { message_id, msg });
		}
	} else {
		tracing::warn!(
			gateway_id = %display_id(&gateway_id),
			request_id = %display_id(&request_id),
			message_index,
			data_len,
			binary,
			"received websocket message for unknown tunnel request"
		);
	}
}

async fn handle_ws_close(
	ctx: &mut EnvoyContext,
	message_id: protocol::MessageId,
	close: protocol::ToEnvoyWebSocketClose,
) {
	let actor_id = ctx
		.request_to_actor
		.get(tunnel_request_key(&message_id.gateway_id, &message_id.request_id))
		.cloned();
	if let Some(actor_id) = &actor_id {
		if let Some(actor) = ctx.get_actor(actor_id, None) {
			let _ = actor.handle.send(crate::actor::ToActor::WsClose {
				message_id: message_id.clone(),
				close,
			});
		} else {
			ctx.buffered_actor_messages
				.entry(actor_id.clone())
				.or_default()
				.push(BufferedActorMessage::WsClose {
					message_id: message_id.clone(),
					close,
				});
		}
	}

	ctx.request_to_actor
		.remove(tunnel_request_key(&message_id.gateway_id, &message_id.request_id));
	ctx.shared
		.live_tunnel_requests
		.remove_async(&tunnel_request_key(
			&message_id.gateway_id,
			&message_id.request_id,
		))
		.await;
}

fn to_envoy_tunnel_message_kind_name(
	kind: &protocol::ToEnvoyTunnelMessageKind,
) -> &'static str {
	match kind {
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(_) => "ToEnvoyRequestStart",
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(_) => "ToEnvoyRequestChunk",
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => "ToEnvoyRequestAbort",
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(_) => "ToEnvoyWebSocketOpen",
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(_) => {
			"ToEnvoyWebSocketMessage"
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(_) => "ToEnvoyWebSocketClose",
	}
}

fn to_envoy_tunnel_message_inner_data_len(kind: &protocol::ToEnvoyTunnelMessageKind) -> usize {
	match kind {
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(msg) => {
			msg.body.as_ref().map_or(0, Vec::len)
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(msg) => msg.body.len(),
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(msg) => msg.data.len(),
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(_)
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(_) => 0,
	}
}

pub fn send_hibernatable_ws_message_ack(
	ctx: &mut EnvoyContext,
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
	envoy_message_index: u16,
) {
	let actor_id = ctx
		.request_to_actor
		.get(tunnel_request_key(&gateway_id, &request_id))
		.cloned();
	if let Some(actor_id) = &actor_id {
		if let Some(actor) = ctx.get_actor(actor_id, None) {
			let _ = actor.handle.send(crate::actor::ToActor::HwsAck {
				gateway_id,
				request_id,
				envoy_message_index,
			});
		}
	}
}

pub async fn resend_buffered_tunnel_messages(ctx: &mut EnvoyContext) {
	if ctx.buffered_messages.is_empty() {
		return;
	}

	tracing::info!(
		count = ctx.buffered_messages.len(),
		"resending buffered tunnel messages"
	);

	let messages = std::mem::take(&mut ctx.buffered_messages);
	crate::metrics::METRICS
		.outbound_queue_depth
		.sub(messages.len() as i64);
	for msg in messages {
		ws_send(&ctx.shared, protocol::ToRivet::ToRivetTunnelMessage(msg)).await;
	}
}

async fn send_error_response(
	ctx: &EnvoyContext,
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
) {
	let body = b"Actor not found".to_vec();
	let mut headers = rivet_util_serde::HashableMap::new();
	headers.insert(
		"x-rivet-error".to_string(),
		"envoy.actor_not_found".to_string(),
	);
	headers.insert("content-length".to_string(), body.len().to_string());

	ws_send(
		&ctx.shared,
		protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
			message_id: protocol::MessageId {
				gateway_id,
				request_id,
				message_index: 0,
			},
			message_kind: protocol::ToRivetTunnelMessageKind::ToRivetResponseStart(
				protocol::ToRivetResponseStart {
					status: 503,
					headers,
					body: Some(body),
					stream: false,
				},
			),
		}),
	)
	.await;
}
