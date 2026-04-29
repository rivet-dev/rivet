use anyhow::{Context, Result};
use gas::prelude::util::serde::HashableMap;
use pegboard::actor_lifecycle::{
	ActorLifecycleMessage, ActorSuspension, RESTORE_HTTP_RETRY_AFTER_SECONDS,
	RESTORE_WS_CLOSE_CODE, RESTORE_WS_CLOSE_REASON,
};
use pegboard::pubsub_subjects::GatewayReceiverSubject;
use rivet_envoy_protocol::{self as protocol, PROTOCOL_VERSION, versioned};
use scc::HashMap;
use universalpubsub::PublishOpts;
use vbare::OwnedVersionedData;

pub type RouteKey = (protocol::GatewayId, protocol::RequestId);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteKind {
	Http,
	WebSocket,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteState {
	pub actor_id: String,
	pub kind: RouteKind,
}

pub async fn maybe_reject_suspended_tunnel_message(
	udb: &universaldb::Database,
	ups: &universalpubsub::PubSub,
	routes: &HashMap<RouteKey, RouteState>,
	msg: &protocol::ToEnvoyTunnelMessage,
) -> Result<bool> {
	let actor_id = match actor_id_for_tunnel_message(routes, msg).await {
		Some(actor_id) => actor_id,
		None => return Ok(false),
	};

	if !pegboard::actor_lifecycle::is_suspended(udb, &actor_id).await? {
		return Ok(false);
	}

	publish_suspended_response(ups, msg).await?;
	Ok(true)
}

pub async fn track_forwarded_tunnel_message(
	routes: &HashMap<RouteKey, RouteState>,
	msg: &protocol::ToEnvoyTunnelMessage,
) {
	match &msg.message_kind {
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(req) => {
			let _ = routes
				.insert_async(
					(msg.message_id.gateway_id, msg.message_id.request_id),
					RouteState {
						actor_id: req.actor_id.clone(),
						kind: RouteKind::Http,
					},
				)
				.await;
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(open) => {
			let _ = routes
				.insert_async(
					(msg.message_id.gateway_id, msg.message_id.request_id),
					RouteState {
						actor_id: open.actor_id.clone(),
						kind: RouteKind::WebSocket,
					},
				)
				.await;
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(_)
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => {
			routes
				.remove_async(&(msg.message_id.gateway_id, msg.message_id.request_id))
				.await;
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(_)
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(_) => {}
	}
}

pub async fn track_from_envoy_tunnel_message(
	routes: &HashMap<RouteKey, RouteState>,
	msg: &protocol::ToRivetTunnelMessage,
) {
	match &msg.message_kind {
		protocol::ToRivetTunnelMessageKind::ToRivetResponseStart(res)
			if !res.stream =>
		{
			routes
				.remove_async(&(msg.message_id.gateway_id, msg.message_id.request_id))
				.await;
		}
		protocol::ToRivetTunnelMessageKind::ToRivetResponseAbort
		| protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(_) => {
			routes
				.remove_async(&(msg.message_id.gateway_id, msg.message_id.request_id))
				.await;
		}
		protocol::ToRivetTunnelMessageKind::ToRivetResponseStart(_)
		| protocol::ToRivetTunnelMessageKind::ToRivetResponseChunk(_)
		| protocol::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(_)
		| protocol::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(_)
		| protocol::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(_) => {}
	}
}

pub async fn handle_lifecycle_message(
	ups: &universalpubsub::PubSub,
	routes: &HashMap<RouteKey, RouteState>,
	payload: &[u8],
) -> Result<()> {
	let message = pegboard::actor_lifecycle::decode_lifecycle_message(payload)?;
	match message {
		ActorLifecycleMessage::Suspended(suspension) => {
			close_websocket_routes_for_suspension(ups, routes, &suspension).await
		}
		ActorLifecycleMessage::Resumed { actor_id: _ } => Ok(()),
	}
}

pub async fn close_websocket_routes_for_suspension(
	ups: &universalpubsub::PubSub,
	routes: &HashMap<RouteKey, RouteState>,
	suspension: &ActorSuspension,
) -> Result<()> {
	let mut closing = Vec::new();
	routes.iter_async(|route, state| {
		if state.actor_id == suspension.actor_id && state.kind == RouteKind::WebSocket {
			closing.push(*route);
		}
		true
	}).await;

	for (gateway_id, request_id) in closing {
		let msg = protocol::ToRivetTunnelMessage {
			message_id: protocol::MessageId {
				gateway_id,
				request_id,
				message_index: 0,
			},
			message_kind: suspended_ws_close(),
		};
		publish_to_gateway(ups, msg).await?;
		routes.remove_async(&(gateway_id, request_id)).await;
	}

	Ok(())
}

async fn actor_id_for_tunnel_message(
	routes: &HashMap<RouteKey, RouteState>,
	msg: &protocol::ToEnvoyTunnelMessage,
) -> Option<String> {
	match &msg.message_kind {
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(req) => Some(req.actor_id.clone()),
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(open) => {
			Some(open.actor_id.clone())
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(_)
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(_)
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(_) => routes
			.get_async(&(msg.message_id.gateway_id, msg.message_id.request_id))
			.await
			.map(|entry| entry.actor_id.clone()),
	}
}

async fn publish_suspended_response(
	ups: &universalpubsub::PubSub,
	msg: &protocol::ToEnvoyTunnelMessage,
) -> Result<()> {
	let message_kind = match &msg.message_kind {
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(_)
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(_)
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => suspended_http_response(),
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(_)
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(_)
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(_) => suspended_ws_close(),
	};

	publish_to_gateway(
		ups,
		protocol::ToRivetTunnelMessage {
			message_id: msg.message_id.clone(),
			message_kind,
		},
	)
	.await
}

fn suspended_http_response() -> protocol::ToRivetTunnelMessageKind {
	let body = b"actor restore in progress".to_vec();
	let mut headers = HashableMap::new();
	headers.insert(
		"retry-after".to_string(),
		RESTORE_HTTP_RETRY_AFTER_SECONDS.to_string(),
	);
	headers.insert(
		"x-rivet-error".to_string(),
		RESTORE_WS_CLOSE_REASON.to_string(),
	);
	headers.insert("content-length".to_string(), body.len().to_string());

	protocol::ToRivetTunnelMessageKind::ToRivetResponseStart(protocol::ToRivetResponseStart {
		status: 503,
		headers,
		body: Some(body),
		stream: false,
	})
}

fn suspended_ws_close() -> protocol::ToRivetTunnelMessageKind {
	protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(protocol::ToRivetWebSocketClose {
		code: Some(RESTORE_WS_CLOSE_CODE),
		reason: Some(RESTORE_WS_CLOSE_REASON.to_string()),
		hibernate: false,
	})
}

async fn publish_to_gateway(
	ups: &universalpubsub::PubSub,
	msg: protocol::ToRivetTunnelMessage,
) -> Result<()> {
	let gateway_reply_to = GatewayReceiverSubject::new(msg.message_id.gateway_id).to_string();
	let msg_serialized =
		versioned::ToGateway::wrap_latest(protocol::ToGateway::ToRivetTunnelMessage(msg))
			.serialize_with_embedded_version(PROTOCOL_VERSION)
			.context("serialize suspended tunnel response")?;

	ups.publish(&gateway_reply_to, &msg_serialized, PublishOpts::one())
		.await
		.with_context(|| format!("publish suspended tunnel response to {gateway_reply_to}"))
}
