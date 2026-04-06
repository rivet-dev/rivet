use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use rivet_api_types::actors::sleep::*;
use rivet_api_util::request_remote_datacenter_raw;
use rivet_util::Id;

use crate::ctx::ApiCtx;

#[utoipa::path(
	post,
	operation_id = "actors_sleep",
	path = "/actors/{actor_id}/sleep",
	params(
		("actor_id" = Id, Path),
		SleepQuery,
	),
	request_body(content = SleepRequest, content_type = "application/json"),
	responses(
		(status = 200, body = SleepResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn sleep(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<SleepPath>,
	Query(query): Query<SleepQuery>,
	Json(body): Json<SleepRequest>,
) -> Response {
	match sleep_inner(ctx, path, query, body).await {
		Ok(response) => response,
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn sleep_inner(
	ctx: ApiCtx,
	path: SleepPath,
	query: SleepQuery,
	body: SleepRequest,
) -> Result<Response> {
	ctx.auth().await?;

	if path.actor_id.label() == ctx.config().dc_label() {
		let res = rivet_api_peer::actors::sleep::sleep(ctx.into(), path, query, body).await?;

		Ok(Json(res).into_response())
	} else {
		request_remote_datacenter_raw(
			&ctx,
			path.actor_id.label(),
			&format!("/actors/{}/sleep", path.actor_id),
			axum::http::Method::POST,
			Some(&query),
			Some(&body),
		)
		.await
	}
}
