use anyhow::{Context, bail};
use bytes::Bytes;
use depot::{
	conveyer::Db,
	error::SqliteStorageError,
	workflows::compaction::{
		DATABASE_BRANCH_ID_TAG, DbManagerInput, DeltasAvailable, database_branch_tag_value,
	},
};
use depot_client::{
	database::NativeDatabaseHandle,
	types::{BindParam, ColumnValue, ExecuteResult, QueryResult},
};
use depot_client_embedded::open_database_from_embedded_depot;
use futures_util::{FutureExt, TryStreamExt};
use gas::prelude::Id;
use gas::prelude::*;
use hyper_tungstenite::tungstenite::Message;
use pegboard::actor_kv;
use pegboard::pubsub_subjects::GatewayReceiverSubject;
use rivet_data::converted::{ActorNameKeyData, MetadataKeyData};
use rivet_envoy_protocol::{self as protocol, PROTOCOL_VERSION, versioned};
use rivet_guard_core::websocket_handle::WebSocketReceiver;
use scc::HashMap;
use std::{
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
	LifecycleResult,
	actor_event_demuxer::ActorEventDemuxer,
	conn::{Conn, RemoteSqliteExecutors},
	errors, sqlite_runtime,
};

const MAX_REMOTE_SQL_BIND_BYTES: usize = 128 * 1024;

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
				handle_message(&ctx, conn.clone(), event_demuxer, msg).await?;
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
	conn: Arc<Conn>,
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
			let response = handle_sqlite_get_pages_response(ctx, &conn, req.data).await;
			send_sqlite_get_pages_response(&conn, req.request_id, response).await?;
		}
		protocol::ToRivet::ToRivetSqliteCommitRequest(req) => {
			let actor_id = req.data.actor_id.clone();
			let request_id = req.request_id;
			let timed_response =
				async { handle_sqlite_commit_response(ctx, &conn, req.data).await }
					.instrument(tracing::debug_span!(
						"handle_sqlite_commit",
						actor_id = %actor_id,
						request_id = ?request_id
					))
					.await;
			send_sqlite_commit_response(&conn, request_id, timed_response.response).await?;
			crate::metrics::SQLITE_COMMIT_ENVOY_RESPONSE_DURATION
				.observe(timed_response.commit_completed_at.elapsed().as_secs_f64());
		}
		protocol::ToRivet::ToRivetSqliteExecRequest(req) => {
			let response = handle_remote_sqlite_exec_response(ctx, &conn, req.data).await;
			send_sqlite_exec_response(&conn, req.request_id, response).await?;
		}
		protocol::ToRivet::ToRivetSqliteExecuteRequest(req) => {
			let response = handle_remote_sqlite_execute_response(ctx, &conn, req.data).await;
			send_sqlite_execute_response(&conn, req.request_id, response).await?;
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
	let pgnos = request.pgnos.clone();
	let expected_generation = request.expected_generation;
	let expected_head_txid = request.expected_head_txid;
	match handle_sqlite_get_pages(ctx, conn, request).await {
		Ok(response) => response,
		Err(err) => {
			tracing::error!(
				actor_id = %actor_id,
				?pgnos,
				?expected_generation,
				?expected_head_txid,
				?err,
				"sqlite get_pages request failed"
			);
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

async fn handle_remote_sqlite_exec_response(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteExecRequest,
) -> protocol::SqliteExecResponse {
	let actor_id = request.actor_id.clone();
	match handle_remote_sqlite_exec(ctx, conn, request).await {
		Ok(result) => protocol::SqliteExecResponse::SqliteExecOk(protocol::SqliteExecOk {
			result: protocol_query_result(result),
		}),
		Err(err) => {
			tracing::error!(actor_id = %actor_id, ?err, "remote sqlite exec request failed");
			protocol::SqliteExecResponse::SqliteErrorResponse(sqlite_error_response(&err))
		}
	}
}

async fn handle_remote_sqlite_execute_response(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteExecuteRequest,
) -> protocol::SqliteExecuteResponse {
	let actor_id = request.actor_id.clone();
	match handle_remote_sqlite_execute(ctx, conn, request).await {
		Ok(result) => protocol::SqliteExecuteResponse::SqliteExecuteOk(protocol::SqliteExecuteOk {
			result: protocol_execute_result(result),
		}),
		Err(err) => {
			tracing::error!(actor_id = %actor_id, ?err, "remote sqlite execute request failed");
			protocol::SqliteExecuteResponse::SqliteErrorResponse(sqlite_error_response(&err))
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
	_authorized_tunnel_routes: &HashMap<(protocol::GatewayId, protocol::RequestId), ()>,
	msg: protocol::ToRivetTunnelMessage,
) -> Result<()> {
	// Extract inner data length before consuming msg
	let inner_data_len = tunnel_message_inner_data_len(&msg.message_kind);

	// Enforce incoming payload size
	if inner_data_len > ctx.config().pegboard().envoy_max_response_payload_size() {
		return Err(errors::WsError::InvalidPacket("payload too large".to_string()).build());
	}

	// if !authorized_tunnel_routes
	// 	.contains_async(&(msg.message_id.gateway_id, msg.message_id.request_id))
	// 	.await
	// {
	// 	return Err(
	// 		errors::WsError::InvalidPacket("unauthorized tunnel message".to_string()).build(),
	// 	);
	// }

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
	validate_sqlite_actor(ctx, conn, &request.actor_id).await?;

	let actor_db = actor_db(ctx, conn, request.actor_id.clone()).await?;
	let pages = actor_db.get_pages(request.pgnos).await?;
	Ok(sqlite_get_pages_ok(pages).await?)
}

async fn sqlite_get_pages_ok(
	pages: Vec<depot::types::FetchedPage>,
) -> Result<protocol::SqliteGetPagesResponse> {
	Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
		protocol::SqliteGetPagesOk {
			pages: pages
				.into_iter()
				.map(sqlite_runtime::protocol_sqlite_conveyer_fetched_page)
				.collect(),
		},
	))
}

async fn handle_sqlite_commit(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteCommitRequest,
) -> Result<protocol::SqliteCommitResponse> {
	let decode_request_start = Instant::now();
	validate_sqlite_actor(ctx, conn, &request.actor_id).await?;
	let decode_request_duration = decode_request_start.elapsed();
	crate::metrics::SQLITE_COMMIT_ENVOY_DISPATCH_DURATION
		.observe(decode_request_duration.as_secs_f64());

	let actor_id = request.actor_id.clone();
	let actor_db = actor_db(ctx, conn, actor_id.clone()).await?;
	let engine_result = actor_db
		.commit(
			request
				.dirty_pages
				.into_iter()
				.map(pump_dirty_page)
				.collect(),
			request.db_size_pages,
			request.now_ms,
		)
		.await;
	let response_build_start = Instant::now();
	let response = match engine_result {
		Ok(()) => Ok(protocol::SqliteCommitResponse::SqliteCommitOk),
		Err(err) => match depot_error(&err) {
			Some(SqliteStorageError::CommitTooLarge {
				actual_size_bytes,
				max_size_bytes,
			}) => Ok(protocol::SqliteCommitResponse::SqliteErrorResponse(
				protocol::SqliteErrorResponse {
					message: format!(
						"sqlite commit too large: actual_size_bytes={actual_size_bytes}, max_size_bytes={max_size_bytes}"
					),
				},
			)),
			_ => Err(err),
		},
	}?;
	crate::metrics::SQLITE_COMMIT_ENVOY_RESPONSE_DURATION
		.observe(response_build_start.elapsed().as_secs_f64());
	Ok(response)
}

async fn handle_remote_sqlite_exec(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteExecRequest,
) -> Result<QueryResult> {
	validate_remote_sqlite_actor(
		ctx,
		conn,
		&request.namespace_id,
		&request.actor_id,
		request.generation,
	)
	.await?;
	let actor_db = actor_db(ctx, conn, request.actor_id.clone()).await?;
	let database = remote_sqlite_executor_from_parts(
		&conn.remote_sqlite_executors,
		actor_db,
		&request.actor_id,
		request.generation,
	)
	.await?;
	database.exec(request.sql).await
}

async fn handle_remote_sqlite_execute(
	ctx: &StandaloneCtx,
	conn: &Conn,
	request: protocol::SqliteExecuteRequest,
) -> Result<ExecuteResult> {
	validate_remote_sqlite_actor(
		ctx,
		conn,
		&request.namespace_id,
		&request.actor_id,
		request.generation,
	)
	.await?;
	validate_remote_sqlite_params(request.params.as_ref())?;
	let params = request
		.params
		.map(|params| params.into_iter().map(bind_param_from_protocol).collect());
	let actor_db = actor_db(ctx, conn, request.actor_id.clone()).await?;
	let database = remote_sqlite_executor_from_parts(
		&conn.remote_sqlite_executors,
		actor_db,
		&request.actor_id,
		request.generation,
	)
	.await?;
	database.execute(request.sql, params).await
}

async fn validate_remote_sqlite_actor(
	ctx: &StandaloneCtx,
	conn: &Conn,
	namespace_name: &str,
	actor_id: &str,
	generation: u64,
) -> Result<()> {
	if namespace_name != conn.namespace_name {
		bail!("actor does not exist");
	}
	validate_sqlite_actor(ctx, conn, actor_id).await?;
	validate_remote_sqlite_generation(ctx, conn, actor_id, generation).await
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

async fn validate_remote_sqlite_generation(
	ctx: &StandaloneCtx,
	conn: &Conn,
	actor_id: &str,
	generation: u64,
) -> Result<()> {
	let actor_id = Id::parse(actor_id).context("invalid sqlite actor id")?;
	let generation = u32::try_from(generation).context("invalid sqlite actor generation")?;
	let namespace_id = conn.namespace_id;
	let envoy_key = conn.envoy_key.clone();
	let (active_generation, has_pending_start_command) = ctx
		.udb()?
		.run(|tx| {
			let envoy_key = envoy_key.clone();
			async move {
				let tx = tx.with_subspace(pegboard::keys::subspace());
				let active_generation = tx
					.read_opt(
						&pegboard::keys::envoy::ActorKey::new(
							namespace_id,
							envoy_key.clone(),
							actor_id,
						),
						Serializable,
					)
					.await?;

				let command_subspace = pegboard::keys::subspace().subspace(
					&pegboard::keys::envoy::ActorCommandKey::subspace_with_actor(
						namespace_id,
						envoy_key,
						actor_id,
						generation,
					),
				);
				let mut command_entries = tx.get_ranges_keyvalues(
					RangeOption {
						mode: StreamingMode::WantAll,
						..(&command_subspace).into()
					},
					Serializable,
				);
				let mut has_pending_start_command = false;
				while let Some(entry) = command_entries.try_next().await? {
					let (_, command) =
						tx.read_entry::<pegboard::keys::envoy::ActorCommandKey>(&entry)?;
					match command {
						protocol::ActorCommandKeyData::CommandStartActor(_) => {
							has_pending_start_command = true;
							break;
						}
						protocol::ActorCommandKeyData::CommandStopActor(_) => {}
					}
				}

				Ok((active_generation, has_pending_start_command))
			}
		})
		.await?;

	if active_generation != Some(generation) && !has_pending_start_command {
		bail!("actor does not exist");
	}

	Ok(())
}

async fn remote_sqlite_executor_cell(
	executors: &RemoteSqliteExecutors,
	actor_id: &str,
	generation: u64,
) -> Arc<tokio::sync::OnceCell<NativeDatabaseHandle>> {
	let key = (actor_id.to_string(), generation);
	executors
		.entry_async(key)
		.await
		.or_insert_with(|| Arc::new(tokio::sync::OnceCell::new()))
		.get()
		.clone()
}

async fn remote_sqlite_executor_from_parts(
	executors: &RemoteSqliteExecutors,
	actor_db: Arc<Db>,
	actor_id: &str,
	generation: u64,
) -> Result<NativeDatabaseHandle> {
	let cell = remote_sqlite_executor_cell(executors, actor_id, generation).await;
	let actor_id = actor_id.to_string();
	let database = cell
		.get_or_try_init(|| async move {
			open_database_from_embedded_depot(
				actor_db,
				actor_id,
				generation,
				tokio::runtime::Handle::current(),
				None,
			)
			.await
		})
		.await?;
	Ok(database.clone())
}

#[cfg(test)]
async fn remove_remote_sqlite_executor_generation(
	executors: &RemoteSqliteExecutors,
	actor_id: &str,
	generation: u64,
) {
	let _ = executors
		.remove_async(&(actor_id.to_string(), generation))
		.await;
}

#[cfg(test)]
fn remove_remote_sqlite_executors_for_actor(executors: &RemoteSqliteExecutors, actor_id: &str) {
	executors.retain_sync(|(entry_actor_id, _), _| entry_actor_id != actor_id);
}

#[cfg(test)]
fn clear_remote_sqlite_executors(executors: &RemoteSqliteExecutors) {
	executors.clear_sync();
}

fn validate_remote_sqlite_params(params: Option<&Vec<protocol::SqliteBindParam>>) -> Result<()> {
	let Some(params) = params else {
		return Ok(());
	};
	let total = params
		.iter()
		.map(|param| match param {
			protocol::SqliteBindParam::SqliteValueNull => 0,
			protocol::SqliteBindParam::SqliteValueInteger(_) => std::mem::size_of::<i64>(),
			protocol::SqliteBindParam::SqliteValueFloat(_) => std::mem::size_of::<u64>(),
			protocol::SqliteBindParam::SqliteValueText(value) => value.value.len(),
			protocol::SqliteBindParam::SqliteValueBlob(value) => value.value.len(),
		})
		.sum::<usize>();
	if total > MAX_REMOTE_SQL_BIND_BYTES {
		bail!(
			"remote sqlite bind params had {total} bytes, exceeding limit {MAX_REMOTE_SQL_BIND_BYTES}"
		);
	}
	Ok(())
}

fn pump_dirty_page(page: protocol::SqliteDirtyPage) -> depot::types::DirtyPage {
	depot::types::DirtyPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}

fn bind_param_from_protocol(param: protocol::SqliteBindParam) -> BindParam {
	match param {
		protocol::SqliteBindParam::SqliteValueNull => BindParam::Null,
		protocol::SqliteBindParam::SqliteValueInteger(value) => BindParam::Integer(value.value),
		protocol::SqliteBindParam::SqliteValueFloat(value) => {
			BindParam::Float(f64::from_bits(u64::from_be_bytes(value.value)))
		}
		protocol::SqliteBindParam::SqliteValueText(value) => BindParam::Text(value.value),
		protocol::SqliteBindParam::SqliteValueBlob(value) => BindParam::Blob(value.value),
	}
}

fn protocol_query_result(result: QueryResult) -> protocol::SqliteQueryResult {
	protocol::SqliteQueryResult {
		columns: result.columns,
		rows: result
			.rows
			.into_iter()
			.map(|row| row.into_iter().map(protocol_column_value).collect())
			.collect(),
	}
}

fn protocol_execute_result(result: ExecuteResult) -> protocol::SqliteExecuteResult {
	protocol::SqliteExecuteResult {
		columns: result.columns,
		rows: result
			.rows
			.into_iter()
			.map(|row| row.into_iter().map(protocol_column_value).collect())
			.collect(),
		changes: result.changes,
		last_insert_row_id: result.last_insert_row_id,
	}
}

fn protocol_column_value(value: ColumnValue) -> protocol::SqliteColumnValue {
	match value {
		ColumnValue::Null => protocol::SqliteColumnValue::SqliteValueNull,
		ColumnValue::Integer(value) => {
			protocol::SqliteColumnValue::SqliteValueInteger(protocol::SqliteValueInteger { value })
		}
		ColumnValue::Float(value) => {
			protocol::SqliteColumnValue::SqliteValueFloat(protocol::SqliteValueFloat {
				value: value.to_bits().to_be_bytes(),
			})
		}
		ColumnValue::Text(value) => {
			protocol::SqliteColumnValue::SqliteValueText(protocol::SqliteValueText { value })
		}
		ColumnValue::Blob(value) => {
			protocol::SqliteColumnValue::SqliteValueBlob(protocol::SqliteValueBlob { value })
		}
	}
}

async fn actor_db(ctx: &StandaloneCtx, conn: &Conn, actor_id: String) -> Result<Arc<Db>> {
	let db = conn
		.actor_dbs
		.entry_async(actor_id.clone())
		.await
		.or_insert_with(|| {
			let signal_ctx = ctx.clone();
			let workflow_actor_id = actor_id.clone();
			let compaction_signaler = Arc::new(move |signal: DeltasAvailable| {
				let signal_ctx = signal_ctx.clone();
				let actor_id = workflow_actor_id.clone();
				async move {
					let tag_value = database_branch_tag_value(signal.database_branch_id);
					let workflow_id = signal_ctx
						.workflow(DbManagerInput {
							database_branch_id: signal.database_branch_id,
							actor_id: Some(actor_id),
						})
						.tag(DATABASE_BRANCH_ID_TAG, &tag_value)
						.unique()
						.dispatch()
						.await?;
					signal_ctx
						.signal(signal)
						.to_workflow_id(workflow_id)
						.send()
						.await?;
					Ok(())
				}
				.boxed()
			});

			Arc::new(Db::new_with_compaction_signaler(
				conn.udb.clone(),
				conn.namespace_id,
				actor_id,
				conn.node_id,
				conn.sqlite_cold_tier.clone(),
				compaction_signaler,
			))
		})
		.get()
		.clone();
	Ok(db)
}

fn depot_error(err: &anyhow::Error) -> Option<&SqliteStorageError> {
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

async fn send_sqlite_exec_response(
	conn: &Conn,
	request_id: u32,
	data: protocol::SqliteExecResponse,
) -> Result<()> {
	send_to_envoy(
		conn,
		protocol::ToEnvoy::ToEnvoySqliteExecResponse(protocol::ToEnvoySqliteExecResponse {
			request_id,
			data,
		}),
		"sqlite exec response",
	)
	.await
}

async fn send_sqlite_execute_response(
	conn: &Conn,
	request_id: u32,
	data: protocol::SqliteExecuteResponse,
) -> Result<()> {
	send_to_envoy(
		conn,
		protocol::ToEnvoy::ToEnvoySqliteExecuteResponse(protocol::ToEnvoySqliteExecuteResponse {
			request_id,
			data,
		}),
		"sqlite execute response",
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
