use anyhow::Result;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use pegboard::pubsub_subjects::GatewayReceiverSubject;
use rivet_runner_protocol::{self as protocol, PROTOCOL_MK2_VERSION, versioned};
use std::sync::Arc;
use tokio::sync::watch;
use universalpubsub as ups;
use universalpubsub::{NextOutput, PublishOpts, Subscriber};
use vbare::OwnedVersionedData;

use crate::{LifecycleResult, conn::Conn, errors};

#[tracing::instrument(skip_all, fields(runner_id=?conn.runner_id, workflow_id=?conn.workflow_id, protocol_version=%conn.protocol_version))]
pub async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	mut tunnel_sub: Subscriber,
	mut eviction_sub: Subscriber,
	mut tunnel_to_ws_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	loop {
		match recv_msg(
			&mut tunnel_sub,
			&mut eviction_sub,
			&mut tunnel_to_ws_abort_rx,
		)
		.await?
		{
			Ok(msg) => {
				if protocol::is_mk2(conn.protocol_version) {
					handle_message_mk2(&ctx, &conn, msg).await?;
				} else {
					handle_message_mk1(&ctx, &conn, msg).await?;
				}
			}
			Err(lifecycle_res) => return Ok(lifecycle_res),
		}
	}
}

async fn recv_msg(
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
			tracing::debug!("runner evicted");
			return Err(errors::WsError::Eviction.build());
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

async fn handle_message_mk2(
	ctx: &StandaloneCtx,
	conn: &Conn,
	tunnel_msg: ups::Message,
) -> Result<()> {
	// Parse message
	let msg = match versioned::ToRunnerMk2::deserialize_with_embedded_version(&tunnel_msg.payload) {
		Result::Ok(x) => x,
		Err(err) => {
			tracing::error!(?err, "failed to parse tunnel message");
			return Ok(());
		}
	};

	// Convert to ToClient types
	let to_client_msg = match msg {
		protocol::mk2::ToRunner::ToRunnerPing(ping) => {
			// Publish pong to UPS
			let gateway_reply_to = GatewayReceiverSubject::new(ping.gateway_id).to_string();
			let msg_serialized = versioned::ToGateway::wrap_latest(
				protocol::mk2::ToGateway::ToGatewayPong(protocol::mk2::ToGatewayPong {
					request_id: ping.request_id,
					ts: ping.ts,
				}),
			)
			.serialize_with_embedded_version(PROTOCOL_MK2_VERSION)
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

			// Not sent to client
			return Ok(());
		}
		protocol::mk2::ToRunner::ToRunnerClose => return Err(errors::WsError::Eviction.build()),
		protocol::mk2::ToRunner::ToClientCommands(mut command_wrappers) => {
			for command_wrapper in &mut command_wrappers {
				if let protocol::mk2::Command::CommandStartActor(
					protocol::mk2::CommandStartActor {
						hibernating_requests,
						..
					},
				) = &mut command_wrapper.inner
				{
					let ids = ctx
						.op(pegboard::ops::actor::hibernating_request::list::Input {
							actor_id: Id::parse(&command_wrapper.checkpoint.actor_id)?,
						})
						.await?;

					// Dynamically populate hibernating request ids
					*hibernating_requests = ids
						.into_iter()
						.map(|x| protocol::mk2::HibernatingRequest {
							gateway_id: x.gateway_id,
							request_id: x.request_id,
						})
						.collect();
				}
			}

			// NOTE: `command_wrappers` is mutated in this match arm, it is not the same as the
			// ToRunner data
			protocol::mk2::ToClient::ToClientCommands(command_wrappers)
		}
		protocol::mk2::ToRunner::ToClientAckEvents(x) => {
			protocol::mk2::ToClient::ToClientAckEvents(x)
		}
		protocol::mk2::ToRunner::ToClientTunnelMessage(x) => {
			protocol::mk2::ToClient::ToClientTunnelMessage(x)
		}
	};

	// Forward raw message to WebSocket
	let serialized_msg =
		versioned::ToClientMk2::wrap_latest(to_client_msg).serialize(conn.protocol_version)?;
	let ws_msg = Message::Binary(serialized_msg.into());
	conn.ws_handle
		.send(ws_msg)
		.await
		.context("failed to send message to WebSocket")?;

	Ok(())
}

async fn handle_message_mk1(
	ctx: &StandaloneCtx,
	conn: &Conn,
	tunnel_msg: ups::Message,
) -> Result<()> {
	// Parse message
	let msg = match versioned::ToRunner::deserialize_with_embedded_version(&tunnel_msg.payload) {
		Result::Ok(x) => x,
		Err(err) => {
			tracing::error!(?err, "failed to parse tunnel message");
			return Ok(());
		}
	};

	// Convert to ToClient types
	let to_client_msg = match msg {
		protocol::ToRunner::ToRunnerPing(ping) => {
			// Publish pong to UPS
			let gateway_reply_to = GatewayReceiverSubject::new(ping.gateway_id).to_string();
			let msg_serialized = versioned::ToGateway::wrap_latest(
				protocol::mk2::ToGateway::ToGatewayPong(protocol::mk2::ToGatewayPong {
					request_id: ping.request_id,
					ts: ping.ts,
				}),
			)
			.serialize_with_embedded_version(PROTOCOL_MK2_VERSION)
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

			// Not sent to client
			return Ok(());
		}
		protocol::ToRunner::ToClientInit(x) => protocol::ToClient::ToClientInit(x),
		protocol::ToRunner::ToClientClose => return Err(errors::WsError::Eviction.build()),
		// Dynamically populate hibernating request ids
		protocol::ToRunner::ToClientCommands(mut command_wrappers) => {
			for command_wrapper in &mut command_wrappers {
				if let protocol::Command::CommandStartActor(protocol::CommandStartActor {
					actor_id,
					hibernating_requests,
					..
				}) = &mut command_wrapper.inner
				{
					let ids = ctx
						.op(pegboard::ops::actor::hibernating_request::list::Input {
							actor_id: Id::parse(actor_id)?,
						})
						.await?;

					*hibernating_requests = ids
						.into_iter()
						.map(|x| protocol::HibernatingRequest {
							gateway_id: x.gateway_id,
							request_id: x.request_id,
						})
						.collect();
				}
			}

			// NOTE: `command_wrappers` is mutated in this match arm, it is not the same as the
			// ToRunner data
			protocol::ToClient::ToClientCommands(command_wrappers)
		}
		protocol::ToRunner::ToClientAckEvents(x) => protocol::ToClient::ToClientAckEvents(x),
		protocol::ToRunner::ToClientKvResponse(x) => protocol::ToClient::ToClientKvResponse(x),
		protocol::ToRunner::ToClientTunnelMessage(x) => {
			protocol::ToClient::ToClientTunnelMessage(x)
		}
	};

	// Forward raw message to WebSocket
	tracing::debug!(?to_client_msg, "sending runner message to client");
	let serialized_msg =
		versioned::ToClient::wrap_latest(to_client_msg).serialize(conn.protocol_version)?;
	let ws_msg = Message::Binary(serialized_msg.into());
	conn.ws_handle
		.send(ws_msg)
		.await
		.context("failed to send message to WebSocket")?;

	Ok(())
}
