use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use rivet_api_types::actors::patch_metadata::*;
use rivet_api_util::request_remote_datacenter_raw;
use rivet_util::Id;

use crate::ctx::ApiCtx;

#[utoipa::path(
	patch,
	operation_id = "actors_patch_metadata",
	path = "/actors/{actor_id}/metadata",
	params(
		("actor_id" = Id, Path),
		PatchMetadataQuery,
	),
	request_body(content = PatchMetadataRequest, content_type = "application/json"),
	responses(
		(status = 200, body = PatchMetadataResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn patch_metadata(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<PatchMetadataPath>,
	Query(query): Query<PatchMetadataQuery>,
	Json(body): Json<PatchMetadataRequest>,
) -> Response {
	match patch_metadata_inner(ctx, path, query, body).await {
		Ok(response) => response,
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn patch_metadata_inner(
	ctx: ApiCtx,
	path: PatchMetadataPath,
	query: PatchMetadataQuery,
	body: PatchMetadataRequest,
) -> Result<Response> {
	ctx.auth().await?;

	if path.actor_id.label() == ctx.config().dc_label() {
		let res =
			rivet_api_peer::actors::patch_metadata::patch_metadata(ctx.into(), path, query, body)
				.await?;

		Ok(Json(res).into_response())
	} else {
		request_remote_datacenter_raw(
			&ctx,
			path.actor_id.label(),
			&format!("/actors/{}/metadata", path.actor_id),
			axum::http::Method::PATCH,
			Some(&query),
			Some(&body),
		)
		.await
	}
}
