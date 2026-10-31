use rivet_envoy_protocol as protocol;
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use tokio::sync::oneshot;

use crate::connection::{WsSendResult, ws_send_for_session};
use crate::deadlines::DeadlineKey;
use crate::envoy::EnvoyContext;
use crate::kv::KV_EXPIRE_MS;
use crate::metrics::METRICS;
use crate::utils::{
	EnvoyShutdownError, RemoteSqliteConnectionSessionLostError,
	RemoteSqliteIndeterminateResultError, SqliteRequestTimeoutError,
};

/// Diagnostic-only context retained with one existing pending request. It never crosses
/// the wire, and is absent unless the prefetch TRACE target is enabled at submission.
#[derive(Clone, Debug)]
pub struct SqliteTrace {
	span: tracing::Span,
	dispatch: tracing::Dispatch,
}
impl Drop for SqliteTrace {
	fn drop(&mut self) {
		// The registry closes parent references through the ambient subscriber.
		// Retained contexts can be released by another worker or request collector.
		tracing::dispatcher::with_default(&self.dispatch, || {
			drop(std::mem::replace(&mut self.span, tracing::Span::none()));
		});
	}
}

impl SqliteTrace {
	pub(crate) fn capture() -> Option<Self> {
		tracing::enabled!(target: "sqlite_prefetch::trace", tracing::Level::TRACE).then(|| Self {
			span: tracing::Span::current(),
			dispatch: tracing::dispatcher::get_default(Clone::clone),
		})
	}
	fn event(
		&self,
		event: &'static str,
		envoy_key: &str,
		wire_request_id: u32,
		session: Option<u64>,
	) {
		tracing::dispatcher::with_default(&self.dispatch, || {
			self.span.in_scope(|| {
				tracing::trace!(target: "sqlite_prefetch::trace", event, envoy_key,
                wire_request_id, session, "native SQLite wire correlation");
			})
		});
	}
}

#[derive(Clone)]
pub enum SqliteRequest {
	GetPages(protocol::SqliteGetPagesRequest),
	Commit(protocol::SqliteCommitRequest),
	CommitStageBegin(protocol::SqliteCommitStageBeginRequest),
	CommitStageSegment(protocol::SqliteCommitStageSegmentRequest),
	CommitFinalize(protocol::SqliteCommitFinalizeRequest),
}

impl SqliteRequest {
	pub fn kind(&self) -> &'static str {
		match self {
			SqliteRequest::GetPages(_) => "get_pages",
			SqliteRequest::Commit(_) => "commit",
			SqliteRequest::CommitStageBegin(_) => "commit_stage_begin",
			SqliteRequest::CommitStageSegment(_) => "commit_stage_segment",
			SqliteRequest::CommitFinalize(_) => "commit_finalize",
		}
	}
}

pub enum SqliteResponse {
	GetPages(protocol::SqliteGetPagesResponse),
	Commit(protocol::SqliteCommitResponse),
	CommitStageBegin(protocol::SqliteCommitStageBeginResponse),
	CommitStageSegment(protocol::SqliteCommitStageSegmentResponse),
	CommitFinalize(protocol::SqliteCommitFinalizeResponse),
}

#[derive(Clone, Debug)]
pub enum RemoteSqliteRequest {
	Exec(protocol::SqliteExecRequest),
	Execute(protocol::SqliteExecuteRequest),
	ExecuteBatch(protocol::SqliteExecuteBatchRequest),
}

#[derive(Debug)]
pub enum RemoteSqliteResponse {
	Exec(protocol::SqliteExecResponse),
	Execute(protocol::SqliteExecuteResponse),
	ExecuteBatch(protocol::SqliteExecuteBatchResponse),
}

#[derive(Debug)]
pub struct RemoteSqliteResponseEnvelope {
	pub response: RemoteSqliteResponse,
	pub session: u64,
}

impl RemoteSqliteRequest {
	fn operation(&self) -> &'static str {
		match self {
			RemoteSqliteRequest::Exec(_) => "exec",
			RemoteSqliteRequest::Execute(_) => "execute",
			RemoteSqliteRequest::ExecuteBatch(_) => "execute_batch",
		}
	}

	pub fn kind(&self) -> &'static str {
		match self {
			RemoteSqliteRequest::Exec(_) => "remote_exec",
			RemoteSqliteRequest::Execute(_) => "remote_execute",
			RemoteSqliteRequest::ExecuteBatch(_) => "remote_execute_batch",
		}
	}
}

pub struct SqliteRequestEntry {
	pub trace: Option<SqliteTrace>,
	pub request: SqliteRequest,
	pub response_tx: oneshot::Sender<anyhow::Result<SqliteResponse>>,
	pub sent: bool,
	pub sent_session: Option<u64>,
	pub timestamp: crate::time::Instant,
	/// Counts this request on `sqlite_requests_inflight` while it is tracked.
	pub _inflight: IntGaugeGuard,
}

pub struct RemoteSqliteRequestEntry {
	pub request: RemoteSqliteRequest,
	pub response_tx: oneshot::Sender<anyhow::Result<RemoteSqliteResponseEnvelope>>,
	pub expected_session: Option<u64>,
	pub sent_session: Option<u64>,
	pub sent: bool,
	pub timestamp: crate::time::Instant,
	/// Counts this request on `remote_sqlite_requests_inflight` while it is tracked.
	pub _inflight: IntGaugeGuard,
}

pub async fn handle_sqlite_request(
	ctx: &mut EnvoyContext,
	request: SqliteRequest,
	response_tx: oneshot::Sender<anyhow::Result<SqliteResponse>>,
	trace: Option<SqliteTrace>,
) {
	let request_id = ctx.next_sqlite_request_id;
	ctx.next_sqlite_request_id += 1;

	let entry = SqliteRequestEntry {
		trace,
		request,
		response_tx,
		sent: false,
		sent_session: None,
		timestamp: crate::time::Instant::now(),
		_inflight: METRICS.sqlite_requests_inflight.inc_guard(),
	};

	ctx.deadlines.schedule(
		DeadlineKey::Sqlite(request_id),
		sqlite_deadline(entry.timestamp),
	);
	ctx.sqlite_requests.insert(request_id, entry);

	let ws_available = {
		let guard = ctx.shared.ws_tx.lock().await;
		guard.is_some()
	};

	if ws_available {
		send_single_sqlite_request(ctx, request_id).await;
	}
}

pub async fn handle_remote_sqlite_request(
	ctx: &mut EnvoyContext,
	request: RemoteSqliteRequest,
	expected_session: Option<u64>,
	response_tx: oneshot::Sender<anyhow::Result<RemoteSqliteResponseEnvelope>>,
) {
	let request_id = ctx.next_remote_sqlite_request_id;
	ctx.next_remote_sqlite_request_id += 1;

	let entry = RemoteSqliteRequestEntry {
		request,
		response_tx,
		expected_session,
		sent_session: None,
		sent: false,
		timestamp: crate::time::Instant::now(),
		_inflight: METRICS.remote_sqlite_requests_inflight.inc_guard(),
	};

	ctx.deadlines.schedule(
		DeadlineKey::RemoteSqlite(request_id),
		sqlite_deadline(entry.timestamp),
	);
	ctx.remote_sqlite_requests.insert(request_id, entry);

	send_single_remote_sqlite_request(ctx, request_id).await;
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

pub async fn handle_sqlite_commit_stage_segment_response(
	ctx: &mut EnvoyContext,
	response: protocol::ToEnvoySqliteCommitStageSegmentResponse,
) {
	handle_sqlite_response(
		ctx,
		response.request_id,
		SqliteResponse::CommitStageSegment(response.data),
		"sqlite_commit_stage_segment",
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

pub async fn handle_remote_sqlite_exec_response(
	ctx: &mut EnvoyContext,
	response: protocol::ToEnvoySqliteExecResponse,
) {
	handle_remote_sqlite_response(
		ctx,
		response.request_id,
		RemoteSqliteResponse::Exec(response.data),
		"remote_sqlite_exec",
	);
}

pub async fn handle_remote_sqlite_execute_response(
	ctx: &mut EnvoyContext,
	response: protocol::ToEnvoySqliteExecuteResponse,
) {
	handle_remote_sqlite_response(
		ctx,
		response.request_id,
		RemoteSqliteResponse::Execute(response.data),
		"remote_sqlite_execute",
	);
}

pub async fn handle_remote_sqlite_execute_batch_response(
	ctx: &mut EnvoyContext,
	response: protocol::ToEnvoySqliteExecuteBatchResponse,
) {
	handle_remote_sqlite_response(
		ctx,
		response.request_id,
		RemoteSqliteResponse::ExecuteBatch(response.data),
		"remote_sqlite_execute_batch",
	);
}

fn handle_sqlite_response(
	ctx: &mut EnvoyContext,
	request_id: u32,
	response: SqliteResponse,
	op: &str,
) {
	let request = ctx.sqlite_requests.remove(&request_id);
	ctx.deadlines.cancel(&DeadlineKey::Sqlite(request_id));

	if let Some(request) = request {
		if let Some(trace) = &request.trace {
			trace.event(
				"wire_response",
				&ctx.shared.envoy_key,
				request_id,
				request.sent_session,
			);
		}
		let _ = request.response_tx.send(Ok(response));
	} else {
		tracing::error!(
			request_id,
			op,
			"received sqlite response for unknown request id"
		);
	}
}

fn handle_remote_sqlite_response(
	ctx: &mut EnvoyContext,
	request_id: u32,
	response: RemoteSqliteResponse,
	op: &str,
) {
	let request = ctx.remote_sqlite_requests.remove(&request_id);
	ctx.deadlines.cancel(&DeadlineKey::RemoteSqlite(request_id));

	if let Some(request) = request {
		let session = request
			.sent_session
			.expect("a remote sqlite response must belong to a sent request");
		let _ = request
			.response_tx
			.send(Ok(RemoteSqliteResponseEnvelope { response, session }));
	} else {
		tracing::error!(
			request_id,
			op,
			"received remote sqlite response for unknown request id"
		);
	}
}

pub async fn send_single_sqlite_request(ctx: &mut EnvoyContext, request_id: u32) {
	if ctx
		.sqlite_requests
		.get(&request_id)
		.is_some_and(|request| request.response_tx.is_closed())
	{
		ctx.sqlite_requests.remove(&request_id);
		return;
	}
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
			SqliteRequest::CommitStageSegment(data) => {
				protocol::ToRivet::ToRivetSqliteCommitStageSegmentRequest(
					protocol::ToRivetSqliteCommitStageSegmentRequest { request_id, data },
				)
			}
			SqliteRequest::CommitFinalize(data) => {
				protocol::ToRivet::ToRivetSqliteCommitFinalizeRequest(
					protocol::ToRivetSqliteCommitFinalizeRequest { request_id, data },
				)
			}
		};

	if let WsSendResult::Sent { session } = ws_send_for_session(&ctx.shared, message, None).await
		&& let Some(request) = ctx.sqlite_requests.get_mut(&request_id)
	{
		request.sent = true;
		request.sent_session = Some(session);
		if let Some(trace) = &request.trace {
			trace.event(
				"wire_send",
				&ctx.shared.envoy_key,
				request_id,
				Some(session),
			);
		}
	}
}

pub async fn send_single_remote_sqlite_request(ctx: &mut EnvoyContext, request_id: u32) {
	if ctx
		.remote_sqlite_requests
		.get(&request_id)
		.is_some_and(|request| request.response_tx.is_closed())
	{
		ctx.remote_sqlite_requests.remove(&request_id);
		return;
	}
	let request = ctx.remote_sqlite_requests.get_mut(&request_id);
	let Some(request) = request else { return };
	if request.sent {
		return;
	}

	let expected_session = request.expected_session;
	let message = remote_sqlite_request_to_message(request_id, request.request.clone());

	match ws_send_for_session(&ctx.shared, message, expected_session).await {
		WsSendResult::Sent { session } => {
			if let Some(request) = ctx.remote_sqlite_requests.get_mut(&request_id) {
				request.sent = true;
				request.sent_session = Some(session);
			}
		}
		WsSendResult::Unavailable if expected_session.is_none() => {
			// Ordinary requests that have never crossed the socket remain eligible
			// for replay. A transaction-affine request takes the error branch below.
		}
		WsSendResult::Unavailable => {
			fail_remote_sqlite_session_request(ctx, request_id, None);
		}
		WsSendResult::StaleSession { current } => {
			fail_remote_sqlite_session_request(ctx, request_id, current);
		}
	}
}

fn fail_remote_sqlite_session_request(
	ctx: &mut EnvoyContext,
	request_id: u32,
	current: Option<u64>,
) {
	let Some(request) = ctx.remote_sqlite_requests.remove(&request_id) else {
		return;
	};
	let expected = request
		.expected_session
		.expect("only session-affine requests fail session admission");
	let _ = request.response_tx.send(Err(anyhow::anyhow!(
		RemoteSqliteConnectionSessionLostError { expected, current }
	)));
}

pub fn remote_sqlite_request_to_message(
	request_id: u32,
	request: RemoteSqliteRequest,
) -> protocol::ToRivet {
	match request {
		RemoteSqliteRequest::Exec(data) => {
			protocol::ToRivet::ToRivetSqliteExecRequest(protocol::ToRivetSqliteExecRequest {
				request_id,
				data,
			})
		}
		RemoteSqliteRequest::Execute(data) => {
			protocol::ToRivet::ToRivetSqliteExecuteRequest(protocol::ToRivetSqliteExecuteRequest {
				request_id,
				data,
			})
		}
		RemoteSqliteRequest::ExecuteBatch(data) => {
			protocol::ToRivet::ToRivetSqliteExecuteBatchRequest(
				protocol::ToRivetSqliteExecuteBatchRequest { request_id, data },
			)
		}
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

pub async fn process_unsent_remote_sqlite_requests(ctx: &mut EnvoyContext) {
	let ws_available = {
		let guard = ctx.shared.ws_tx.lock().await;
		guard.is_some()
	};

	if !ws_available {
		return;
	}

	let unsent: Vec<u32> = ctx
		.remote_sqlite_requests
		.iter()
		.filter(|(_, req)| !req.sent)
		.map(|(id, _)| *id)
		.collect();

	for request_id in unsent {
		send_single_remote_sqlite_request(ctx, request_id).await;
	}
}

/// A SQLite request keeps its first timestamp across retries, so its deadline does too.
fn sqlite_deadline(timestamp: crate::time::Instant) -> crate::time::Instant {
	timestamp + std::time::Duration::from_millis(KV_EXPIRE_MS)
}

/// Fails one SQLite request whose deadline passed.
pub(crate) fn expire_sqlite_request(
	ctx: &mut EnvoyContext,
	request_id: u32,
	now: crate::time::Instant,
) {
	let Some(request) = ctx.sqlite_requests.get(&request_id) else {
		return;
	};
	let deadline = sqlite_deadline(request.timestamp);
	if deadline > now {
		ctx.deadlines
			.schedule(DeadlineKey::Sqlite(request_id), deadline);
		return;
	}
	let request = ctx
		.sqlite_requests
		.remove(&request_id)
		.expect("checked sqlite request");
	let kind = request.request.kind();
	let was_sent = if request.sent { "true" } else { "false" };
	METRICS
		.sqlite_request_expired_total
		.with_label_values(&[kind, was_sent])
		.inc();
	tracing::warn!(
		request_id,
		kind,
		was_sent = request.sent,
		age_ms = now.duration_since(request.timestamp).as_millis() as u64,
		"sqlite request expired at its deadline; if was_sent=true this indicates an abandoned in-flight request"
	);
	let _ = request
		.response_tx
		.send(Err(anyhow::anyhow!(SqliteRequestTimeoutError {
			operation: kind,
			sent: request.sent,
		})));
}

/// Fails one remote SQLite request whose deadline passed.
pub(crate) fn expire_remote_sqlite_request(
	ctx: &mut EnvoyContext,
	request_id: u32,
	now: crate::time::Instant,
) {
	let Some(request) = ctx.remote_sqlite_requests.get(&request_id) else {
		return;
	};
	let deadline = sqlite_deadline(request.timestamp);
	if deadline > now {
		ctx.deadlines
			.schedule(DeadlineKey::RemoteSqlite(request_id), deadline);
		return;
	}
	let request = ctx
		.remote_sqlite_requests
		.remove(&request_id)
		.expect("checked remote sqlite request");
	let kind = request.request.kind();
	let was_sent = if request.sent { "true" } else { "false" };
	METRICS
		.sqlite_request_expired_total
		.with_label_values(&[kind, was_sent])
		.inc();
	tracing::warn!(
		request_id,
		kind,
		was_sent = request.sent,
		age_ms = now.duration_since(request.timestamp).as_millis() as u64,
		"remote sqlite request expired at its deadline; if was_sent=true this indicates an abandoned in-flight request"
	);
	let _ = request
		.response_tx
		.send(Err(anyhow::anyhow!("remote sqlite request timed out")));
}

pub fn fail_sqlite_requests_with_shutdown(ctx: &mut EnvoyContext) {
	for (_id, request) in ctx.sqlite_requests.drain() {
		let _ = request
			.response_tx
			.send(Err(anyhow::anyhow!(EnvoyShutdownError)));
	}
}

pub fn fail_remote_sqlite_requests_with_shutdown(ctx: &mut EnvoyContext) {
	for (_id, request) in ctx.remote_sqlite_requests.drain() {
		let _ = request
			.response_tx
			.send(Err(anyhow::anyhow!(EnvoyShutdownError)));
	}
}

pub fn fail_sent_remote_sqlite_requests(ctx: &mut EnvoyContext, session: u64) {
	let request_ids: Vec<u32> = ctx
		.remote_sqlite_requests
		.iter()
		.filter(|(_, request)| request.sent_session == Some(session))
		.map(|(request_id, _)| *request_id)
		.collect();

	for request_id in request_ids {
		if let Some(request) = ctx.remote_sqlite_requests.remove(&request_id) {
			let operation = request.request.operation();
			tracing::warn!(
				request_id,
				operation,
				"remote sqlite response lost after websocket disconnect"
			);
			let _ = request.response_tx.send(Err(anyhow::anyhow!(
				RemoteSqliteIndeterminateResultError { operation }
			)));
		}
	}
}

/// Requeue safe reads under fresh IDs, but never replay mutations whose result was lost.
pub fn handle_sqlite_connection_closed(ctx: &mut EnvoyContext, session: u64) {
	let request_ids = ctx
		.sqlite_requests
		.iter()
		.filter_map(|(id, request)| (request.sent_session == Some(session)).then_some(*id))
		.collect::<Vec<_>>();

	for request_id in request_ids {
		let Some(mut request) = ctx.sqlite_requests.remove(&request_id) else {
			continue;
		};
		if request.response_tx.is_closed() {
			continue;
		}
		if matches!(request.request, SqliteRequest::GetPages(_)) {
			request.sent = false;
			request.sent_session = None;
			// A fresh ID prevents a delayed reply from the closed session winning the retry.
			let retry_id = ctx.next_sqlite_request_id;
			ctx.next_sqlite_request_id += 1;
			ctx.deadlines.cancel(&DeadlineKey::Sqlite(request_id));
			ctx.deadlines.schedule(
				DeadlineKey::Sqlite(retry_id),
				sqlite_deadline(request.timestamp),
			);
			ctx.sqlite_requests.insert(retry_id, request);
		} else {
			let operation = request.request.kind();
			let _ = request.response_tx.send(Err(anyhow::anyhow!(
				RemoteSqliteIndeterminateResultError { operation }
			)));
		}
	}

	fail_sent_remote_sqlite_requests(ctx, session);
}




