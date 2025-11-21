use anyhow::Context;
use futures_util::TryStreamExt;
use gas::prelude::Id;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message as WsMessage;
use hyper_tungstenite::tungstenite::Message;
use pegboard::pubsub_subjects::GatewayReceiverSubject;
use pegboard_actor_kv as kv;
use rivet_guard_core::websocket_handle::WebSocketReceiver;
use rivet_runner_protocol::{self as protocol, PROTOCOL_VERSION, versioned};
use std::sync::{Arc, atomic::Ordering};
use tokio::sync::{Mutex, watch};
use universalpubsub::PublishOpts;
use universalpubsub::Subscriber;
use vbare::OwnedVersionedData;

use crate::{LifecycleResult, conn::Conn, errors};

#[tracing::instrument(skip_all, fields(runner_id=?conn.runner_id, workflow_id=?conn.workflow_id, protocol_version=%conn.protocol_version))]
pub async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	ws_rx: Arc<Mutex<WebSocketReceiver>>,
	mut eviction_sub2: Subscriber,
	mut ws_to_tunnel_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	let mut ws_rx = ws_rx.lock().await;

	loop {
		let msg = tokio::select! {
			res = ws_rx.try_next() => {
				if let Some(msg) = res? {
					msg
				} else {
					tracing::debug!("websocket closed");
					return Ok(LifecycleResult::Closed);
				}
			}
			_ = eviction_sub2.next() => {
				tracing::debug!("runner evicted");
				return Err(errors::WsError::Eviction.build());
			}
			_ = ws_to_tunnel_abort_rx.changed() => {
				tracing::debug!("task aborted");
				return Ok(LifecycleResult::Aborted);
			}
		};

		match msg {
			WsMessage::Binary(data) => {
				tracing::trace!(
					data_len = data.len(),
					"received binary message from WebSocket"
				);

				// HACK: Decode v2 to handle tunnel ack
				if rivet_runner_protocol::compat::version_needs_tunnel_ack(conn.protocol_version) {
					match compat_ack_tunnel_message(&conn, &data[..]).await {
						Ok(_) => {}
						Err(err) => {
							tracing::error!(?err, "failed to send compat ack tunnel message")
						}
					}
				}

				// Parse message
				let msg = match versioned::ToServer::deserialize(&data, conn.protocol_version) {
					Ok(x) => x,
					Err(err) => {
						tracing::warn!(
							?err,
							data_len = data.len(),
							"failed to deserialize message"
						);
						continue;
					}
				};
				tracing::debug!(?msg, "received runner message from client");

				handle_message(&ctx, &conn, msg)
					.await
					.context("failed to handle WebSocket message")?;
			}
			WsMessage::Close(_) => {
				tracing::debug!("websocket closed");
				return Ok(LifecycleResult::Closed);
			}
			_ => {
				// Ignore other message types
			}
		}
	}
}

#[tracing::instrument(skip_all)]
async fn handle_message(
	ctx: &StandaloneCtx,
	conn: &Arc<Conn>,
	msg: protocol::ToServer,
) -> Result<()> {
	match msg {
		protocol::ToServer::ToServerPing(ping) => {
			let now = util::timestamp::now();

			let delta = if ping.ts <= now {
				// Calculate delta, clamping to u32::MAX if too large
				let delta_ms = now.saturating_sub(ping.ts);
				delta_ms.min(u32::MAX as i64) as u32
			} else {
				// If ping timestamp is in the future (clock skew), default to 0
				tracing::warn!(
					ping_ts = ping.ts,
					now_ts = now,
					"ping timestamp is in the future, possibly due to clock skew"
				);
				0
			};

			// Assuming symmetric delta
			let rtt = delta * 2;

			conn.last_rtt.store(rtt, Ordering::Relaxed);
		}
		// Process KV request
		protocol::ToServer::ToServerKvRequest(req) => {
			let actor_id = match Id::parse(&req.actor_id) {
				Ok(actor_id) => actor_id,
				Err(err) => {
					let res_msg = versioned::ToClient::wrap_latest(
						protocol::ToClient::ToClientKvResponse(protocol::ToClientKvResponse {
							request_id: req.request_id,
							data: protocol::KvResponseData::KvErrorResponse(
								protocol::KvErrorResponse {
									message: err.to_string(),
								},
							),
						}),
					);

					let res_msg_serialized = res_msg
						.serialize(conn.protocol_version)
						.context("failed to serialize KV error response")?;
					conn.ws_handle
						.send(Message::Binary(res_msg_serialized.into()))
						.await
						.context("failed to send KV error response to client")?;

					return Ok(());
				}
			};

			let actors_res = ctx
				.op(pegboard::ops::actor::get_runner::Input {
					actor_ids: vec![actor_id],
				})
				.await
				.with_context(|| format!("failed to get runner for actor: {}", actor_id))?;
			let actor_belongs = actors_res
				.actors
				.first()
				.map(|x| x.runner_id == conn.runner_id)
				.unwrap_or_default();

			// Verify actor belongs to this runner
			if !actor_belongs {
				let res_msg = versioned::ToClient::wrap_latest(
					protocol::ToClient::ToClientKvResponse(protocol::ToClientKvResponse {
						request_id: req.request_id,
						data: protocol::KvResponseData::KvErrorResponse(
							protocol::KvErrorResponse {
								message: "given actor does not belong to runner".to_string(),
							},
						),
					}),
				);

				let res_msg_serialized = res_msg
					.serialize(conn.protocol_version)
					.context("failed to serialize KV actor validation error")?;
				conn.ws_handle
					.send(Message::Binary(res_msg_serialized.into()))
					.await
					.context("failed to send KV actor validation error to client")?;

				return Ok(());
			}

			// TODO: Add queue and bg thread for processing kv ops
			// Run kv operation
			match req.data {
				protocol::KvRequestData::KvGetRequest(body) => {
					let res = kv::get(&*ctx.udb()?, actor_id, body.keys).await;

					let res_msg = versioned::ToClient::wrap_latest(
						protocol::ToClient::ToClientKvResponse(protocol::ToClientKvResponse {
							request_id: req.request_id,
							data: match res {
								Ok((keys, values, metadata)) => {
									protocol::KvResponseData::KvGetResponse(
										protocol::KvGetResponse {
											keys,
											values,
											metadata,
										},
									)
								}
								Err(err) => protocol::KvResponseData::KvErrorResponse(
									protocol::KvErrorResponse {
										// TODO: Don't return actual error?
										message: err.to_string(),
									},
								),
							},
						}),
					);

					let res_msg_serialized = res_msg
						.serialize(conn.protocol_version)
						.context("failed to serialize KV get response")?;
					conn.ws_handle
						.send(Message::Binary(res_msg_serialized.into()))
						.await
						.context("failed to send KV get response to client")?;
				}
				protocol::KvRequestData::KvListRequest(body) => {
					let res = kv::list(
						&*ctx.udb()?,
						actor_id,
						body.query,
						body.reverse.unwrap_or_default(),
						body.limit
							.map(TryInto::try_into)
							.transpose()
							.context("KV list limit value overflow")?,
					)
					.await;

					let res_msg = versioned::ToClient::wrap_latest(
						protocol::ToClient::ToClientKvResponse(protocol::ToClientKvResponse {
							request_id: req.request_id,
							data: match res {
								Ok((keys, values, metadata)) => {
									protocol::KvResponseData::KvListResponse(
										protocol::KvListResponse {
											keys,
											values,
											metadata,
										},
									)
								}
								Err(err) => protocol::KvResponseData::KvErrorResponse(
									protocol::KvErrorResponse {
										// TODO: Don't return actual error?
										message: err.to_string(),
									},
								),
							},
						}),
					);

					let res_msg_serialized = res_msg
						.serialize(conn.protocol_version)
						.context("failed to serialize KV list response")?;
					conn.ws_handle
						.send(Message::Binary(res_msg_serialized.into()))
						.await
						.context("failed to send KV list response to client")?;
				}
				protocol::KvRequestData::KvPutRequest(body) => {
					let res = kv::put(&*ctx.udb()?, actor_id, body.keys, body.values).await;

					let res_msg = versioned::ToClient::wrap_latest(
						protocol::ToClient::ToClientKvResponse(protocol::ToClientKvResponse {
							request_id: req.request_id,
							data: match res {
								Ok(()) => protocol::KvResponseData::KvPutResponse,
								Err(err) => {
									protocol::KvResponseData::KvErrorResponse(
										protocol::KvErrorResponse {
											// TODO: Don't return actual error?
											message: err.to_string(),
										},
									)
								}
							},
						}),
					);

					let res_msg_serialized = res_msg
						.serialize(conn.protocol_version)
						.context("failed to serialize KV put response")?;
					conn.ws_handle
						.send(Message::Binary(res_msg_serialized.into()))
						.await
						.context("failed to send KV put response to client")?;
				}
				protocol::KvRequestData::KvDeleteRequest(body) => {
					let res = kv::delete(&*ctx.udb()?, actor_id, body.keys).await;

					let res_msg = versioned::ToClient::wrap_latest(
						protocol::ToClient::ToClientKvResponse(protocol::ToClientKvResponse {
							request_id: req.request_id,
							data: match res {
								Ok(()) => protocol::KvResponseData::KvDeleteResponse,
								Err(err) => protocol::KvResponseData::KvErrorResponse(
									protocol::KvErrorResponse {
										// TODO: Don't return actual error?
										message: err.to_string(),
									},
								),
							},
						}),
					);

					let res_msg_serialized = res_msg
						.serialize(conn.protocol_version)
						.context("failed to serialize KV delete response")?;
					conn.ws_handle
						.send(Message::Binary(res_msg_serialized.into()))
						.await
						.context("failed to send KV delete response to client")?;
				}
				protocol::KvRequestData::KvDropRequest => {
					let res = kv::delete_all(&*ctx.udb()?, actor_id).await;

					let res_msg = versioned::ToClient::wrap_latest(
						protocol::ToClient::ToClientKvResponse(protocol::ToClientKvResponse {
							request_id: req.request_id,
							data: match res {
								Ok(()) => protocol::KvResponseData::KvDropResponse,
								Err(err) => protocol::KvResponseData::KvErrorResponse(
									protocol::KvErrorResponse {
										// TODO: Don't return actual error?
										message: err.to_string(),
									},
								),
							},
						}),
					);

					let res_msg_serialized = res_msg
						.serialize(conn.protocol_version)
						.context("failed to serialize KV drop response")?;
					conn.ws_handle
						.send(Message::Binary(res_msg_serialized.into()))
						.await
						.context("failed to send KV drop response to client")?;
				}
			}
		}
		protocol::ToServer::ToServerTunnelMessage(tunnel_msg) => {
			handle_tunnel_message(&ctx, tunnel_msg)
				.await
				.context("failed to handle tunnel message")?;
		}
		// Forward to runner wf
		protocol::ToServer::ToServerInit(_)
		| protocol::ToServer::ToServerEvents(_)
		| protocol::ToServer::ToServerAckCommands(_)
		| protocol::ToServer::ToServerStopping => {
			ctx.signal(pegboard::workflows::runner::Forward {
				inner: protocol::ToServer::try_from(msg)
					.context("failed to convert message for workflow forwarding")?,
			})
			.to_workflow_id(conn.workflow_id)
			.send()
			.await
			.with_context(|| {
				format!("failed to forward signal to workflow: {}", conn.workflow_id)
			})?;
		}
	}

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn handle_tunnel_message(
	ctx: &StandaloneCtx,
	msg: protocol::ToServerTunnelMessage,
) -> Result<()> {
	// Ignore DeprecatedTunnelAck messages (used only for backwards compatibility)
	if matches!(
		msg.message_kind,
		protocol::ToServerTunnelMessageKind::DeprecatedTunnelAck
	) {
		return Ok(());
	}

	// Publish message to UPS
	let gateway_reply_to = GatewayReceiverSubject::new(msg.message_id.gateway_id).to_string();
	let msg_serialized =
		versioned::ToGateway::wrap_latest(protocol::ToGateway::ToServerTunnelMessage(msg))
			.serialize_with_embedded_version(PROTOCOL_VERSION)
			.context("failed to serialize tunnel message for gateway")?;
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

	Ok(())
}

/// Send ack message for deprecated tunnel versions.
///
/// We have to parse as specifically a v2 message since we need the exact request & message ID
/// provided by the user and not available in v3.
async fn compat_ack_tunnel_message(conn: &Arc<Conn>, payload: &[u8]) -> Result<()> {
	use rivet_runner_protocol::generated::v2 as protocol_v2;

	// Parse payload
	let msg = serde_bare::from_slice::<protocol_v2::ToServer>(&payload)?;
	let protocol_v2::ToServer::ToServerTunnelMessage(msg) = msg else {
		return Ok(());
	};

	tracing::debug!(?msg.request_id, ?msg.message_id, "sending v2 compat tunnel ack");

	// Serialize response
	let ack_msg = serde_bare::to_vec(&protocol_v2::ToClient::ToClientTunnelMessage(
		protocol_v2::ToClientTunnelMessage {
			request_id: msg.request_id,
			message_id: msg.message_id,
			message_kind: protocol_v2::ToClientTunnelMessageKind::TunnelAck,
			gateway_reply_to: None,
		},
	))?;

	conn.ws_handle
		.send(hyper_tungstenite::tungstenite::Message::Binary(
			ack_msg.into(),
		))
		.await
		.context("failed to send DeprecatedTunnelAck to runner")?;

	Ok(())
}
