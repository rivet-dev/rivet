use std::{
	convert::Infallible,
	ops::Deref,
	sync::Arc,
	time::{Duration, Instant, SystemTime},
};

use anyhow::{Context, Result, bail};
use axum::{
	http::StatusCode,
	response::{
		IntoResponse, Response,
		sse::{Event, KeepAlive, Sse},
	},
};
use futures_util::{TryStreamExt, stream};
use namespace::types::SqliteNamespaceConfig;
use parking_lot::Mutex;
use rivet_api_builder::{
	ApiBadRequest, ApiError, ApiNotFound,
	extract::{Extension, Json, Path},
};
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use sqlite_storage::{
	admin::{
		self, AdminOpRecord, AuditFields, ForkDstSpec, ForkMode, OpKind, OpResult, OpStatus,
		RefcountKind, RestoreMode, RestoreTarget, SQLITE_OP_SUBJECT, SqliteAdminError, SqliteOp,
		SqliteOpRequest, SqliteOpSubject, decode_admin_op_record, encode_admin_op_record,
		encode_sqlite_op_request,
	},
	keys,
	pump::types::RetentionConfig,
};
use universaldb::{RangeOption, options::StreamingMode, tuple, utils::IsolationLevel::Serializable};
use universalpubsub::PublishOpts;
use uuid::Uuid;

use crate::{actors::sqlite_admin_metrics, ctx::ApiCtx};

const INLINE_OP_TIMEOUT: Duration = Duration::from_secs(2);
const SSE_POLL_INTERVAL: Duration = Duration::from_millis(500);
const SSE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const SQLITE_ADMIN_AUDIT_TOPIC: &str = "sqlite_admin_audit";

type RateLimitKey = (Uuid, &'static str);

lazy_static::lazy_static! {
	static ref RATE_LIMITERS: scc::HashMap<RateLimitKey, Arc<Mutex<TokenBucket>>> = scc::HashMap::new();
}

#[derive(Debug)]
struct TokenBucket {
	tokens: f64,
	last_refill: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SqliteAdminAuditEvent {
	pub topic: &'static str,
	pub stage: AuditStage,
	pub operation_id: Uuid,
	pub op_kind: OpKind,
	pub actor_id: String,
	pub status: OpStatus,
	pub caller_id: String,
	pub namespace_id: Uuid,
	pub request_origin_ts_ms: i64,
	pub event_ts_ms: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditStage {
	Acked,
	Terminal,
}

#[cfg(debug_assertions)]
lazy_static::lazy_static! {
	static ref AUDIT_TEST_LOG: Mutex<Vec<SqliteAdminAuditEvent>> = Mutex::new(Vec::new());
}

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

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreModeBody {
	Apply,
	DryRun,
}

#[derive(Debug, Clone, Copy, Deserialize)]
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
	create_and_publish_restore(ctx, op_id, path.actor_id, body.namespace_id, op).await?;
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
	let record = wait_for_terminal(udb, &actor_id, op_id, INLINE_OP_TIMEOUT).await?;
	emit_audit_event(
		AuditStage::Terminal,
		record.operation_id,
		record.op_kind,
		&record.actor_id,
		record.status,
		&record.audit,
	)?;
	Ok(record)
}

async fn create_and_publish_restore(
	ctx: ApiCtx,
	op_id: Uuid,
	actor_id: String,
	namespace_id: Option<Uuid>,
	op: SqliteOp,
) -> Result<()> {
	create_and_publish_inner(
		ctx,
		op_id,
		OpKind::Restore,
		actor_id,
		namespace_id,
		op,
		true,
	)
	.await
}

async fn create_and_publish(
	ctx: ApiCtx,
	op_id: Uuid,
	op_kind: OpKind,
	actor_id: String,
	namespace_id: Option<Uuid>,
	op: SqliteOp,
) -> Result<()> {
	create_and_publish_inner(ctx, op_id, op_kind, actor_id, namespace_id, op, false).await
}

async fn create_and_publish_inner(
	ctx: ApiCtx,
	op_id: Uuid,
	op_kind: OpKind,
	actor_id: String,
	namespace_id: Option<Uuid>,
	op: SqliteOp,
	suspend_for_restore: bool,
) -> Result<()> {
	let preflight = prepare_admin_op(&ctx, &actor_id, namespace_id, &op).await?;
	let audit = AuditFields {
		caller_id: caller_id(&ctx),
		request_origin_ts_ms: now_ms()?,
		namespace_id: preflight.namespace_id,
	};
	let udb = ctx.udb_arc()?;
	create_record_after_gates(
		Arc::clone(&udb),
		op_id,
		op_kind,
		actor_id.clone(),
		audit.clone(),
		preflight.clone(),
	)
	.await?;
	let ups = ctx.ups()?;
	if suspend_for_restore {
		pegboard::actor_lifecycle::suspend_actor(
			&udb,
			&ups,
			actor_id.clone(),
			pegboard::actor_lifecycle::RESTORE_SUSPENSION_REASON,
			op_id,
		)
		.await?;
	}
	let request = SqliteOpRequest {
		request_id: op_id,
		op,
		audit: audit.clone(),
	};
	ups.publish(
			SqliteOpSubject,
			&encode_sqlite_op_request(request).context("encode sqlite op request")?,
			PublishOpts::one(),
	)
		.await
		.with_context(|| format!("publish {SQLITE_OP_SUBJECT} sqlite admin op"))?;
	emit_audit_event(AuditStage::Acked, op_id, op_kind, &actor_id, OpStatus::Pending, &audit)?;
	if suspend_for_restore {
		spawn_restore_resume_monitor(ctx, udb, actor_id, op_id, op_kind, audit);
	} else if matches!(op_kind, OpKind::Fork) {
		spawn_terminal_audit_monitor(udb, actor_id, op_id, op_kind, audit);
	}
	Ok(())
}

#[derive(Clone)]
struct AdminPreflight {
	namespace_id: Uuid,
	namespace_config: SqliteNamespaceConfig,
	src_actor_id: Option<String>,
}

async fn prepare_admin_op(
	ctx: &ApiCtx,
	actor_id: &str,
	namespace_hint: Option<Uuid>,
	op: &SqliteOp,
) -> Result<AdminPreflight> {
	let namespace_id = resolve_namespace_id(ctx, actor_id, namespace_hint).await?;
	let namespace_config = load_namespace_config(ctx, namespace_id).await?;
	authorize_sqlite_op(ctx, op, namespace_id, &namespace_config).await?;

	Ok(AdminPreflight {
		namespace_id,
		namespace_config,
		src_actor_id: fork_src_actor_id(op),
	})
}

async fn resolve_namespace_id(
	ctx: &ApiCtx,
	actor_id: &str,
	namespace_hint: Option<Uuid>,
) -> Result<Uuid> {
	if let Ok(actor_id) = Id::parse(actor_id) {
		let Some(actor) = ctx
			.op(pegboard::ops::actor::get_for_kv::Input { actor_id })
			.await?
		else {
			return Err(pegboard::errors::Actor::NotFound.build());
		};
		return id_to_uuid(actor.namespace_id);
	}

	Ok(namespace_hint.unwrap_or_else(Uuid::nil))
}

async fn load_namespace_config(
	ctx: &ApiCtx,
	namespace_id: Uuid,
) -> Result<SqliteNamespaceConfig> {
	if namespace_id == Uuid::nil() {
		return Ok(allow_all_sqlite_config());
	}

	ctx.op(namespace::ops::sqlite_config::get::Input {
		namespace_id: Id::v1(namespace_id, 0),
	})
	.await
}

async fn authorize_sqlite_op(
	ctx: &ApiCtx,
	op: &SqliteOp,
	src_namespace_id: Uuid,
	src_config: &SqliteNamespaceConfig,
) -> Result<()> {
	match op {
		SqliteOp::Restore { mode, .. } => {
			if !src_config.allow_pitr_read {
				return Err(SqliteAdminError::PitrDisabledForNamespace.build());
			}
			if matches!(mode, RestoreMode::Apply) && !src_config.allow_pitr_destructive {
				return Err(SqliteAdminError::PitrDestructiveDisabledForNamespace.build());
			}
		}
		SqliteOp::DescribeRetention { .. } | SqliteOp::GetRetention { .. } => {
			if !src_config.allow_pitr_read {
				return Err(SqliteAdminError::PitrDisabledForNamespace.build());
			}
		}
		SqliteOp::SetRetention { .. } | SqliteOp::ClearRefcount { .. } => {
			if !src_config.allow_pitr_admin {
				return Err(SqliteAdminError::PitrAdminDisabledForNamespace.build());
			}
		}
		SqliteOp::Fork {
			dst,
			src_actor_id,
			mode: _,
			target: _,
		} => {
			if !src_config.allow_fork {
				return Err(SqliteAdminError::ForkDisabledForNamespace.build());
			}

			let dst_namespace_id = match dst {
				ForkDstSpec::Allocate { dst_namespace_id } => *dst_namespace_id,
				ForkDstSpec::Existing { dst_actor_id } => {
					resolve_namespace_id(ctx, dst_actor_id, Some(src_namespace_id)).await?
				}
			};
			let dst_config = if dst_namespace_id == src_namespace_id {
				src_config.clone()
			} else {
				load_namespace_config(ctx, dst_namespace_id).await?
			};
			if !dst_config.allow_fork {
				tracing::info!(
					src_actor_id,
					dst_namespace_id = %dst_namespace_id,
					"sqlite fork rejected by destination namespace capability"
				);
				return Err(SqliteAdminError::ForkDisabledForNamespace.build());
			}
		}
	}

	Ok(())
}

async fn create_record_after_gates(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	op_kind: OpKind,
	actor_id: String,
	audit: AuditFields,
	preflight: AdminPreflight,
) -> Result<()> {
	check_rate_limit(preflight.namespace_id, op_kind, preflight.namespace_config.admin_op_rate_per_min)?;
	let now_ms = now_ms()?;
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		let audit = audit.clone();
		let preflight = preflight.clone();

		async move {
			check_concurrent_gates(&tx, op_kind, &preflight).await?;
			let record = AdminOpRecord {
				operation_id: op_id,
				op_kind,
				actor_id: actor_id.clone(),
				created_at_ms: now_ms,
				last_progress_at_ms: now_ms,
				status: OpStatus::Pending,
				holder_id: None,
				progress: None,
				result: None,
				audit,
			};
			tx.informal().set(
				&keys::meta_admin_op_key(&actor_id, op_id),
				&encode_admin_op_record(record).context("encode sqlite admin op")?,
			);
			Ok(())
		}
	})
	.await
}

fn check_rate_limit(namespace_id: Uuid, op_kind: OpKind, rate_per_min: u32) -> Result<()> {
	if namespace_id == Uuid::nil() {
		return Ok(());
	}
	if rate_per_min == 0 {
		record_rate_limited(namespace_id);
		return Err(SqliteAdminError::AdminOpRateLimited {
			retry_after_ms: 60_000,
		}
		.build());
	}

	let now = Instant::now();
	let key = (namespace_id, op_kind_label(op_kind));
	let limiter = RATE_LIMITERS
		.entry_sync(key)
		.or_insert_with(|| {
			Arc::new(Mutex::new(TokenBucket {
				tokens: rate_per_min as f64,
				last_refill: now,
			}))
		})
		.get()
		.clone();
	let mut bucket = limiter.lock();
	let capacity = rate_per_min as f64;
	let elapsed_ms = now.duration_since(bucket.last_refill).as_millis() as f64;
	bucket.tokens = (bucket.tokens + elapsed_ms * capacity / 60_000.0).min(capacity);
	bucket.last_refill = now;
	if bucket.tokens >= 1.0 {
		bucket.tokens -= 1.0;
		return Ok(());
	}

	let retry_after_ms = ((1.0 - bucket.tokens) * 60_000.0 / capacity).ceil() as u64;
	record_rate_limited(namespace_id);
	Err(SqliteAdminError::AdminOpRateLimited { retry_after_ms }.build())
}

async fn check_concurrent_gates(
	tx: &universaldb::Transaction,
	op_kind: OpKind,
	preflight: &AdminPreflight,
) -> Result<()> {
	if preflight.namespace_id == Uuid::nil() || !matches!(op_kind, OpKind::Restore | OpKind::Fork) {
		return Ok(());
	}

	let mut namespace_inflight = 0_u32;
	let mut src_fork_inflight = 0_u32;
	let src_actor_id = preflight.src_actor_id.as_deref();
	let sqlite_subspace =
		universaldb::Subspace::from(tuple::Subspace::from_bytes(vec![keys::SQLITE_SUBSPACE_PREFIX]));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&sqlite_subspace)
		},
		Serializable,
	);

	while let Some(entry) = stream.try_next().await? {
		let key = entry.key();
		if !key_has_admin_op_suffix(key) {
			continue;
		}
		let record = decode_admin_op_record(entry.value()).context("decode sqlite admin op record")?;
		if record.audit.namespace_id != preflight.namespace_id || !is_inflight(record.status) {
			continue;
		}
		if matches!(record.op_kind, OpKind::Restore | OpKind::Fork) {
			namespace_inflight = namespace_inflight.saturating_add(1);
		}
		if matches!(record.op_kind, OpKind::Fork) && src_actor_id == Some(record.actor_id.as_str()) {
			src_fork_inflight = src_fork_inflight.saturating_add(1);
		}
	}

	if namespace_inflight >= preflight.namespace_config.concurrent_admin_ops {
		record_rate_limited(preflight.namespace_id);
		return Err(SqliteAdminError::AdminOpRateLimited { retry_after_ms: 0 }.build());
	}
	if matches!(op_kind, OpKind::Fork)
		&& src_fork_inflight >= preflight.namespace_config.concurrent_forks_per_src
	{
		record_rate_limited(preflight.namespace_id);
		return Err(SqliteAdminError::AdminOpRateLimited { retry_after_ms: 0 }.build());
	}

	Ok(())
}

fn spawn_restore_resume_monitor(
	ctx: ApiCtx,
	udb: Arc<universaldb::Database>,
	actor_id: String,
	op_id: Uuid,
	op_kind: OpKind,
	audit: AuditFields,
) {
	tokio::spawn(async move {
		match wait_for_terminal(Arc::clone(&udb), &actor_id, op_id, SSE_TIMEOUT).await {
			Ok(record) if record.status == OpStatus::Completed => {
				if let Err(err) = emit_audit_event(
					AuditStage::Terminal,
					record.operation_id,
					record.op_kind,
					&record.actor_id,
					record.status,
					&record.audit,
				) {
					tracing::error!(actor_id = %actor_id, %op_id, ?err, "failed to emit sqlite admin audit terminal event");
				}
				let ups = match ctx.ups() {
					Ok(ups) => ups,
					Err(err) => {
						tracing::error!(actor_id = %actor_id, %op_id, ?err, "failed to load ups for restore resume");
						return;
					}
				};
				if let Err(err) = pegboard::actor_lifecycle::resume_actor(&udb, &ups, actor_id.clone()).await {
					tracing::error!(actor_id = %actor_id, %op_id, ?err, "failed to resume actor after sqlite restore");
				}
			}
			Ok(record) => {
				if let Err(err) = emit_audit_event(
					AuditStage::Terminal,
					record.operation_id,
					record.op_kind,
					&record.actor_id,
					record.status,
					&record.audit,
				) {
					tracing::error!(actor_id = %actor_id, %op_id, ?err, "failed to emit sqlite admin audit terminal event");
				}
				tracing::warn!(
					actor_id = %actor_id,
					%op_id,
					status = ?record.status,
					"sqlite restore finished without completion; leaving actor suspended"
				);
			}
			Err(err) => {
				if let Err(audit_err) = emit_audit_event(
					AuditStage::Terminal,
					op_id,
					op_kind,
					&actor_id,
					OpStatus::Failed,
					&audit,
				) {
					tracing::error!(actor_id = %actor_id, %op_id, ?audit_err, "failed to emit sqlite admin audit failure event");
				}
				tracing::warn!(actor_id = %actor_id, %op_id, ?err, "sqlite restore monitor ended; leaving actor suspended");
			}
		}
	});
}

fn spawn_terminal_audit_monitor(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	op_id: Uuid,
	op_kind: OpKind,
	audit: AuditFields,
) {
	tokio::spawn(async move {
		let event = match wait_for_terminal(udb, &actor_id, op_id, SSE_TIMEOUT).await {
			Ok(record) => emit_audit_event(
				AuditStage::Terminal,
				record.operation_id,
				record.op_kind,
				&record.actor_id,
				record.status,
				&record.audit,
			),
			Err(_) => emit_audit_event(
				AuditStage::Terminal,
				op_id,
				op_kind,
				&actor_id,
				OpStatus::Failed,
				&audit,
			),
		};
		if let Err(err) = event {
			tracing::error!(actor_id = %actor_id, %op_id, ?err, "failed to emit sqlite admin audit terminal event");
		}
	});
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

fn allow_all_sqlite_config() -> SqliteNamespaceConfig {
	SqliteNamespaceConfig {
		allow_pitr_read: true,
		allow_pitr_destructive: true,
		allow_pitr_admin: true,
		allow_fork: true,
		admin_op_rate_per_min: u32::MAX,
		concurrent_admin_ops: u32::MAX,
		concurrent_forks_per_src: u32::MAX,
		..SqliteNamespaceConfig::default()
	}
}

fn id_to_uuid(id: Id) -> Result<Uuid> {
	let bytes = id.as_bytes();
	Uuid::from_slice(&bytes[1..17]).context("decode namespace uuid from id")
}

fn fork_src_actor_id(op: &SqliteOp) -> Option<String> {
	match op {
		SqliteOp::Fork { src_actor_id, .. } => Some(src_actor_id.clone()),
		SqliteOp::Restore { .. }
		| SqliteOp::DescribeRetention { .. }
		| SqliteOp::GetRetention { .. }
		| SqliteOp::SetRetention { .. }
		| SqliteOp::ClearRefcount { .. } => None,
	}
}

fn op_kind_label(op_kind: OpKind) -> &'static str {
	match op_kind {
		OpKind::Restore => "restore",
		OpKind::Fork => "fork",
		OpKind::DescribeRetention => "describe_retention",
		OpKind::GetRetention => "get_retention",
		OpKind::SetRetention => "set_retention",
		OpKind::ClearRefcount => "clear_refcount",
	}
}

fn record_rate_limited(namespace_id: Uuid) {
	sqlite_admin_metrics::SQLITE_ADMIN_OP_RATE_LIMITED_TOTAL
		.with_label_values(&[namespace_id.to_string().as_str()])
		.inc();
}

fn key_has_admin_op_suffix(key: &[u8]) -> bool {
	key.windows(b"/META/admin_op/".len())
		.any(|window| window == b"/META/admin_op/")
}

fn is_inflight(status: OpStatus) -> bool {
	matches!(status, OpStatus::Pending | OpStatus::InProgress)
}

fn emit_audit_event(
	stage: AuditStage,
	operation_id: Uuid,
	op_kind: OpKind,
	actor_id: &str,
	status: OpStatus,
	audit: &AuditFields,
) -> Result<()> {
	let event = SqliteAdminAuditEvent {
		topic: SQLITE_ADMIN_AUDIT_TOPIC,
		stage,
		operation_id,
		op_kind,
		actor_id: actor_id.to_string(),
		status,
		caller_id: audit.caller_id.clone(),
		namespace_id: audit.namespace_id,
		request_origin_ts_ms: audit.request_origin_ts_ms,
		event_ts_ms: now_ms()?,
	};
	let payload = serde_json::to_string(&event).context("encode sqlite admin audit event")?;
	tracing::info!(
		topic = SQLITE_ADMIN_AUDIT_TOPIC,
		payload = %payload,
		operation_id = %operation_id,
		op_kind = ?op_kind,
		status = ?status,
		actor_id,
		namespace_id = %audit.namespace_id,
		caller_id = %audit.caller_id,
		"sqlite admin audit event"
	);
	#[cfg(debug_assertions)]
	test_hooks::record_audit_event(event);
	Ok(())
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

#[cfg(debug_assertions)]
pub mod test_hooks {
	use super::*;

	pub fn take_audit_log() -> Vec<SqliteAdminAuditEvent> {
		std::mem::take(&mut *AUDIT_TEST_LOG.lock())
	}

	pub(super) fn record_audit_event(event: SqliteAdminAuditEvent) {
		AUDIT_TEST_LOG.lock().push(event);
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
