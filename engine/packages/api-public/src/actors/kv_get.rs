use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use rivet_api_types::actors::kv_get::*;
use rivet_api_util::request_remote_datacenter_raw;
use rivet_util::Id;

use crate::ctx::ApiCtx;

#[utoipa::path(
	get,
	operation_id = "actors_kv_get",
	path = "/actors/{actor_id}/kv/keys/{key}",
	params(
		("actor_id" = Id, Path),
		("key" = String, Path),
		KvGetQuery,
	),
	responses(
		(status = 200, body = KvGetResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn kv_get(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<KvGetPath>,
	Query(query): Query<KvGetQuery>,
) -> Response {
	match kv_get_inner(ctx, path, query).await {
		Ok(response) => response,
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn kv_get_inner(ctx: ApiCtx, path: KvGetPath, query: KvGetQuery) -> Result<Response> {
	ctx.auth().await?;

	if path.actor_id.label() == ctx.config().dc_label() {
		let res = rivet_api_peer::actors::kv_get::kv_get(ctx.into(), path, query).await?;

		Ok(Json(res).into_response())
	} else {
		request_remote_datacenter_raw(
			&ctx,
			path.actor_id.label(),
			&format!(
				"/actors/{}/kv/keys/{}",
				path.actor_id,
				urlencoding::encode(&path.key)
			),
			axum::http::Method::GET,
			Some(&query),
			Option::<&()>::None,
		)
		.await
	}
}
