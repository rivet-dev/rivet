use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Path},
};
use rivet_api_util::request_remote_datacenter_raw;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ctx::ApiCtx;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KvGetPath {
	pub actor_id: Id,
	pub key: String,
}

#[derive(Serialize, ToSchema)]
#[schema(as = ActorsKvGetResponse)]
pub struct KvGetResponse {
	pub value: String,
	pub update_ts: i64,
}

#[utoipa::path(
	get,
	operation_id = "actors_kv_get",
	path = "/actors/{actor_id}/kv/keys/{key}",
	params(
		("actor_id" = Id, Path),
		("key" = String, Path),
	),
	responses(
		(status = 200, body = KvGetResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn kv_get(Extension(ctx): Extension<ApiCtx>, Path(path): Path<KvGetPath>) -> Response {
	match kv_get_inner(ctx, path).await {
		Ok(response) => response,
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn kv_get_inner(ctx: ApiCtx, path: KvGetPath) -> Result<Response> {
	use axum::Json;

	ctx.auth().await?;

	if path.actor_id.label() == ctx.config().dc_label() {
		let peer_path = rivet_api_peer::actors::kv_get::KvGetPath {
			actor_id: path.actor_id,
			key: path.key,
		};
		let peer_query = rivet_api_peer::actors::kv_get::KvGetQuery {};
		let res = rivet_api_peer::actors::kv_get::kv_get(ctx.into(), peer_path, peer_query).await?;

		Ok(Json(res).into_response())
	} else {
		request_remote_datacenter_raw(
			&ctx,
			path.actor_id.label(),
			&format!("/actors/{}/kv/keys/{}", path.actor_id, urlencoding::encode(&path.key)),
			axum::http::Method::GET,
			Option::<&()>::None,
			Option::<&()>::None,
		)
		.await
	}
}
