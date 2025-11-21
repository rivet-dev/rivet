use anyhow::Result;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message as WsMessage;
use rivet_runner_protocol::{self as protocol, versioned};
use std::sync::Arc;
use tokio::sync::watch;
use universalpubsub::{NextOutput, Subscriber};
use vbare::OwnedVersionedData;

use crate::{LifecycleResult, conn::Conn, errors};

#[tracing::instrument(skip_all, fields(runner_id=?conn.runner_id, workflow_id=?conn.workflow_id, protocol_version=%conn.protocol_version))]
pub async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	mut sub: Subscriber,
	mut eviction_sub: Subscriber,
	mut tunnel_to_ws_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	loop {
		let ups_msg = tokio::select! {
			res = sub.next() => {
				if let NextOutput::Message(ups_msg) = res.context("pubsub_to_client_task sub failed")? {
					ups_msg
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
				return Ok(LifecycleResult::Aborted);
			}
		};

		tracing::debug!(
			payload_len = ups_msg.payload.len(),
			"received message from pubsub, forwarding to WebSocket"
		);

		// Parse message
		let msg = match versioned::ToRunner::deserialize_with_embedded_version(&ups_msg.payload) {
			Result::Ok(x) => x,
			Err(err) => {
				tracing::error!(?err, "failed to parse tunnel message");
				continue;
			}
		};

		// Convert to ToClient types
		let to_client_msg = match msg {
			protocol::ToRunner::ToRunnerKeepAlive(_) => {
				// TODO:
				continue;
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
		let serialized_msg = match versioned::ToClient::wrap_latest(to_client_msg)
			.serialize(conn.protocol_version)
		{
			Result::Ok(x) => x,
			Err(err) => {
				tracing::error!(?err, "failed to serialize tunnel message");
				continue;
			}
		};
		let ws_msg = WsMessage::Binary(serialized_msg.into());
		conn.ws_handle
			.send(ws_msg)
			.await
			.context("failed to send message to WebSocket")?
	}
}
