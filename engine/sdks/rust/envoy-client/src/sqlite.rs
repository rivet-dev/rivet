use rivet_envoy_protocol as protocol;
use tokio::sync::oneshot;

use crate::connection::ws_send;
use crate::envoy::EnvoyContext;
use crate::kv::KV_EXPIRE_MS;

#[derive(Clone)]
pub enum SqliteRequest {
	GetPages(protocol::SqliteGetPagesRequest),
	Commit(protocol::SqliteCommitRequest),
	CommitStageBegin(protocol::SqliteCommitStageBeginRequest),
	CommitStage(protocol::SqliteCommitStageRequest),
	CommitFinalize(protocol::SqliteCommitFinalizeRequest),
	PersistPreloadHints(protocol::SqlitePersistPreloadHintsRequest),
}

pub enum SqliteResponse {
	GetPages(protocol::SqliteGetPagesResponse),
	Commit(protocol::SqliteCommitResponse),
	CommitStageBegin(protocol::SqliteCommitStageBeginResponse),
	CommitStage(protocol::SqliteCommitStageResponse),
	CommitFinalize(protocol::SqliteCommitFinalizeResponse),
	PersistPreloadHints(protocol::SqlitePersistPreloadHintsResponse),
}

pub struct SqliteRequestEntry {
	pub request: SqliteRequest,
	pub response_tx: oneshot::Sender<anyhow::Result<SqliteResponse>>,
	pub sent: bool,
	pub timestamp: std::time::Instant,
}

pub async fn handle_sqlite_request(
	ctx: &mut EnvoyContext,
	request: SqliteRequest,
	response_tx: oneshot::Sender<anyhow::Result<SqliteResponse>>,
) {
	let request_id = ctx.next_sqlite_request_id;
	ctx.next_sqlite_request_id += 1;

	let entry = SqliteRequestEntry {
		request,
		response_tx,
		sent: false,
		timestamp: std::time::Instant::now(),
	};

	ctx.sqlite_requests.insert(request_id, entry);

	let ws_available = {
		let guard = ctx.shared.ws_tx.lock().await;
		guard.is_some()
	};

	if ws_available {
		send_single_sqlite_request(ctx, request_id).await;
	}
}

pub async fn handle_sqlite_get_pages_response(
	ctx: &mut EnvoyContext,
	response: protocol::ToEnvoySqliteGetPagesResponse,
) {
	handle_sqlite_response(
		ctx,
		response.request_id,
		SqliteResponse::GetPages(response.data),
		"sqlite_get_pages",
	);
}

pub async fn handle_sqlite_commit_response(
	ctx: &mut EnvoyContext,
	response: protocol::ToEnvoySqliteCommitResponse,
) {
	handle_sqlite_response(
		ctx,
		response.request_id,
		SqliteResponse::Commit(response.data),
		"sqlite_commit",
	);
}

pub async fn handle_sqlite_commit_stage_begin_response(
	ctx: &mut EnvoyContext,
	response: protocol::ToEnvoySqliteCommitStageBeginResponse,
) {
	handle_sqlite_response(
		ctx,
		response.request_id,
		SqliteResponse::CommitStageBegin(response.data),
		"sqlite_commit_stage_begin",
	);
}

pub async fn handle_sqlite_commit_stage_response(
	ctx: &mut EnvoyContext,
	response: protocol::ToEnvoySqliteCommitStageResponse,
) {
	handle_sqlite_response(
		ctx,
		response.request_id,
		SqliteResponse::CommitStage(response.data),
		"sqlite_commit_stage",
	);
}

pub async fn handle_sqlite_commit_finalize_response(
	ctx: &mut EnvoyContext,
	response: protocol::ToEnvoySqliteCommitFinalizeResponse,
) {
	handle_sqlite_response(
		ctx,
		response.request_id,
		SqliteResponse::CommitFinalize(response.data),
		"sqlite_commit_finalize",
	);
}

pub async fn handle_sqlite_persist_preload_hints_response(
	ctx: &mut EnvoyContext,
	response: protocol::ToEnvoySqlitePersistPreloadHintsResponse,
) {
	handle_sqlite_response(
		ctx,
		response.request_id,
		SqliteResponse::PersistPreloadHints(response.data),
		"sqlite_persist_preload_hints",
	);
}

fn handle_sqlite_response(
	ctx: &mut EnvoyContext,
	request_id: u32,
	response: SqliteResponse,
	op: &str,
) {
	let request = ctx.sqlite_requests.remove(&request_id);

	if let Some(request) = request {
		let _ = request.response_tx.send(Ok(response));
	} else {
		tracing::error!(
			request_id,
			op,
			"received sqlite response for unknown request id"
		);
	}
}

pub async fn send_single_sqlite_request(ctx: &mut EnvoyContext, request_id: u32) {
	let request = ctx.sqlite_requests.get_mut(&request_id);
	let Some(request) = request else { return };
	if request.sent {
		return;
	}

	let message =
		match request.request.clone() {
			SqliteRequest::GetPages(data) => protocol::ToRivet::ToRivetSqliteGetPagesRequest(
				protocol::ToRivetSqliteGetPagesRequest { request_id, data },
			),
			SqliteRequest::Commit(data) => protocol::ToRivet::ToRivetSqliteCommitRequest(
				protocol::ToRivetSqliteCommitRequest { request_id, data },
			),
			SqliteRequest::CommitStageBegin(data) => {
				protocol::ToRivet::ToRivetSqliteCommitStageBeginRequest(
					protocol::ToRivetSqliteCommitStageBeginRequest { request_id, data },
				)
			}
			SqliteRequest::CommitStage(data) => protocol::ToRivet::ToRivetSqliteCommitStageRequest(
				protocol::ToRivetSqliteCommitStageRequest { request_id, data },
			),
			SqliteRequest::CommitFinalize(data) => {
				protocol::ToRivet::ToRivetSqliteCommitFinalizeRequest(
					protocol::ToRivetSqliteCommitFinalizeRequest { request_id, data },
				)
			}
			SqliteRequest::PersistPreloadHints(data) => {
				protocol::ToRivet::ToRivetSqlitePersistPreloadHintsRequest(
					protocol::ToRivetSqlitePersistPreloadHintsRequest { request_id, data },
				)
			}
		};

	ws_send(&ctx.shared, message).await;

	if let Some(request) = ctx.sqlite_requests.get_mut(&request_id) {
		request.sent = true;
		request.timestamp = std::time::Instant::now();
	}
}

pub async fn process_unsent_sqlite_requests(ctx: &mut EnvoyContext) {
	let ws_available = {
		let guard = ctx.shared.ws_tx.lock().await;
		guard.is_some()
	};

	if !ws_available {
		return;
	}

	let unsent: Vec<u32> = ctx
		.sqlite_requests
		.iter()
		.filter(|(_, req)| !req.sent)
		.map(|(id, _)| *id)
		.collect();

	for request_id in unsent {
		send_single_sqlite_request(ctx, request_id).await;
	}
}

pub fn cleanup_old_sqlite_requests(ctx: &mut EnvoyContext) {
	let now = std::time::Instant::now();
	let mut to_delete = Vec::new();

	for (request_id, request) in &ctx.sqlite_requests {
		if now.duration_since(request.timestamp).as_millis() > KV_EXPIRE_MS as u128 {
			to_delete.push(*request_id);
		}
	}

	for request_id in to_delete {
		if let Some(request) = ctx.sqlite_requests.remove(&request_id) {
			let _ = request
				.response_tx
				.send(Err(anyhow::anyhow!("sqlite request timed out")));
		}
	}
}
