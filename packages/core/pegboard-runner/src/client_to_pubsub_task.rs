use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use gas::prelude::Id;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message as WsMessage;
use hyper_tungstenite::tungstenite::Message;
use pegboard_actor_kv as kv;
use rivet_guard_core::websocket_handle::WebSocketReceiver;
use rivet_runner_protocol::{self as protocol, PROTOCOL_VERSION, versioned};
use std::sync::{Arc, atomic::Ordering};
use universalpubsub::PublishOpts;
use vbare::OwnedVersionedData as _;

use crate::{
	conn::Conn,
	utils::{self},
};

pub async fn task(ctx: StandaloneCtx, conn: Arc<Conn>, ws_rx: WebSocketReceiver) {
	match task_inner(ctx, conn, ws_rx).await {
		Ok(_) => {}
		Err(err) => {
			tracing::error!(?err, "client to pubsub errored");
		}
	}
}

async fn task_inner(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	mut ws_rx: WebSocketReceiver,
) -> Result<()> {
	tracing::debug!("starting WebSocket to pubsub forwarding task");
	while let Some(msg) = ws_rx.next().await {
		match msg {
			Result::Ok(WsMessage::Binary(data)) => {
				tracing::trace!(
					data_len = data.len(),
					"received binary message from WebSocket"
				);

				// Parse message
				let msg =
					match versioned::ToServer::deserialize_version(&data, conn.protocol_version)
						.and_then(|x| x.into_latest())
					{
						Result::Ok(x) => x,
						Err(err) => {
							tracing::warn!(
								?err,
								data_len = data.len(),
								"failed to deserialize message"
							);
							continue;
						}
					};

				handle_message(&ctx, &conn, msg)
					.await
					.context("failed to handle WebSocket message")?;
			}
			Result::Ok(WsMessage::Close(_)) => {
				tracing::debug!(?conn.runner_id, "WebSocket closed");
				break;
			}
			Result::Ok(_) => {
				// Ignore other message types
			}
			Err(e) => {
				tracing::error!(?e, "WebSocket error");
				break;
			}
		}
	}
	tracing::debug!("WebSocket to pubsub forwarding task ended");

	Ok(())
}

async fn handle_message(
	ctx: &StandaloneCtx,
	conn: &Arc<Conn>,
	msg: protocol::ToServer,
) -> Result<()> {
	match msg {
		protocol::ToServer::ToServerPing(ping) => {
			let rtt = util::timestamp::now()
				.saturating_sub(ping.ts)
				.try_into()
				.context("failed to calculate RTT from ping timestamp")?;

			conn.last_rtt.store(rtt, Ordering::Relaxed);
		}
		// Process KV request
		protocol::ToServer::ToServerKvRequest(req) => {
			let actor_id = match Id::parse(&req.actor_id) {
				Ok(actor_id) => actor_id,
				Err(err) => {
					let res_msg = versioned::ToClient::latest(
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
				let res_msg = versioned::ToClient::latest(protocol::ToClient::ToClientKvResponse(
					protocol::ToClientKvResponse {
						request_id: req.request_id,
						data: protocol::KvResponseData::KvErrorResponse(
							protocol::KvErrorResponse {
								message: "given actor does not belong to runner".to_string(),
							},
						),
					},
				));

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

					let res_msg = versioned::ToClient::latest(
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

					let res_msg = versioned::ToClient::latest(
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

					let res_msg = versioned::ToClient::latest(
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

					let res_msg = versioned::ToClient::latest(
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

					let res_msg = versioned::ToClient::latest(
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
			handle_tunnel_message(&ctx, &conn, tunnel_msg)
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

async fn handle_tunnel_message(
	ctx: &StandaloneCtx,
	conn: &Arc<Conn>,
	msg: protocol::ToServerTunnelMessage,
) -> Result<()> {
	// Determine reply to subject
	let request_id = msg.request_id;
	let gateway_reply_to = {
		let active_requests = conn.tunnel_active_requests.lock().await;
		if let Some(req) = active_requests.get(&request_id) {
			req.gateway_reply_to.clone()
		} else {
			tracing::warn!("no active request for tunnel message, may have timed out");
			return Ok(());
		}
	};

	// Remove active request entries when terminal
	if utils::is_to_server_tunnel_message_kind_request_close(&msg.message_kind) {
		let mut active_requests = conn.tunnel_active_requests.lock().await;
		active_requests.remove(&request_id);
	}

	// Publish message to UPS
	let msg_serialized = versioned::ToGateway::latest(protocol::ToGateway { message: msg })
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
