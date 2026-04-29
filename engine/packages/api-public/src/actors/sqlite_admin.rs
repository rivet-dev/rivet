use std::{
	convert::Infallible,
	ops::Deref,
	sync::Arc,
	time::{Duration, SystemTime},
};

use anyhow::{Context, Result, bail};
use axum::{
	http::StatusCode,
	response::{
		IntoResponse, Response,
		sse::{Event, KeepAlive, Sse},
	},
};
use futures_util::stream;
use rivet_api_builder::{
	ApiBadRequest, ApiError, ApiNotFound,
	extract::{Extension, Json, Path},
};
use serde::{Deserialize, Serialize};
use sqlite_storage::{
	admin::{
		self, AdminOpRecord, AuditFields, ForkDstSpec, ForkMode, OpKind, OpResult, OpStatus,
		RefcountKind, RestoreMode, RestoreTarget, SQLITE_OP_SUBJECT, SqliteOp, SqliteOpRequest,
		SqliteOpSubject, encode_sqlite_op_request,
	},
	pump::types::RetentionConfig,
};
use universalpubsub::PublishOpts;
use uuid::Uuid;

use crate::ctx::ApiCtx;

const INLINE_OP_TIMEOUT: Duration = Duration::from_secs(2);
const SSE_POLL_INTERVAL: Duration = Duration::from_millis(500);
const SSE_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Deserialize)]
pub struct ActorPath {
	actor_id: String,
}

#[derive(Debug, Deserialize)]
pub struct OperationPath {
	actor_id: String,
	op_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct PostRestoreRequest {
	target: RestoreTargetBody,
	mode: RestoreModeBody,
	#[serde(default)]
	namespace_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct PostForkRequest {
	target: RestoreTargetBody,
	mode: ForkModeBody,
	dst: ForkDstBody,
	#[serde(default)]
	namespace_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RestoreTargetBody {
	Txid { txid: u64 },
	TimestampMs { timestamp_ms: i64 },
	LatestCheckpoint,
	CheckpointTxid { txid: u64 },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreModeBody {
	Apply,
	DryRun,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForkModeBody {
	Apply,
	DryRun,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ForkDstBody {
	Allocate { dst_namespace_id: Uuid },
	Existing { dst_actor_id: String },
}

#[derive(Debug, Deserialize)]
pub struct ClearRefcountRequest {
	kind: RefcountKindBody,
	txid: u64,
	#[serde(default)]
	namespace_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefcountKindBody {
	Checkpoint,
	Delta,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationAcceptedResponse {
	operation_id: Uuid,
	status: &'static str,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClearRefcountResponse {
	cleared: bool,
}

#[tracing::instrument(skip_all)]
pub async fn post_restore(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Json(body): Json<PostRestoreRequest>,
) -> Response {
	match post_restore_inner(ctx, path, body).await {
		Ok(response) => (StatusCode::ACCEPTED, Json(response)).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn post_restore_inner(
	ctx: ApiCtx,
	path: ActorPath,
	body: PostRestoreRequest,
) -> Result<OperationAcceptedResponse> {
	ctx.auth().await?;
	let op_id = Uuid::new_v4();
	let op = SqliteOp::Restore {
		actor_id: path.actor_id.clone(),
		target: body.target.into(),
		mode: body.mode.into(),
	};
	create_and_publish(ctx, op_id, OpKind::Restore, path.actor_id, body.namespace_id, op).await?;
	Ok(accepted(op_id))
}

#[tracing::instrument(skip_all)]
pub async fn post_fork(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Json(body): Json<PostForkRequest>,
) -> Response {
	match post_fork_inner(ctx, path, body).await {
		Ok(response) => (StatusCode::ACCEPTED, Json(response)).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn post_fork_inner(
	ctx: ApiCtx,
	path: ActorPath,
	body: PostForkRequest,
) -> Result<OperationAcceptedResponse> {
	ctx.auth().await?;
	let op_id = Uuid::new_v4();
	let dst = ForkDstSpec::from(body.dst);
	let namespace_id = body.namespace_id.or_else(|| match &dst {
		ForkDstSpec::Allocate { dst_namespace_id } => Some(*dst_namespace_id),
		ForkDstSpec::Existing { dst_actor_id: _ } => None,
	});
	let op = SqliteOp::Fork {
		src_actor_id: path.actor_id.clone(),
		target: body.target.into(),
		mode: body.mode.into(),
		dst,
	};
	create_and_publish(ctx, op_id, OpKind::Fork, path.actor_id, namespace_id, op).await?;
	Ok(accepted(op_id))
}

#[tracing::instrument(skip_all)]
pub async fn get_operation(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<OperationPath>,
) -> Response {
	match get_operation_inner(ctx, path).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn get_operation_inner(ctx: ApiCtx, path: OperationPath) -> Result<AdminOpRecord> {
	ctx.auth().await?;
	let record = read_operation(ctx.udb_arc()?, &path.actor_id, path.op_id).await?;
	Ok(record)
}

#[tracing::instrument(skip_all)]
pub async fn get_operation_sse(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<OperationPath>,
) -> Response {
	match get_operation_sse_inner(ctx, path).await {
		Ok(response) => response.into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn get_operation_sse_inner(
	ctx: ApiCtx,
	path: OperationPath,
) -> Result<Sse<impl futures_util::Stream<Item = std::result::Result<Event, Infallible>>>> {
	ctx.auth().await?;
	let udb = ctx.udb_arc()?;
	read_operation(Arc::clone(&udb), &path.actor_id, path.op_id).await?;
	let deadline = tokio::time::Instant::now() + SSE_TIMEOUT;
	let state = SseState {
		udb,
		actor_id: path.actor_id,
		op_id: path.op_id,
		last_payload: None,
		deadline,
		done: false,
	};

	Ok(Sse::new(stream::unfold(state, sse_next)).keep_alive(KeepAlive::default()))
}

#[tracing::instrument(skip_all)]
pub async fn get_retention(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
) -> Response {
	match get_retention_inner(ctx, path).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn get_retention_inner(
	ctx: ApiCtx,
	path: ActorPath,
) -> Result<admin::RetentionView> {
	ctx.auth().await?;
	let op_id = Uuid::new_v4();
	let op = SqliteOp::DescribeRetention {
		actor_id: path.actor_id.clone(),
	};
	let record = create_publish_and_wait(
		ctx,
		op_id,
		OpKind::DescribeRetention,
		path.actor_id,
		None,
		op,
	)
	.await?;
	match record.result {
		Some(OpResult::RetentionView(view)) => Ok(view),
		other => bail!("sqlite retention operation completed without retention view: {other:?}"),
	}
}

#[tracing::instrument(skip_all)]
pub async fn put_retention(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Json(body): Json<RetentionConfig>,
) -> Response {
	match put_retention_inner(ctx, path, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn put_retention_inner(
	ctx: ApiCtx,
	path: ActorPath,
	body: RetentionConfig,
) -> Result<admin::RetentionView> {
	ctx.auth().await?;
	let op_id = Uuid::new_v4();
	let op = SqliteOp::SetRetention {
		actor_id: path.actor_id.clone(),
		config: body,
	};
	let record = create_publish_and_wait(ctx, op_id, OpKind::SetRetention, path.actor_id, None, op)
		.await?;
	match record.result {
		Some(OpResult::RetentionView(view)) => Ok(view),
		other => bail!("sqlite set retention operation completed without retention view: {other:?}"),
	}
}

#[tracing::instrument(skip_all)]
pub async fn post_refcount_clear(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Json(body): Json<ClearRefcountRequest>,
) -> Response {
	match post_refcount_clear_inner(ctx, path, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn post_refcount_clear_inner(
	ctx: ApiCtx,
	path: ActorPath,
	body: ClearRefcountRequest,
) -> Result<ClearRefcountResponse> {
	ctx.auth().await?;
	let op_id = Uuid::new_v4();
	let op = SqliteOp::ClearRefcount {
		actor_id: path.actor_id.clone(),
		kind: body.kind.into(),
		txid: body.txid,
	};
	let record = create_publish_and_wait(
		ctx,
		op_id,
		OpKind::ClearRefcount,
		path.actor_id,
		body.namespace_id,
		op,
	)
	.await?;
	match record.result {
		Some(OpResult::ClearRefcount(_)) => Ok(ClearRefcountResponse { cleared: true }),
		other => bail!("sqlite clear refcount operation completed without clear result: {other:?}"),
	}
}

async fn create_publish_and_wait(
	ctx: ApiCtx,
	op_id: Uuid,
	op_kind: OpKind,
	actor_id: String,
	namespace_id: Option<Uuid>,
	op: SqliteOp,
) -> Result<AdminOpRecord> {
	let udb = ctx.udb_arc()?;
	create_and_publish(ctx, op_id, op_kind, actor_id.clone(), namespace_id, op).await?;
	wait_for_terminal(udb, &actor_id, op_id, INLINE_OP_TIMEOUT).await
}

async fn create_and_publish(
	ctx: ApiCtx,
	op_id: Uuid,
	op_kind: OpKind,
	actor_id: String,
	namespace_id: Option<Uuid>,
	op: SqliteOp,
) -> Result<()> {
	let audit = AuditFields {
		caller_id: caller_id(&ctx),
		request_origin_ts_ms: now_ms()?,
		namespace_id: namespace_id.unwrap_or_else(Uuid::nil),
	};
	let udb = ctx.udb_arc()?;
	admin::create_record(Arc::clone(&udb), op_id, op_kind, actor_id, audit.clone()).await?;
	let request = SqliteOpRequest {
		request_id: op_id,
		op,
		audit,
	};
	ctx.ups()?
		.publish(
			SqliteOpSubject,
			&encode_sqlite_op_request(request).context("encode sqlite op request")?,
			PublishOpts::one(),
		)
		.await
		.with_context(|| format!("publish {SQLITE_OP_SUBJECT} sqlite admin op"))?;
	Ok(())
}

async fn read_operation(
	udb: Arc<universaldb::Database>,
	actor_id: &str,
	op_id: Uuid,
) -> Result<AdminOpRecord> {
	let Some(record) = admin::read(udb, op_id).await? else {
		return Err(ApiNotFound.build());
	};
	if record.actor_id != actor_id {
		return Err(ApiNotFound.build());
	}
	Ok(record)
}

async fn wait_for_terminal(
	udb: Arc<universaldb::Database>,
	actor_id: &str,
	op_id: Uuid,
	timeout: Duration,
) -> Result<AdminOpRecord> {
	let deadline = tokio::time::Instant::now() + timeout;
	loop {
		let record = read_operation(Arc::clone(&udb), actor_id, op_id).await?;
		match record.status {
			OpStatus::Completed => return Ok(record),
			OpStatus::Failed | OpStatus::Orphaned => {
				return Err(ApiBadRequest {
					reason: format!("sqlite admin op ended with status {:?}", record.status),
				}
				.build());
			}
			OpStatus::Pending | OpStatus::InProgress => {}
		}

		let now = tokio::time::Instant::now();
		if now >= deadline {
			return Err(ApiBadRequest {
				reason: "sqlite admin op did not complete within inline timeout".to_string(),
			}
			.build());
		}
		tokio::time::sleep((deadline - now).min(Duration::from_millis(50))).await;
	}
}

#[derive(Clone)]
struct SseState {
	udb: Arc<universaldb::Database>,
	actor_id: String,
	op_id: Uuid,
	last_payload: Option<String>,
	deadline: tokio::time::Instant,
	done: bool,
}

async fn sse_next(mut state: SseState) -> Option<(std::result::Result<Event, Infallible>, SseState)> {
	if state.done || tokio::time::Instant::now() >= state.deadline {
		return None;
	}

	loop {
		let record = match read_operation(Arc::clone(&state.udb), &state.actor_id, state.op_id).await {
			Ok(record) => record,
			Err(err) => {
				let payload = serde_json::json!({
					"group": "api",
					"code": "not_found",
					"message": err.to_string(),
				})
				.to_string();
				state.done = true;
				return Some((Ok(Event::default().event("error").data(payload)), state));
			}
		};

		let payload = match serde_json::to_string(&record) {
			Ok(payload) => payload,
			Err(err) => {
				state.done = true;
				return Some((
					Ok(Event::default().event("error").data(
						serde_json::json!({
							"group": "api",
							"code": "internal_error",
							"message": err.to_string(),
						})
						.to_string(),
					)),
					state,
				));
			}
		};
		let terminal = is_terminal(record.status);
		if state.last_payload.as_deref() != Some(payload.as_str()) {
			state.last_payload = Some(payload.clone());
			state.done = terminal;
			return Some((Ok(Event::default().event("operation").data(payload)), state));
		}
		if terminal {
			return None;
		}

		let now = tokio::time::Instant::now();
		if now >= state.deadline {
			return None;
		}
		tokio::time::sleep((state.deadline - now).min(SSE_POLL_INTERVAL)).await;
	}
}

fn accepted(operation_id: Uuid) -> OperationAcceptedResponse {
	OperationAcceptedResponse {
		operation_id,
		status: "pending",
	}
}

fn caller_id(ctx: &ApiCtx) -> String {
	ctx.token()
		.map(|token| format!("bearer:{token}"))
		.unwrap_or_else(|| "api-public".to_string())
}

fn is_terminal(status: OpStatus) -> bool {
	matches!(
		status,
		OpStatus::Completed | OpStatus::Failed | OpStatus::Orphaned
	)
}

fn now_ms() -> Result<i64> {
	let elapsed = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH)
		.context("system clock was before unix epoch")?;
	i64::try_from(elapsed.as_millis()).context("sqlite admin op timestamp exceeded i64")
}

trait ApiCtxExt {
	fn udb_arc(&self) -> Result<Arc<universaldb::Database>>;
}

impl ApiCtxExt for ApiCtx {
	fn udb_arc(&self) -> Result<Arc<universaldb::Database>> {
		Ok(Arc::new(self.udb()?.deref().clone()))
	}
}

impl From<RestoreTargetBody> for RestoreTarget {
	fn from(value: RestoreTargetBody) -> Self {
		match value {
			RestoreTargetBody::Txid { txid } => Self::Txid(txid),
			RestoreTargetBody::TimestampMs { timestamp_ms } => Self::TimestampMs(timestamp_ms),
			RestoreTargetBody::LatestCheckpoint => Self::LatestCheckpoint,
			RestoreTargetBody::CheckpointTxid { txid } => Self::CheckpointTxid(txid),
		}
	}
}

impl From<RestoreModeBody> for RestoreMode {
	fn from(value: RestoreModeBody) -> Self {
		match value {
			RestoreModeBody::Apply => Self::Apply,
			RestoreModeBody::DryRun => Self::DryRun,
		}
	}
}

impl From<ForkModeBody> for ForkMode {
	fn from(value: ForkModeBody) -> Self {
		match value {
			ForkModeBody::Apply => Self::Apply,
			ForkModeBody::DryRun => Self::DryRun,
		}
	}
}

impl From<ForkDstBody> for ForkDstSpec {
	fn from(value: ForkDstBody) -> Self {
		match value {
			ForkDstBody::Allocate { dst_namespace_id } => Self::Allocate { dst_namespace_id },
			ForkDstBody::Existing { dst_actor_id } => Self::Existing { dst_actor_id },
		}
	}
}

impl From<RefcountKindBody> for RefcountKind {
	fn from(value: RefcountKindBody) -> Self {
		match value {
			RefcountKindBody::Checkpoint => Self::Checkpoint,
			RefcountKindBody::Delta => Self::Delta,
		}
	}
}
