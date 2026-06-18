use anyhow::Result;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use pegboard::pubsub_subjects::GatewayReceiverSubject;
use rivet_envoy_protocol::{self as protocol, PROTOCOL_VERSION, versioned};
use std::{sync::Arc, time::Instant};
use tokio::sync::watch;
use universalpubsub as ups;
use universalpubsub::{NextOutput, PublishOpts, Subscriber};
use vbare::OwnedVersionedData;

use crate::{
	LifecycleResult, actor_lifecycle, conn::Conn, hibernating_requests, metrics,
	tunnel_message_task, ws_to_tunnel_task,
};

#[tracing::instrument(name = "tunnel_to_ws_task", skip_all, fields(ray_id=?ctx.ray_id(), req_id=?ctx.req_id(), namespace_id=%conn.namespace_id, pool_name=%conn.pool_name, envoy_key=%conn.envoy_key, protocol_version=%conn.protocol_version))]
pub async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	mut tunnel_sub: Subscriber,
	mut eviction_sub: Subscriber,
	mut tunnel_to_ws_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	loop {
		match recv_msg(
			&conn,
			&mut tunnel_sub,
			&mut eviction_sub,
			&mut tunnel_to_ws_abort_rx,
		)
		.await?
		{
			Ok(msg) => {
				let evicted = handle_message(&ctx, &conn, msg).await?;
				if evicted {
					return Ok(LifecycleResult::Evicted);
				}
			}
			Err(lifecycle_res) => return Ok(lifecycle_res),
		}
	}
}

#[tracing::instrument(skip_all)]
async fn recv_msg(
	conn: &Conn,
	tunnel_sub: &mut Subscriber,
	eviction_sub: &mut Subscriber,
	tunnel_to_ws_abort_rx: &mut watch::Receiver<()>,
) -> Result<std::result::Result<ups::Message, LifecycleResult>> {
	let tunnel_msg = tokio::select! {
		res = tunnel_sub.next() => {
			if let NextOutput::Message(tunnel_msg) = res? {
				tunnel_msg
			} else {
				tracing::debug!("tunnel sub closed");
				bail!("tunnel sub closed");
			}
		}
		_ = eviction_sub.next() => {
			tracing::debug!("envoy evicted");

			metrics::EVICTION_TOTAL
				.with_label_values(&[
					conn.namespace_id.to_string().as_str(),
					&conn.pool_name,
					conn.protocol_version.to_string().as_str(),
				])
				.inc();

			return Ok(Err(LifecycleResult::Evicted));
		}
		_ = tunnel_to_ws_abort_rx.changed() => {
			tracing::debug!("task aborted");
			return Ok(Err(LifecycleResult::Aborted));
		}
	};

	tracing::debug!(
		payload_len = tunnel_msg.payload.len(),
		"received message from pubsub, forwarding to WebSocket"
	);

	Ok(Ok(tunnel_msg))
}

#[tracing::instrument(skip_all)]
async fn handle_message(
	ctx: &StandaloneCtx,
	conn: &Conn,
	tunnel_msg: ups::Message,
) -> Result<bool> {
	tracing::trace!(
		namespace_id = %conn.namespace_id,
		pool_name = %conn.pool_name,
		envoy_key = %conn.envoy_key,
		message_id=?tunnel_msg.message_id,
		payload_len = tunnel_msg.payload.len(),
		"received gateway message from pubsub"
	);

	// Parse message
	let start = Instant::now();
	let msg = match versioned::ToEnvoyConn::deserialize_with_embedded_version(&tunnel_msg.payload) {
		Result::Ok(x) => x,
		Err(err) => {
			tracing::error!(?err, "failed to parse tunnel message");
			return Ok(false);
		}
	};

	// Need to reply to tunnel request so it can continue
	tunnel_msg.reply(&[]).await?;

	metrics::ACK_MSG_DURATION
		.with_label_values(&[conn.namespace_id.to_string().as_str(), &conn.pool_name])
		.observe(start.elapsed().as_secs_f64());

	let start = Instant::now();

	// Convert to ToEnvoy types
	let mut tunnel_message_meta = None;
	let to_client_msg = match msg {
		protocol::ToEnvoyConn::ToEnvoyConnPing(ping) => {
			// Publish pong to UPS
			let gateway_reply_to = GatewayReceiverSubject::new(ping.gateway_id);
			let msg_serialized = versioned::ToGateway::wrap_latest(
				protocol::ToGateway::ToGatewayPong(protocol::ToGatewayPong {
					request_id: ping.request_id,
					ts: ping.ts,
				}),
			)
			.serialize_with_embedded_version(PROTOCOL_VERSION)
			.context("failed to serialize pong message for gateway")?;
			ctx.ups()
				.context("failed to get UPS instance for tunnel message")?
				.publish(&gateway_reply_to, &msg_serialized, PublishOpts::one())
				.await
				.with_context(|| {
					format!(
						"failed to publish tunnel message to gateway reply topic: {}",
						gateway_reply_to
					)
				})?;

			// Not sent to envoy
			return Ok(false);
		}
		protocol::ToEnvoyConn::ToEnvoyConnClose => return Ok(true),
		protocol::ToEnvoyConn::ToEnvoyCommands(mut command_wrappers) => {
			// TODO: Parallelize
			for command_wrapper in &mut command_wrappers {
				hibernating_requests::hydrate_command_wrapper(ctx, command_wrapper).await?;
				if let protocol::Command::CommandStopActor(_) = &command_wrapper.inner {
					actor_lifecycle::stop_actor(conn, &command_wrapper.checkpoint).await?;
				}
			}

			// NOTE: `command_wrappers` is mutated in this match arm, it is not the same as the
			// ToEnvoyConn data
			protocol::ToEnvoy::ToEnvoyCommands(command_wrappers)
		}
		protocol::ToEnvoyConn::ToEnvoyAckEvents(x) => protocol::ToEnvoy::ToEnvoyAckEvents(x),
		protocol::ToEnvoyConn::ToEnvoyTunnelMessage(x) => {
			let gateway_id = x.message_id.gateway_id;
			let request_id = x.message_id.request_id;
			let message_index = x.message_id.message_index;
			let message_kind = to_envoy_tunnel_message_kind_name(&x.message_kind);
			let inner_data_len = to_envoy_tunnel_message_inner_data_len(&x.message_kind);
			tracing::trace!(
				gateway_id = %tunnel_message_task::display_id(&gateway_id),
				request_id = %tunnel_message_task::display_id(&request_id),
				message_index,
				message_kind,
				inner_data_len,
				"decoded tunnel message from gateway"
			);
			tunnel_message_meta = Some((
				gateway_id,
				request_id,
				message_index,
				message_kind,
				inner_data_len,
			));
			let _ = conn
				.authorized_tunnel_routes
				.insert_async((x.message_id.gateway_id, x.message_id.request_id), ())
				.await;
			// Start the envoy-side actor wake timer for ToEnvoyWebSocketOpen so
			// ws_to_tunnel_task can observe the duration when the matching
			// ToRivetWebSocketOpen (or ToRivetWebSocketClose) reply arrives.
			// Arm the entry before sending on the WS so a fast reply cannot race
			// past the insert and observe nothing.
			if matches!(
				&x.message_kind,
				protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(_)
			) {
				let _ = conn
					.pending_websocket_opens
					.insert_async(
						(x.message_id.gateway_id, x.message_id.request_id),
						Instant::now(),
					)
					.await;
			}
			protocol::ToEnvoy::ToEnvoyTunnelMessage(x)
		}
	};

	// Forward raw message to WebSocket
	let serialized_msg =
		versioned::ToEnvoy::wrap_latest(to_client_msg).serialize(conn.protocol_version)?;
	let ws_msg = Message::Binary(serialized_msg.into());
	if let Some((gateway_id, request_id, message_index, message_kind, inner_data_len)) =
		&tunnel_message_meta
	{
		tracing::trace!(
			gateway_id = %tunnel_message_task::display_id(gateway_id),
			request_id = %tunnel_message_task::display_id(request_id),
			message_index,
			message_kind,
			inner_data_len,
			serialized_len = ws_msg.len(),
			"sending tunnel message to actor websocket"
		);
	}
	let _in_flight = ws_to_tunnel_task::WsResponseInFlightGuard::new();
	conn.ws_handle
		.send(ws_msg)
		.await
		.context("failed to send message to WebSocket")?;
	drop(_in_flight);
	if let Some((gateway_id, request_id, message_index, message_kind, inner_data_len)) =
		&tunnel_message_meta
	{
		tracing::trace!(
			gateway_id = %tunnel_message_task::display_id(gateway_id),
			request_id = %tunnel_message_task::display_id(request_id),
			message_index,
			message_kind,
			inner_data_len,
			"sent tunnel message to actor websocket"
		);
	}

	metrics::PROCESS_MSG_DURATION
		.with_label_values(&[conn.namespace_id.to_string().as_str(), &conn.pool_name])
		.observe(start.elapsed().as_secs_f64());
	metrics::MSG_PROCESSED_TOTAL
		.with_label_values(&[conn.namespace_id.to_string().as_str(), &conn.pool_name])
		.inc();

	Ok(false)
}

fn to_envoy_tunnel_message_kind_name(kind: &protocol::ToEnvoyTunnelMessageKind) -> &'static str {
	match kind {
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(_) => "ToEnvoyRequestStart",
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(_) => "ToEnvoyRequestChunk",
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(_) => "ToEnvoyRequestAbort",
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(_) => "ToEnvoyWebSocketOpen",
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(_) => "ToEnvoyWebSocketMessage",
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
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(_)
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(_)
		| protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(_) => 0,
	}
}
