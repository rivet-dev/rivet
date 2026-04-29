use std::{collections::BTreeMap, ops::Deref, sync::Arc};

use anyhow::{Context, Result};
use axum::{
	extract::ws::{Message, WebSocket, WebSocketUpgrade},
	response::{IntoResponse, Response},
};
use futures_util::TryStreamExt;
use rivet_api_builder::{
	ApiError, ApiUnauthorized,
	extract::{Extension, Json, Path, Query},
};
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use sqlite_storage::{
	admin::{AdminOpRecord, CheckpointView, OpKind, RetentionView, decode_admin_op_record},
	compactor,
	keys,
};
use subtle::ConstantTimeEq;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	tuple,
	utils::IsolationLevel::Snapshot,
};
use uuid::Uuid;

use crate::ctx::ApiCtx;

const ADMIN_OP_HISTORY_LIMIT: usize = 50;
const ADMIN_OP_HISTORY_MAX_LIMIT: usize = 100;
const ADMIN_OP_HISTORY_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Deserialize)]
pub struct ActorPath {
	actor_id: String,
}

#[derive(Debug, Deserialize)]
pub struct NamespacePath {
	ns_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct InspectorActorQuery {
	#[serde(default)]
	namespace_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct AdminOpsQuery {
	#[serde(default)]
	since: Option<i64>,
	#[serde(default)]
	limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointsResponse {
	pub checkpoints: Vec<CheckpointView>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminOpsResponse {
	pub operations: Vec<AdminOpRecord>,
	pub next_since: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NamespaceOverviewResponse {
	pub namespace_id: Uuid,
	pub storage_used_live_bytes: u64,
	pub storage_used_pitr_bytes: u64,
	pub checkpoint_count: u64,
	pub pinned_checkpoint_warnings: u64,
	pub recent_op_counts: BTreeMap<String, u64>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "route", rename_all = "snake_case")]
pub enum InspectorWsRequest {
	Checkpoints {
		actor_id: String,
		#[serde(default)]
		namespace_id: Option<Uuid>,
	},
	Retention {
		actor_id: String,
		#[serde(default)]
		namespace_id: Option<Uuid>,
	},
	AdminOps {
		actor_id: String,
		#[serde(default)]
		since: Option<i64>,
		#[serde(default)]
		limit: Option<usize>,
	},
	NamespaceOverview {
		namespace_id: Uuid,
	},
}

#[derive(Debug, Serialize)]
#[serde(tag = "route", content = "data", rename_all = "snake_case")]
enum InspectorWsResponse {
	Checkpoints(CheckpointsResponse),
	Retention(RetentionView),
	AdminOps(AdminOpsResponse),
	NamespaceOverview(NamespaceOverviewResponse),
	Error { message: String },
}

#[tracing::instrument(skip_all)]
pub async fn get_checkpoints(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Query(query): Query<InspectorActorQuery>,
) -> Response {
	match get_checkpoints_inner(ctx, path, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn get_checkpoints_inner(
	ctx: ApiCtx,
	path: ActorPath,
	query: InspectorActorQuery,
) -> Result<CheckpointsResponse> {
	inspector_auth(&ctx)?;
	let view = compactor::inspect_retention_view(
		ctx.udb_arc()?,
		path.actor_id,
		query.namespace_id.unwrap_or_else(Uuid::nil),
	)
	.await?;
	Ok(CheckpointsResponse {
		checkpoints: view.checkpoints,
	})
}

#[tracing::instrument(skip_all)]
pub async fn get_retention(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Query(query): Query<InspectorActorQuery>,
) -> Response {
	match get_retention_inner(ctx, path, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn get_retention_inner(
	ctx: ApiCtx,
	path: ActorPath,
	query: InspectorActorQuery,
) -> Result<RetentionView> {
	inspector_auth(&ctx)?;
	compactor::inspect_retention_view(
		ctx.udb_arc()?,
		path.actor_id,
		query.namespace_id.unwrap_or_else(Uuid::nil),
	)
	.await
}

#[tracing::instrument(skip_all)]
pub async fn get_admin_ops(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ActorPath>,
	Query(query): Query<AdminOpsQuery>,
) -> Response {
	match get_admin_ops_inner(ctx, path, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn get_admin_ops_inner(
	ctx: ApiCtx,
	path: ActorPath,
	query: AdminOpsQuery,
) -> Result<AdminOpsResponse> {
	inspector_auth(&ctx)?;
	list_admin_ops(
		ctx.udb_arc()?,
		path.actor_id,
		query.since,
		query.limit.unwrap_or(ADMIN_OP_HISTORY_LIMIT),
	)
	.await
}

#[tracing::instrument(skip_all)]
pub async fn get_namespace_overview(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<NamespacePath>,
) -> Response {
	match get_namespace_overview_inner(ctx, path).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn get_namespace_overview_inner(
	ctx: ApiCtx,
	path: NamespacePath,
) -> Result<NamespaceOverviewResponse> {
	inspector_auth(&ctx)?;
	namespace_overview(ctx.udb_arc()?, path.ns_id).await
}

#[tracing::instrument(skip_all)]
pub async fn websocket(
	Extension(ctx): Extension<ApiCtx>,
	ws: WebSocketUpgrade,
) -> Response {
	match inspector_auth(&ctx) {
		Ok(()) => ws.on_upgrade(move |socket| websocket_inner(ctx, socket)).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn websocket_inner(ctx: ApiCtx, mut socket: WebSocket) {
	while let Some(message) = socket.recv().await {
		let response = match message {
			Ok(Message::Text(text)) => handle_ws_request(&ctx, text.as_str()).await,
			Ok(Message::Close(_)) => break,
			Ok(Message::Ping(_) | Message::Pong(_) | Message::Binary(_)) => continue,
			Err(err) => InspectorWsResponse::Error {
				message: err.to_string(),
			},
		};
		let Ok(payload) = serde_json::to_string(&response) else {
			break;
		};
		if socket.send(Message::Text(payload.into())).await.is_err() {
			break;
		}
	}
}

async fn handle_ws_request(ctx: &ApiCtx, payload: &str) -> InspectorWsResponse {
	match handle_ws_request_inner(ctx, payload).await {
		Ok(response) => response,
		Err(err) => InspectorWsResponse::Error {
			message: err.to_string(),
		},
	}
}

async fn handle_ws_request_inner(ctx: &ApiCtx, payload: &str) -> Result<InspectorWsResponse> {
	let request: InspectorWsRequest = serde_json::from_str(payload)?;
	match request {
		InspectorWsRequest::Checkpoints {
			actor_id,
			namespace_id,
		} => {
			let view = compactor::inspect_retention_view(
				ctx.udb_arc()?,
				actor_id,
				namespace_id.unwrap_or_else(Uuid::nil),
			)
			.await?;
			Ok(InspectorWsResponse::Checkpoints(CheckpointsResponse {
				checkpoints: view.checkpoints,
			}))
		}
		InspectorWsRequest::Retention {
			actor_id,
			namespace_id,
		} => Ok(InspectorWsResponse::Retention(
			compactor::inspect_retention_view(
				ctx.udb_arc()?,
				actor_id,
				namespace_id.unwrap_or_else(Uuid::nil),
			)
			.await?,
		)),
		InspectorWsRequest::AdminOps {
			actor_id,
			since,
			limit,
		} => Ok(InspectorWsResponse::AdminOps(
			list_admin_ops(
				ctx.udb_arc()?,
				actor_id,
				since,
				limit.unwrap_or(ADMIN_OP_HISTORY_LIMIT),
			)
			.await?,
		)),
		InspectorWsRequest::NamespaceOverview { namespace_id } => Ok(
			InspectorWsResponse::NamespaceOverview(namespace_overview(ctx.udb_arc()?, namespace_id).await?),
		),
	}
}

async fn list_admin_ops(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	since: Option<i64>,
	limit: usize,
) -> Result<AdminOpsResponse> {
	let now_ms = now_ms()?;
	let cutoff = now_ms.saturating_sub(ADMIN_OP_HISTORY_WINDOW_MS);
	let since = since
		.map(|value| value.saturating_add(1))
		.unwrap_or(cutoff)
		.max(cutoff);
	let limit = limit.min(ADMIN_OP_HISTORY_MAX_LIMIT);
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let mut operations = Vec::new();
			let prefix = keys::meta_admin_op_prefix(&actor_id);
			for (_key, value) in tx_scan_prefix_values(&tx, &prefix).await? {
				let record = decode_admin_op_record(&value)
					.context("decode sqlite inspector admin op record")?;
				if record.created_at_ms >= since {
					operations.push(record);
				}
			}
			operations.sort_by_key(|record| (record.created_at_ms, record.operation_id));
			let next_since = if operations.len() > limit {
				limit.checked_sub(1).and_then(|idx| {
					operations.get(idx).map(|record| record.created_at_ms)
				})
			} else {
				None
			};
			operations.truncate(limit);
			Ok(AdminOpsResponse {
				operations,
				next_since,
			})
		}
	})
	.await
}

async fn namespace_overview(
	udb: Arc<universaldb::Database>,
	namespace_id: Uuid,
) -> Result<NamespaceOverviewResponse> {
	let now_ms = now_ms()?;
	let cutoff = now_ms.saturating_sub(ADMIN_OP_HISTORY_WINDOW_MS);
	let namespace_metric_id = Id::v1(namespace_id, 0);
	udb.run(move |tx| async move {
		let namespace_tx = tx.with_subspace(namespace::keys::subspace());
		let metrics_subspace = namespace::keys::subspace()
			.subspace(&namespace::keys::metric::MetricKey::subspace(namespace_metric_id));
		let mut storage_used_live_bytes = 0i64;
		let mut storage_used_pitr_bytes = 0i64;
		let mut checkpoint_count = 0i64;
		let mut pinned_checkpoint_warnings = 0i64;
		let mut metric_stream = namespace_tx.get_ranges_keyvalues(
			RangeOption {
				mode: StreamingMode::WantAll,
				..(&metrics_subspace).into()
			},
			Snapshot,
		);

		while let Some(entry) = metric_stream.try_next().await? {
			let (key, value) = namespace_tx.read_entry::<namespace::keys::metric::MetricKey>(&entry)?;
			match key.metric {
				namespace::keys::metric::Metric::SqliteStorageLiveUsed(_) => {
					storage_used_live_bytes = storage_used_live_bytes.saturating_add(value);
				}
				namespace::keys::metric::Metric::SqliteStoragePitrUsed(_) => {
					storage_used_pitr_bytes = storage_used_pitr_bytes.saturating_add(value);
				}
				namespace::keys::metric::Metric::SqliteCheckpointCount(_) => {
					checkpoint_count = checkpoint_count.saturating_add(value);
				}
				namespace::keys::metric::Metric::SqliteCheckpointPinned(_) => {
					pinned_checkpoint_warnings =
						pinned_checkpoint_warnings.saturating_add(value);
				}
				namespace::keys::metric::Metric::ActorAwake(_)
				| namespace::keys::metric::Metric::TotalActors(_)
				| namespace::keys::metric::Metric::KvStorageUsed(_)
				| namespace::keys::metric::Metric::KvRead(_)
				| namespace::keys::metric::Metric::KvWrite(_)
				| namespace::keys::metric::Metric::AlarmsSet(_)
				| namespace::keys::metric::Metric::GatewayIngress(_, _)
				| namespace::keys::metric::Metric::GatewayEgress(_, _)
				| namespace::keys::metric::Metric::Requests(_, _)
				| namespace::keys::metric::Metric::ActiveRequests(_, _)
				| namespace::keys::metric::Metric::SqliteCommitBytes(_)
				| namespace::keys::metric::Metric::SqliteReadBytes(_) => {}
			}
		}

		let mut recent_op_counts = BTreeMap::new();
		let sqlite_subspace = universaldb::Subspace::from(tuple::Subspace::from_bytes(vec![
			keys::SQLITE_SUBSPACE_PREFIX,
		]));
		let informal = tx.informal();
		let mut sqlite_stream = informal.get_ranges_keyvalues(
			RangeOption {
				mode: StreamingMode::WantAll,
				..RangeOption::from(&sqlite_subspace)
			},
			Snapshot,
		);
		while let Some(entry) = sqlite_stream.try_next().await? {
			let key = entry.key();
			if !key_has_admin_op_suffix(key) {
				continue;
			}
			let record = decode_admin_op_record(entry.value())
				.context("decode sqlite inspector admin op record")?;
			if record.audit.namespace_id == namespace_id && record.created_at_ms >= cutoff {
				*recent_op_counts.entry(op_kind_label(record.op_kind).to_string()).or_insert(0) +=
					1;
			}
		}

		Ok(NamespaceOverviewResponse {
			namespace_id,
			storage_used_live_bytes: nonnegative_i64_to_u64(storage_used_live_bytes),
			storage_used_pitr_bytes: nonnegative_i64_to_u64(storage_used_pitr_bytes),
			checkpoint_count: nonnegative_i64_to_u64(checkpoint_count),
			pinned_checkpoint_warnings: nonnegative_i64_to_u64(pinned_checkpoint_warnings),
			recent_op_counts,
		})
	})
	.await
}

fn inspector_auth(ctx: &ApiCtx) -> Result<()> {
	ctx.skip_auth();
	let Some(auth) = &ctx.config().auth else {
		return Ok(());
	};
	let Some(token) = ctx.token() else {
		return Err(ApiUnauthorized.build());
	};
	if token
		.as_bytes()
		.ct_ne(auth.admin_token.read().as_bytes())
		.into()
	{
		return Err(ApiUnauthorized.build());
	}
	Ok(())
}

fn key_has_admin_op_suffix(key: &[u8]) -> bool {
	key.windows(b"/META/admin_op/".len())
		.any(|window| window == b"/META/admin_op/")
}

fn op_kind_label(kind: OpKind) -> &'static str {
	match kind {
		OpKind::Restore => "restore",
		OpKind::Fork => "fork",
		OpKind::DescribeRetention => "describe_retention",
		OpKind::GetRetention => "get_retention",
		OpKind::SetRetention => "set_retention",
		OpKind::ClearRefcount => "clear_refcount",
	}
}

async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Snapshot,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

fn nonnegative_i64_to_u64(value: i64) -> u64 {
	u64::try_from(value.max(0)).unwrap_or(0)
}

fn now_ms() -> Result<i64> {
	Ok(std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)?
		.as_millis()
		.try_into()?)
}

trait ApiCtxExt {
	fn udb_arc(&self) -> Result<Arc<universaldb::Database>>;
}

impl ApiCtxExt for ApiCtx {
	fn udb_arc(&self) -> Result<Arc<universaldb::Database>> {
		Ok(Arc::new(self.udb()?.deref().clone()))
	}
}
