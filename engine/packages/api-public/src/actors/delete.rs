use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use rivet_api_types::actors::delete::*;
use rivet_api_util::request_remote_datacenter_raw;
use rivet_util::Id;

use crate::ctx::ApiCtx;

/// ## Datacenter Round Trips
///
/// 2 round trip:
/// - DELETE /actors/{}
/// - [api-peer] namespace::ops::resolve_for_name_global
#[utoipa::path(
	delete,
	operation_id = "actors_delete",
	path = "/actors/{actor_id}",
	params(
		("actor_id" = Id, Path),
		DeleteQuery,
	),
	responses(
		(status = 200, body = DeleteResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn delete(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<DeletePath>,
	Query(query): Query<DeleteQuery>,
) -> Response {
	match delete_inner(ctx, path, query).await {
		Ok(response) => response,
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn delete_inner(ctx: ApiCtx, path: DeletePath, query: DeleteQuery) -> Result<Response> {
	ctx.auth().await?;

	if path.actor_id.label() == ctx.config().dc_label() {
		let res = rivet_api_peer::actors::delete::delete(ctx.into(), path, query).await?;

		Ok(Json(res).into_response())
	} else {
		request_remote_datacenter_raw(
			&ctx,
			path.actor_id.label(),
			&format!("/actors/{}", path.actor_id),
			axum::http::Method::DELETE,
			Some(&query),
			Option::<&()>::None,
		)
		.await
	}
}
