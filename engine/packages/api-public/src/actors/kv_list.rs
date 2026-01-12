use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Path, Query},
};
use rivet_api_util::request_remote_datacenter_raw;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ctx::ApiCtx;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KvListPath {
	pub actor_id: Id,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KvListQuery {
	/// Base64 encoded key prefix to filter by
	pub prefix: Option<String>,
	/// Number of results to return (default 100, max 1000)
	pub limit: Option<usize>,
	/// Whether to reverse the order
	pub reverse: Option<bool>,
}

#[derive(Serialize, ToSchema)]
#[schema(as = ActorsKvListResponse)]
pub struct KvListResponse {
	pub entries: Vec<KvEntry>,
}

#[derive(Serialize, ToSchema)]
pub struct KvEntry {
	/// Key encoded in base64
	pub key: String,
	/// Value encoded in base64
	pub value: String,
	pub update_ts: i64,
}

#[utoipa::path(
	get,
	operation_id = "actors_kv_list",
	path = "/actors/{actor_id}/kv/keys",
	params(
		("actor_id" = Id, Path),
		("prefix" = Option<String>, Query, description = "Base64 encoded key prefix to filter by"),
		("limit" = Option<usize>, Query, description = "Number of results to return (default 100, max 1000)"),
		("reverse" = Option<bool>, Query, description = "Whether to reverse the order"),
	),
	responses(
		(status = 200, body = KvListResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn kv_list(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<KvListPath>,
	Query(query): Query<KvListQuery>,
) -> Response {
	match kv_list_inner(ctx, path, query).await {
		Ok(response) => response,
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn kv_list_inner(ctx: ApiCtx, path: KvListPath, query: KvListQuery) -> Result<Response> {
	use axum::Json;

	ctx.auth().await?;

	if path.actor_id.label() == ctx.config().dc_label() {
		let peer_path = rivet_api_peer::actors::kv_list::KvListPath {
			actor_id: path.actor_id,
		};
		let peer_query = rivet_api_peer::actors::kv_list::KvListQuery {
			prefix: query.prefix,
			limit: query.limit,
			reverse: query.reverse,
		};
		let res = rivet_api_peer::actors::kv_list::kv_list(ctx.into(), peer_path, peer_query).await?;

		Ok(Json(res).into_response())
	} else {
		let mut url = format!("/actors/{}/kv/keys", path.actor_id);
		let mut query_params = vec![];
		
		if let Some(prefix) = query.prefix {
			query_params.push(format!("prefix={}", urlencoding::encode(&prefix)));
		}
		if let Some(limit) = query.limit {
			query_params.push(format!("limit={}", limit));
		}
		if let Some(reverse) = query.reverse {
			query_params.push(format!("reverse={}", reverse));
		}
		
		if !query_params.is_empty() {
			url.push_str("?");
			url.push_str(&query_params.join("&"));
		}

		request_remote_datacenter_raw(
			&ctx,
			path.actor_id.label(),
			&url,
			axum::http::Method::GET,
			Option::<&()>::None,
			Option::<&()>::None,
		)
		.await
	}
}
