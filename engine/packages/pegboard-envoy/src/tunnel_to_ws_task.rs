use anyhow::Result;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use pegboard::pubsub_subjects::GatewayReceiverSubject;
use rivet_envoy_protocol::{self as protocol, PROTOCOL_VERSION, versioned};
use std::sync::Arc;
use tokio::sync::watch;
use universalpubsub as ups;
use universalpubsub::{NextOutput, PublishOpts, Subscriber};
use vbare::OwnedVersionedData;

use crate::{LifecycleResult, conn::Conn, metrics};

#[tracing::instrument(name="tunnel_to_ws_task", skip_all, fields(ray_id=?ctx.ray_id(), req_id=?ctx.req_id(), envoy_key=%conn.envoy_key, protocol_version=%conn.protocol_version))]
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

async fn handle_message(
	ctx: &StandaloneCtx,
	conn: &Conn,
	tunnel_msg: ups::Message,
) -> Result<bool> {
	// Parse message
	let msg = match versioned::ToEnvoyConn::deserialize_with_embedded_version(&tunnel_msg.payload) {
		Result::Ok(x) => x,
		Err(err) => {
			tracing::error!(?err, "failed to parse tunnel message");
			return Ok(false);
		}
	};

	// Convert to ToEnvoy types
	let to_client_msg = match msg {
		protocol::ToEnvoyConn::ToEnvoyConnPing(ping) => {
			// Publish pong to UPS
			let gateway_reply_to = GatewayReceiverSubject::new(ping.gateway_id).to_string();
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
				if let protocol::Command::CommandStartActor(start) =
					&mut command_wrapper.inner
				{
					let actor_id = Id::parse(&command_wrapper.checkpoint.actor_id)?;
					let actor_name = start.config.name.clone();
					let ids = ctx
						.op(pegboard::ops::actor::hibernating_request::list::Input {
							actor_id,
						})
						.await?;

					// Dynamically populate hibernating request ids
					start.hibernating_requests = ids
						.into_iter()
						.map(|x| protocol::HibernatingRequest {
							gateway_id: x.gateway_id,
							request_id: x.request_id,
						})
						.collect();

					if start.preloaded_kv.is_none() {
						let db = ctx.udb()?;
						start.preloaded_kv =
							pegboard::actor_kv::preload::fetch_preloaded_kv(
								&db,
								ctx.config().pegboard(),
								actor_id,
								conn.namespace_id,
								&actor_name,
							)
							.await?;
					}
				}
			}

			// NOTE: `command_wrappers` is mutated in this match arm, it is not the same as the
			// ToEnvoyConn data
			protocol::ToEnvoy::ToEnvoyCommands(command_wrappers)
		}
		protocol::ToEnvoyConn::ToEnvoyAckEvents(x) => protocol::ToEnvoy::ToEnvoyAckEvents(x),
		protocol::ToEnvoyConn::ToEnvoyTunnelMessage(x) => {
			protocol::ToEnvoy::ToEnvoyTunnelMessage(x)
		}
	};

	// Forward raw message to WebSocket
	let serialized_msg =
		versioned::ToEnvoy::wrap_latest(to_client_msg).serialize(conn.protocol_version)?;
	let ws_msg = Message::Binary(serialized_msg.into());
	conn.ws_handle
		.send(ws_msg)
		.await
		.context("failed to send message to WebSocket")?;

	Ok(false)
}
