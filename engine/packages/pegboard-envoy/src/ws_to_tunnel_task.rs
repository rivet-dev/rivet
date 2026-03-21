use anyhow::Context;
use bytes::Bytes;
use futures_util::TryStreamExt;
use gas::prelude::Id;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use pegboard::actor_kv;
use pegboard::pubsub_subjects::GatewayReceiverSubject;
use rivet_envoy_protocol::{self as protocol, PROTOCOL_VERSION, versioned};
use rivet_guard_core::websocket_handle::WebSocketReceiver;
use std::sync::{Arc, atomic::Ordering};
use tokio::sync::{Mutex, MutexGuard, watch};
use universaldb::utils::end_of_key_range;
use universalpubsub::PublishOpts;
use vbare::OwnedVersionedData;

use crate::{LifecycleResult, actor_event_demuxer::ActorEventDemuxer, conn::Conn, errors};

#[tracing::instrument(name="ws_to_tunnel_task", skip_all, fields(ray_id=?ctx.ray_id(), req_id=?ctx.req_id(), envoy_key=%conn.envoy_key, protocol_version=%conn.protocol_version))]
pub async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	ws_rx: Arc<Mutex<WebSocketReceiver>>,
	ws_to_tunnel_abort_rx: watch::Receiver<()>,
) -> Result<LifecycleResult> {
	let mut event_demuxer = ActorEventDemuxer::new(ctx.clone(), conn.envoy_key.clone());

	let res = task_inner(ctx, conn, ws_rx, ws_to_tunnel_abort_rx, &mut event_demuxer).await;

	// Must shutdown demuxer to allow for all in-flight events to finish
	event_demuxer.shutdown().await;

	res
}

#[tracing::instrument(skip_all, fields(envoy_key=%conn.envoy_key, protocol_version=%conn.protocol_version))]
pub async fn task_inner(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	ws_rx: Arc<Mutex<WebSocketReceiver>>,
	mut ws_to_tunnel_abort_rx: watch::Receiver<()>,
	event_demuxer: &mut ActorEventDemuxer,
) -> Result<LifecycleResult> {
	let mut ws_rx = ws_rx.lock().await;
	let mut term_signal = rivet_runtime::TermSignal::get();

	loop {
		match recv_msg(&mut ws_rx, &mut ws_to_tunnel_abort_rx, &mut term_signal).await? {
			Ok(Some(msg)) => {
				handle_message(&ctx, &conn, event_demuxer, msg).await?;
			}
			Ok(None) => {}
			Err(lifecycle_res) => return Ok(lifecycle_res),
		}
	}
}

async fn recv_msg(
	ws_rx: &mut MutexGuard<'_, WebSocketReceiver>,
	ws_to_tunnel_abort_rx: &mut watch::Receiver<()>,
	term_signal: &mut rivet_runtime::TermSignal,
) -> Result<std::result::Result<Option<Bytes>, LifecycleResult>> {
	let msg = tokio::select! {
		res = ws_rx.try_next() => {
			if let Some(msg) = res? {
				msg
			} else {
				tracing::debug!("websocket closed");
				return Ok(Err(LifecycleResult::Closed));
			}
		}
		_ = ws_to_tunnel_abort_rx.changed() => {
			tracing::debug!("task aborted");
			return Ok(Err(LifecycleResult::Aborted));
		}
		_ = term_signal.recv() => {
			return Err(errors::WsError::GoingAway.build());
		}
	};

	match msg {
		Message::Binary(data) => {
			tracing::trace!(
				data_len = data.len(),
				"received binary message from WebSocket"
			);

			Ok(Ok(Some(data)))
		}
		Message::Close(_) => {
			tracing::debug!("websocket closed");
			return Ok(Err(LifecycleResult::Closed));
		}
		_ => {
			// Ignore other message types
			Ok(Ok(None))
		}
	}
}

#[tracing::instrument(skip_all)]
async fn handle_message(
	ctx: &StandaloneCtx,
	conn: &Conn,
	event_demuxer: &mut ActorEventDemuxer,
	msg: Bytes,
) -> Result<()> {
	// Parse message
	let msg = match versioned::ToRivet::deserialize(&msg, conn.protocol_version) {
		Ok(x) => x,
		Err(err) => {
			tracing::warn!(?err, msg_len = msg.len(), "failed to deserialize message");
			return Ok(());
		}
	};

	tracing::debug!(?msg, "received message from envoy");

	match msg {
		protocol::ToRivet::ToRivetPong(pong) => {
			let now = util::timestamp::now();
			let rtt = now.saturating_sub(pong.ts);

			let rtt = if let Ok(rtt) = u32::try_from(rtt) {
				rtt
			} else {
				tracing::debug!("ping ts in the future, ignoring");
				u32::MAX
			};

			conn.last_rtt.store(rtt, Ordering::Relaxed);
			conn.last_ping_ts
				.store(util::timestamp::now(), Ordering::Relaxed);
		}
		// Process KV request
		protocol::ToRivet::ToRivetKvRequest(req) => {
			let actor_id = match Id::parse(&req.actor_id) {
				Ok(actor_id) => actor_id,
				Err(err) => {
					let res_msg = versioned::ToEnvoy::wrap_latest(
						protocol::ToEnvoy::ToEnvoyKvResponse(protocol::ToEnvoyKvResponse {
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

			let actor_res = ctx
				.op(pegboard::ops::actor::get_for_envoy::Input { actor_id })
				.await
				.with_context(|| format!("failed to get envoy for actor: {}", actor_id))?;

			let Some(actor) = actor_res else {
				send_actor_kv_error(&conn, req.request_id, "actor does not exist").await?;
				return Ok(());
			};

			// Verify actor belongs to this envoy
			if actor.namespace_id != conn.namespace_id || actor.envoy_key != conn.envoy_key {
				send_actor_kv_error(&conn, req.request_id, "actor does not belong to envoy")
					.await?;
				return Ok(());
			}

			let recipient = actor_kv::Recipient {
				actor_id,
				namespace_id: conn.namespace_id,
				name: actor.name,
			};

			// TODO: Add queue and bg thread for processing kv ops
			// Run kv operation
			match req.data {
				protocol::KvRequestData::KvGetRequest(body) => {
					let res = actor_kv::get(&*ctx.udb()?, &recipient, body.keys).await;

					let res_msg = versioned::ToEnvoy::wrap_latest(
						protocol::ToEnvoy::ToEnvoyKvResponse(protocol::ToEnvoyKvResponse {
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
					let res = actor_kv::list(
						&*ctx.udb()?,
						&recipient,
						body.query,
						body.reverse.unwrap_or_default(),
						body.limit
							.map(TryInto::try_into)
							.transpose()
							.context("KV list limit value overflow")?,
					)
					.await;

					let res_msg = versioned::ToEnvoy::wrap_latest(
						protocol::ToEnvoy::ToEnvoyKvResponse(protocol::ToEnvoyKvResponse {
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
					let res = actor_kv::put(&*ctx.udb()?, &recipient, body.keys, body.values).await;

					let res_msg = versioned::ToEnvoy::wrap_latest(
						protocol::ToEnvoy::ToEnvoyKvResponse(protocol::ToEnvoyKvResponse {
							request_id: req.request_id,
							data: match res {
								Ok(()) => protocol::KvResponseData::KvPutResponse,
								Err(err) => protocol::KvResponseData::KvErrorResponse(
									protocol::KvErrorResponse {
										message: err.to_string(),
									},
								),
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
					let res = actor_kv::delete(&*ctx.udb()?, &recipient, body.keys).await;

					let res_msg = versioned::ToEnvoy::wrap_latest(
						protocol::ToEnvoy::ToEnvoyKvResponse(protocol::ToEnvoyKvResponse {
							request_id: req.request_id,
							data: match res {
								Ok(()) => protocol::KvResponseData::KvDeleteResponse,
								Err(err) => protocol::KvResponseData::KvErrorResponse(
									protocol::KvErrorResponse {
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
				protocol::KvRequestData::KvDeleteRangeRequest(body) => {
					let res =
						actor_kv::delete_range(&*ctx.udb()?, &recipient, body.start, body.end)
							.await;

					let res_msg = versioned::ToEnvoy::wrap_latest(
						protocol::ToEnvoy::ToEnvoyKvResponse(protocol::ToEnvoyKvResponse {
							request_id: req.request_id,
							data: match res {
								Ok(()) => protocol::KvResponseData::KvDeleteResponse,
								Err(err) => protocol::KvResponseData::KvErrorResponse(
									protocol::KvErrorResponse {
										message: err.to_string(),
									},
								),
							},
						}),
					);

					let res_msg_serialized = res_msg
						.serialize(conn.protocol_version)
						.context("failed to serialize KV delete range response")?;
					conn.ws_handle
						.send(Message::Binary(res_msg_serialized.into()))
						.await
						.context("failed to send KV delete range response to client")?;
				}
				protocol::KvRequestData::KvDropRequest => {
					let res = actor_kv::delete_all(&*ctx.udb()?, &recipient).await;

					let res_msg = versioned::ToEnvoy::wrap_latest(
						protocol::ToEnvoy::ToEnvoyKvResponse(protocol::ToEnvoyKvResponse {
							request_id: req.request_id,
							data: match res {
								Ok(()) => protocol::KvResponseData::KvDropResponse,
								Err(err) => protocol::KvResponseData::KvErrorResponse(
									protocol::KvErrorResponse {
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
		protocol::ToRivet::ToRivetTunnelMessage(tunnel_msg) => {
			handle_tunnel_message(&ctx, tunnel_msg)
				.await
				.context("failed to handle tunnel message")?;
		}
		// NOTE: Init event is processed in `conn::init_conn`
		protocol::ToRivet::ToRivetInit(_) => {
			tracing::debug!("received additional init packet, ignoring");
		}
		// Forward to demuxer which forwards to actor wf
		protocol::ToRivet::ToRivetEvents(events) => {
			for event in events {
				event_demuxer.ingest(Id::parse(&event.checkpoint.actor_id)?, event);
			}
		}
		protocol::ToRivet::ToRivetAckCommands(ack) => {
			ack_commands(&ctx, conn.namespace_id, &conn.envoy_key, ack).await?;
		}
		protocol::ToRivet::ToRivetStopping => {
			// For serverful, remove from lb
			if !conn.is_serverless {
				ctx.op(pegboard::ops::envoy::expire::Input {
					namespace_id: conn.namespace_id,
					envoy_key: conn.envoy_key.to_string(),
				})
				.await?;
			}

			// Evict all actors
			ctx.op(pegboard::ops::envoy::evict_actors::Input {
				namespace_id: conn.namespace_id,
				envoy_key: conn.envoy_key.to_string(),
			})
			.await?;
		}
	}

	Ok(())
}

async fn ack_commands(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	envoy_key: &str,
	ack: protocol::ToRivetAckCommands,
) -> Result<()> {
	ctx.udb()?
		.run(|tx| {
			let ack = ack.clone();
			async move {
				let tx = tx.with_subspace(pegboard::keys::subspace());

				for checkpoint in &ack.last_command_checkpoints {
					let start = tx.pack(
						&pegboard::keys::envoy::ActorCommandKey::subspace_with_actor(
							namespace_id,
							envoy_key.to_string(),
							Id::parse(&checkpoint.actor_id)?,
							checkpoint.generation,
						),
					);
					let end = end_of_key_range(&tx.pack(
						&pegboard::keys::envoy::ActorCommandKey::subspace_with_index(
							namespace_id,
							envoy_key.to_string(),
							Id::parse(&checkpoint.actor_id)?,
							checkpoint.generation,
							checkpoint.index,
						),
					));
					tx.clear_range(&start, &end);
				}

				Ok(())
			}
		})
		.await
}

#[tracing::instrument(skip_all)]
async fn handle_tunnel_message(
	ctx: &StandaloneCtx,
	msg: protocol::ToRivetTunnelMessage,
) -> Result<()> {
	// Extract inner data length before consuming msg
	let inner_data_len = tunnel_message_inner_data_len(&msg.message_kind);

	// Enforce incoming payload size
	if inner_data_len
		> ctx
			.config()
			.pegboard()
			.envoy_max_response_payload_body_size()
	{
		return Err(errors::WsError::InvalidPacket("payload too large".to_string()).build());
	}

	let gateway_reply_to = GatewayReceiverSubject::new(msg.message_id.gateway_id).to_string();
	let msg_serialized =
		versioned::ToGateway::wrap_latest(protocol::ToGateway::ToRivetTunnelMessage(msg))
			.serialize_with_embedded_version(PROTOCOL_VERSION)
			.context("failed to serialize tunnel message for gateway")?;

	tracing::trace!(
		inner_data_len = inner_data_len,
		serialized_len = msg_serialized.len(),
		"publishing tunnel message to gateway"
	);

	// Publish message to UPS
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

/// Returns the length of the inner data payload for a tunnel message kind.
fn tunnel_message_inner_data_len(kind: &protocol::ToRivetTunnelMessageKind) -> usize {
	use protocol::ToRivetTunnelMessageKind;
	match kind {
		ToRivetTunnelMessageKind::ToRivetResponseStart(resp) => {
			resp.body.as_ref().map_or(0, |b| b.len())
		}
		ToRivetTunnelMessageKind::ToRivetResponseChunk(chunk) => chunk.body.len(),
		ToRivetTunnelMessageKind::ToRivetWebSocketMessage(msg) => msg.data.len(),
		ToRivetTunnelMessageKind::ToRivetResponseAbort
		| ToRivetTunnelMessageKind::ToRivetWebSocketOpen(_)
		| ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(_)
		| ToRivetTunnelMessageKind::ToRivetWebSocketClose(_) => 0,
	}
}

async fn send_actor_kv_error(conn: &Conn, request_id: u32, message: &str) -> Result<()> {
	let res_msg = versioned::ToEnvoy::wrap_latest(protocol::ToEnvoy::ToEnvoyKvResponse(
		protocol::ToEnvoyKvResponse {
			request_id,
			data: protocol::KvResponseData::KvErrorResponse(protocol::KvErrorResponse {
				message: message.to_string(),
			}),
		},
	));

	let res_msg_serialized = res_msg
		.serialize(conn.protocol_version)
		.context("failed to serialize KV actor validation error")?;
	conn.ws_handle
		.send(Message::Binary(res_msg_serialized.into()))
		.await
		.context("failed to send KV actor validation error to client")?;

	Ok(())
}
