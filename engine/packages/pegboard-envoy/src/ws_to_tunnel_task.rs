use anyhow::{Context, bail, ensure};
use bytes::Bytes;
use futures_util::TryStreamExt;
use gas::prelude::Id;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use pegboard::actor_kv;
use pegboard::pubsub_subjects::GatewayReceiverSubject;
use rivet_data::converted::{ActorNameKeyData, MetadataKeyData};
use rivet_envoy_protocol::{self as protocol, PROTOCOL_VERSION, versioned};
use rivet_guard_core::websocket_handle::WebSocketReceiver;
use scc::HashMap;
use sqlite_storage::error::SqliteStorageError;
use std::{
	collections::BTreeSet,
	sync::{Arc, atomic::Ordering},
	time::Instant,
};
use tokio::sync::{Mutex, MutexGuard, watch};
use tracing::Instrument;
use universaldb::prelude::*;
use universaldb::utils::end_of_key_range;
use universalpubsub::PublishOpts;
use vbare::OwnedVersionedData;

use crate::{
	LifecycleResult, actor_event_demuxer::ActorEventDemuxer, conn::Conn, errors, sqlite_runtime,
};

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

			conn.last_rtt.store(rtt, Ordering::SeqCst);
			conn.last_ping_ts
				.store(util::timestamp::now(), Ordering::SeqCst);
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
				.op(pegboard::ops::actor::get_for_kv::Input { actor_id })
				.await
				.with_context(|| format!("failed to get envoy for actor: {}", actor_id))?;

			let Some(actor) = actor_res else {
				send_actor_kv_error(&conn, req.request_id, "actor does not exist").await?;
				return Ok(());
			};

			// Verify actor belongs to this namespace
			if actor.namespace_id != conn.namespace_id {
				send_actor_kv_error(&conn, req.request_id, "actor does not exist").await?;
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
		protocol::ToRivet::ToRivetSqliteGetPagesRequest(req) => {
			let response = handle_sqlite_get_pages_response(ctx, conn, req.data).await;
			send_sqlite_get_pages_response(conn, req.request_id, response).await?;
		}
		protocol::ToRivet::ToRivetSqliteCommitRequest(req) => {
			let actor_id = req.data.actor_id.clone();
			let request_id = req.request_id;
			let timed_response = async { handle_sqlite_commit_response(ctx, conn, req.data).await }
				.instrument(tracing::debug_span!(
					"handle_sqlite_commit",
					actor_id = %actor_id,
					request_id = ?request_id
				))
				.await;
			send_sqlite_commit_response(conn, request_id, timed_response.response).await?;
			crate::metrics::SQLITE_COMMIT_ENVOY_RESPONSE_DURATION
				.observe(timed_response.commit_completed_at.elapsed().as_secs_f64());
		}
		protocol::ToRivet::ToRivetSqliteCommitStageBeginRequest(req) => {
			let response = handle_sqlite_commit_stage_begin_response(ctx, conn, req.data).await;
			send_sqlite_commit_stage_begin_response(conn, req.request_id, response).await?;
		}
		protocol::ToRivet::ToRivetSqliteCommitStageRequest(req) => {
			let response = handle_sqlite_commit_stage_response(ctx, conn, req.data).await;
			send_sqlite_commit_stage_response(conn, req.request_id, response).await?;
		}
		protocol::ToRivet::ToRivetSqliteCommitFinalizeRequest(req) => {
			let response = handle_sqlite_commit_finalize_response(ctx, conn, req.data).await;
			send_sqlite_commit_finalize_response(conn, req.request_id, response).await?;
		}
		protocol::ToRivet::ToRivetTunnelMessage(tunnel_msg) => {
			handle_tunnel_message(ctx, &conn.authorized_tunnel_routes, tunnel_msg)
				.await
				.context("failed to handle tunnel message")?;
		}
		protocol::ToRivet::ToRivetMetadata(metadata) => {
			handle_metadata(&ctx, conn.namespace_id, &conn.envoy_key, metadata).await?;
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

struct TimedSqliteCommitResponse {
	response: protocol::SqliteCommitResponse,
	commit_completed_at: Instant,
}

async fn handle_sqlite_get_pages_response(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteGetPagesRequest,
) -> protocol::SqliteGetPagesResponse {
	let actor_id = request.actor_id.clone();
	match handle_sqlite_get_pages(ctx, conn, request).await {
		Ok(response) => response,
		Err(err) => {
			tracing::error!(actor_id = %actor_id, ?err, "sqlite get_pages request failed");
			protocol::SqliteGetPagesResponse::SqliteErrorResponse(sqlite_error_response(&err))
		}
	}
}

async fn handle_sqlite_commit_response(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteCommitRequest,
) -> TimedSqliteCommitResponse {
	let actor_id = request.actor_id.clone();
	let response = match handle_sqlite_commit(ctx, conn, request).await {
		Ok(response) => response,
		Err(err) => {
			tracing::error!(actor_id = %actor_id, ?err, "sqlite commit request failed");
			protocol::SqliteCommitResponse::SqliteErrorResponse(sqlite_error_response(&err))
		}
	};
	TimedSqliteCommitResponse {
		response,
		commit_completed_at: Instant::now(),
	}
}

async fn handle_sqlite_commit_stage_response(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteCommitStageRequest,
) -> protocol::SqliteCommitStageResponse {
	let actor_id = request.actor_id.clone();
	match handle_sqlite_commit_stage(ctx, conn, request).await {
		Ok(response) => response,
		Err(err) => {
			tracing::error!(actor_id = %actor_id, ?err, "sqlite commit_stage request failed");
			protocol::SqliteCommitStageResponse::SqliteErrorResponse(sqlite_error_response(&err))
		}
	}
}

async fn handle_sqlite_commit_stage_begin_response(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteCommitStageBeginRequest,
) -> protocol::SqliteCommitStageBeginResponse {
	let actor_id = request.actor_id.clone();
	match handle_sqlite_commit_stage_begin(ctx, conn, request).await {
		Ok(response) => response,
		Err(err) => {
			tracing::error!(actor_id = %actor_id, ?err, "sqlite commit_stage_begin request failed");
			protocol::SqliteCommitStageBeginResponse::SqliteErrorResponse(sqlite_error_response(
				&err,
			))
		}
	}
}

async fn handle_sqlite_commit_finalize_response(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteCommitFinalizeRequest,
) -> protocol::SqliteCommitFinalizeResponse {
	let actor_id = request.actor_id.clone();
	match handle_sqlite_commit_finalize(ctx, conn, request).await {
		Ok(response) => response,
		Err(err) => {
			tracing::error!(actor_id = %actor_id, ?err, "sqlite commit_finalize request failed");
			protocol::SqliteCommitFinalizeResponse::SqliteErrorResponse(sqlite_error_response(&err))
		}
	}
}

async fn ack_commands(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	envoy_key: &str,
	ack: protocol::ToRivetAckCommands,
) -> Result<()> {
	let ack = &ack;
	ctx.udb()?
		.run(|tx| async move {
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
		})
		.await
}

async fn handle_metadata(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	envoy_key: &str,
	metadata: protocol::ToRivetMetadata,
) -> Result<()> {
	let metadata = &metadata;
	ctx.udb()?
		.run(|tx| {
			async move {
				let tx = tx.with_subspace(pegboard::keys::subspace());

				// Populate actor names if provided
				if let Some(actor_names) = &metadata.prepopulate_actor_names {
					// Write each actor name into the namespace actor names list
					for (name, data) in actor_names {
						let metadata = serde_json::from_str::<
							serde_json::Map<String, serde_json::Value>,
						>(&data.metadata)
						.unwrap_or_default();

						tx.write(
							&pegboard::keys::ns::ActorNameKey::new(namespace_id, name.clone()),
							ActorNameKeyData { metadata },
						)?;
					}
				}

				// Write envoy metadata
				if let Some(metadata) = &metadata.metadata {
					let metadata = MetadataKeyData {
						metadata:
							serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
								&metadata,
							)
							.unwrap_or_default(),
					};

					let metadata_key = pegboard::keys::envoy::MetadataKey::new(
						namespace_id,
						envoy_key.to_string(),
					);

					// Clear old metadata
					tx.delete_key_subspace(&metadata_key);

					// Write metadata
					for (i, chunk) in metadata_key.split(metadata)?.into_iter().enumerate() {
						let chunk_key = metadata_key.chunk(i);

						tx.set(&tx.pack(&chunk_key), &chunk);
					}
				}
				Ok(())
			}
		})
		.await
}

#[tracing::instrument(skip_all)]
async fn handle_tunnel_message(
	ctx: &StandaloneCtx,
	authorized_tunnel_routes: &HashMap<(protocol::GatewayId, protocol::RequestId), ()>,
	msg: protocol::ToRivetTunnelMessage,
) -> Result<()> {
	// Extract inner data length before consuming msg
	let inner_data_len = tunnel_message_inner_data_len(&msg.message_kind);

	// Enforce incoming payload size
	if inner_data_len > ctx.config().pegboard().envoy_max_response_payload_size() {
		return Err(errors::WsError::InvalidPacket("payload too large".to_string()).build());
	}

	if !authorized_tunnel_routes
		.contains_async(&(msg.message_id.gateway_id, msg.message_id.request_id))
		.await
	{
		return Err(
			errors::WsError::InvalidPacket("unauthorized tunnel message".to_string()).build(),
		);
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
	ctx.ups()?
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

async fn handle_sqlite_get_pages(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteGetPagesRequest,
) -> Result<protocol::SqliteGetPagesResponse> {
	validate_sqlite_get_pages_request(&request)?;
	validate_sqlite_actor(ctx, conn, &request.actor_id).await?;

	match conn
		.sqlite_engine
		.get_pages(&request.actor_id, request.generation, request.pgnos.clone())
		.await
	{
		Ok(pages) => Ok(sqlite_get_pages_ok(conn, &request.actor_id, pages).await?),
		Err(err) => match sqlite_storage_error(&err) {
			Some(SqliteStorageError::FenceMismatch { reason }) => {
				Ok(protocol::SqliteGetPagesResponse::SqliteFenceMismatch(
					sqlite_fence_mismatch(conn, &request.actor_id, reason.clone()).await?,
				))
			}
			Some(SqliteStorageError::MetaMissing { operation })
				if *operation == "get_pages" && request.generation == 1 =>
			{
				match conn
					.sqlite_engine
					.takeover(
						&request.actor_id,
						sqlite_storage::takeover::TakeoverConfig::new(util::timestamp::now()),
					)
					.await
				{
					Ok(startup) => {
						tracing::warn!(
							actor_id = %request.actor_id,
							generation = startup.generation,
							"bootstrapped missing sqlite meta during get_pages"
						);
					}
					Err(takeover_err)
						if matches!(
							sqlite_storage_error(&takeover_err),
							Some(SqliteStorageError::ConcurrentTakeover)
						) =>
					{
						tracing::warn!(
							actor_id = %request.actor_id,
							"sqlite meta was bootstrapped concurrently during get_pages"
						);
					}
					Err(takeover_err) => return Err(takeover_err),
				}

				let pages = conn
					.sqlite_engine
					.get_pages(&request.actor_id, request.generation, request.pgnos)
					.await?;
				Ok(sqlite_get_pages_ok(conn, &request.actor_id, pages).await?)
			}
			_ => Err(err),
		},
	}
}

async fn sqlite_get_pages_ok(
	conn: &Conn,
	actor_id: &str,
	pages: Vec<sqlite_storage::types::FetchedPage>,
) -> Result<protocol::SqliteGetPagesResponse> {
	Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
		protocol::SqliteGetPagesOk {
			pages: pages
				.into_iter()
				.map(sqlite_runtime::protocol_sqlite_fetched_page)
				.collect(),
			meta: sqlite_runtime::protocol_sqlite_meta(
				conn.sqlite_engine.load_meta(actor_id).await?,
			),
		},
	))
}

async fn handle_sqlite_commit(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteCommitRequest,
) -> Result<protocol::SqliteCommitResponse> {
	let decode_request_start = Instant::now();
	validate_sqlite_dirty_pages("sqlite commit", &request.dirty_pages)?;
	validate_sqlite_actor(ctx, conn, &request.actor_id).await?;
	let decode_request_duration = decode_request_start.elapsed();
	conn.sqlite_engine.metrics().observe_commit_phase(
		"fast",
		"decode_request",
		decode_request_duration,
	);
	crate::metrics::SQLITE_COMMIT_ENVOY_DISPATCH_DURATION
		.observe(decode_request_duration.as_secs_f64());

	let engine_result = conn
		.sqlite_engine
		.commit(
			&request.actor_id,
			sqlite_storage::commit::CommitRequest {
				generation: request.generation,
				head_txid: request.expected_head_txid,
				db_size_pages: request.new_db_size_pages,
				dirty_pages: request
					.dirty_pages
					.into_iter()
					.map(storage_dirty_page)
					.collect(),
				now_ms: util::timestamp::now(),
			},
		)
		.await;
	let response_build_start = Instant::now();
	let response = match engine_result {
		Ok(result) => Ok(protocol::SqliteCommitResponse::SqliteCommitOk(
			protocol::SqliteCommitOk {
				new_head_txid: result.txid,
				meta: sqlite_runtime::protocol_sqlite_meta(result.meta),
			},
		)),
		Err(err) => match sqlite_storage_error(&err) {
			Some(SqliteStorageError::FenceMismatch { reason }) => {
				Ok(protocol::SqliteCommitResponse::SqliteFenceMismatch(
					sqlite_fence_mismatch(conn, &request.actor_id, reason.clone()).await?,
				))
			}
			Some(SqliteStorageError::CommitTooLarge {
				actual_size_bytes,
				max_size_bytes,
			}) => Ok(protocol::SqliteCommitResponse::SqliteCommitTooLarge(
				protocol::SqliteCommitTooLarge {
					actual_size_bytes: *actual_size_bytes,
					max_size_bytes: *max_size_bytes,
				},
			)),
			_ => Err(err),
		},
	}?;
	conn.sqlite_engine.metrics().observe_commit_phase(
		"fast",
		"response_build",
		response_build_start.elapsed(),
	);
	Ok(response)
}

async fn handle_sqlite_commit_stage(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteCommitStageRequest,
) -> Result<protocol::SqliteCommitStageResponse> {
	validate_sqlite_actor(ctx, conn, &request.actor_id).await?;

	match conn
		.sqlite_engine
		.commit_stage(
			&request.actor_id,
			sqlite_storage::commit::CommitStageRequest {
				generation: request.generation,
				txid: request.txid,
				chunk_idx: request.chunk_idx,
				bytes: request.bytes,
				is_last: request.is_last,
			},
		)
		.await
	{
		Ok(result) => Ok(protocol::SqliteCommitStageResponse::SqliteCommitStageOk(
			protocol::SqliteCommitStageOk {
				chunk_idx_committed: result
					.chunk_idx_committed
					.try_into()
					.context("sqlite stage chunk index exceeded u16")?,
			},
		)),
		Err(err) => match sqlite_storage_error(&err) {
			Some(SqliteStorageError::FenceMismatch { reason }) => {
				Ok(protocol::SqliteCommitStageResponse::SqliteFenceMismatch(
					sqlite_fence_mismatch(conn, &request.actor_id, reason.clone()).await?,
				))
			}
			_ => Err(err),
		},
	}
}

async fn handle_sqlite_commit_stage_begin(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteCommitStageBeginRequest,
) -> Result<protocol::SqliteCommitStageBeginResponse> {
	validate_sqlite_actor(ctx, conn, &request.actor_id).await?;

	match conn
		.sqlite_engine
		.commit_stage_begin(
			&request.actor_id,
			sqlite_storage::commit::CommitStageBeginRequest {
				generation: request.generation,
			},
		)
		.await
	{
		Ok(result) => Ok(
			protocol::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(
				protocol::SqliteCommitStageBeginOk { txid: result.txid },
			),
		),
		Err(err) => match sqlite_storage_error(&err) {
			Some(SqliteStorageError::FenceMismatch { reason }) => Ok(
				protocol::SqliteCommitStageBeginResponse::SqliteFenceMismatch(
					sqlite_fence_mismatch(conn, &request.actor_id, reason.clone()).await?,
				),
			),
			_ => Err(err),
		},
	}
}

async fn handle_sqlite_commit_finalize(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteCommitFinalizeRequest,
) -> Result<protocol::SqliteCommitFinalizeResponse> {
	let decode_request_start = Instant::now();
	validate_sqlite_actor(ctx, conn, &request.actor_id).await?;
	conn.sqlite_engine.metrics().observe_commit_phase(
		"slow",
		"decode_request",
		decode_request_start.elapsed(),
	);

	let engine_result = conn
		.sqlite_engine
		.commit_finalize(
			&request.actor_id,
			sqlite_storage::commit::CommitFinalizeRequest {
				generation: request.generation,
				expected_head_txid: request.expected_head_txid,
				txid: request.txid,
				new_db_size_pages: request.new_db_size_pages,
				now_ms: util::timestamp::now(),
			},
		)
		.await;
	let response_build_start = Instant::now();
	let response = match engine_result {
		Ok(result) => Ok(
			protocol::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				protocol::SqliteCommitFinalizeOk {
					new_head_txid: result.new_head_txid,
					meta: sqlite_runtime::protocol_sqlite_meta(result.meta),
				},
			),
		),
		Err(err) => match sqlite_storage_error(&err) {
			Some(SqliteStorageError::FenceMismatch { reason }) => {
				Ok(protocol::SqliteCommitFinalizeResponse::SqliteFenceMismatch(
					sqlite_fence_mismatch(conn, &request.actor_id, reason.clone()).await?,
				))
			}
			Some(SqliteStorageError::StageNotFound { stage_id }) => {
				Ok(protocol::SqliteCommitFinalizeResponse::SqliteStageNotFound(
					protocol::SqliteStageNotFound {
						stage_id: *stage_id,
					},
				))
			}
			_ => Err(err),
		},
	}?;
	conn.sqlite_engine.metrics().observe_commit_phase(
		"slow",
		"response_build",
		response_build_start.elapsed(),
	);
	Ok(response)
}

async fn validate_sqlite_actor(ctx: &StandaloneCtx, conn: &Conn, actor_id: &str) -> Result<()> {
	let actor_id = Id::parse(actor_id).context("invalid sqlite actor id")?;
	let actor = ctx
		.op(pegboard::ops::actor::get_for_kv::Input { actor_id })
		.await?
		.ok_or_else(|| anyhow::anyhow!("actor does not exist"))?;

	if actor.namespace_id != conn.namespace_id {
		bail!("actor does not exist");
	}

	Ok(())
}

async fn sqlite_fence_mismatch(
	conn: &Conn,
	actor_id: &str,
	reason: String,
) -> Result<protocol::SqliteFenceMismatch> {
	Ok(protocol::SqliteFenceMismatch {
		actual_meta: sqlite_runtime::protocol_sqlite_meta(
			conn.sqlite_engine.load_meta(actor_id).await?,
		),
		reason,
	})
}

fn storage_dirty_page(page: protocol::SqliteDirtyPage) -> sqlite_storage::types::DirtyPage {
	sqlite_storage::types::DirtyPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}

fn validate_sqlite_get_pages_request(request: &protocol::SqliteGetPagesRequest) -> Result<()> {
	for pgno in &request.pgnos {
		ensure!(*pgno > 0, "sqlite get_pages does not accept page 0");
	}

	Ok(())
}

fn validate_sqlite_dirty_pages(
	request_name: &str,
	dirty_pages: &[protocol::SqliteDirtyPage],
) -> Result<()> {
	let mut seen = BTreeSet::new();
	for page in dirty_pages {
		ensure!(page.pgno > 0, "{request_name} does not accept page 0");
		ensure!(
			page.bytes.len() == sqlite_storage::types::SQLITE_PAGE_SIZE as usize,
			"{request_name} page {} had {} bytes, expected {}",
			page.pgno,
			page.bytes.len(),
			sqlite_storage::types::SQLITE_PAGE_SIZE
		);
		ensure!(
			seen.insert(page.pgno),
			"{request_name} duplicated page {} in a single request",
			page.pgno
		);
	}

	Ok(())
}

fn sqlite_storage_error(err: &anyhow::Error) -> Option<&SqliteStorageError> {
	err.downcast_ref::<SqliteStorageError>()
}

fn sqlite_error_reason(err: &anyhow::Error) -> String {
	err.chain()
		.map(ToString::to_string)
		.collect::<Vec<_>>()
		.join(": ")
}

fn sqlite_error_response(err: &anyhow::Error) -> protocol::SqliteErrorResponse {
	protocol::SqliteErrorResponse {
		message: sqlite_error_reason(err),
	}
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

#[cfg(test)]
#[path = "../tests/support/ws_to_tunnel_task.rs"]
mod tests;

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

async fn send_sqlite_get_pages_response(
	conn: &Conn,
	request_id: u32,
	data: protocol::SqliteGetPagesResponse,
) -> Result<()> {
	send_to_envoy(
		conn,
		protocol::ToEnvoy::ToEnvoySqliteGetPagesResponse(protocol::ToEnvoySqliteGetPagesResponse {
			request_id,
			data,
		}),
		"sqlite get_pages response",
	)
	.await
}

async fn send_sqlite_commit_response(
	conn: &Conn,
	request_id: u32,
	data: protocol::SqliteCommitResponse,
) -> Result<()> {
	send_to_envoy(
		conn,
		protocol::ToEnvoy::ToEnvoySqliteCommitResponse(protocol::ToEnvoySqliteCommitResponse {
			request_id,
			data,
		}),
		"sqlite commit response",
	)
	.await
}

async fn send_sqlite_commit_stage_response(
	conn: &Conn,
	request_id: u32,
	data: protocol::SqliteCommitStageResponse,
) -> Result<()> {
	send_to_envoy(
		conn,
		protocol::ToEnvoy::ToEnvoySqliteCommitStageResponse(
			protocol::ToEnvoySqliteCommitStageResponse { request_id, data },
		),
		"sqlite commit_stage response",
	)
	.await
}

async fn send_sqlite_commit_stage_begin_response(
	conn: &Conn,
	request_id: u32,
	data: protocol::SqliteCommitStageBeginResponse,
) -> Result<()> {
	send_to_envoy(
		conn,
		protocol::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(
			protocol::ToEnvoySqliteCommitStageBeginResponse { request_id, data },
		),
		"sqlite commit_stage_begin response",
	)
	.await
}

async fn send_sqlite_commit_finalize_response(
	conn: &Conn,
	request_id: u32,
	data: protocol::SqliteCommitFinalizeResponse,
) -> Result<()> {
	send_to_envoy(
		conn,
		protocol::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(
			protocol::ToEnvoySqliteCommitFinalizeResponse { request_id, data },
		),
		"sqlite commit_finalize response",
	)
	.await
}

async fn send_to_envoy(conn: &Conn, msg: protocol::ToEnvoy, description: &str) -> Result<()> {
	let serialized = versioned::ToEnvoy::wrap_latest(msg)
		.serialize(conn.protocol_version)
		.with_context(|| format!("failed to serialize {description}"))?;
	conn.ws_handle
		.send(Message::Binary(serialized.into()))
		.await
		.with_context(|| format!("failed to send {description}"))?;

	Ok(())
}
