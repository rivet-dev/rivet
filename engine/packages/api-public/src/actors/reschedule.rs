use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use rivet_api_types::actors::reschedule::*;
use rivet_api_util::request_remote_datacenter_raw;
use rivet_util::Id;

use crate::ctx::ApiCtx;

#[utoipa::path(
	post,
	operation_id = "actors_reschedule",
	path = "/actors/{actor_id}/reschedule",
	params(
		("actor_id" = Id, Path),
		RescheduleQuery,
	),
	request_body(content = RescheduleRequest, content_type = "application/json"),
	responses(
		(status = 200, body = RescheduleResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn reschedule(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ReschedulePath>,
	Query(query): Query<RescheduleQuery>,
	Json(body): Json<RescheduleRequest>,
) -> Response {
	match reschedule_inner(ctx, path, query, body).await {
		Ok(response) => response,
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn reschedule_inner(
	ctx: ApiCtx,
	path: ReschedulePath,
	query: RescheduleQuery,
	body: RescheduleRequest,
) -> Result<Response> {
	ctx.auth().await?;

	if path.actor_id.label() == ctx.config().dc_label() {
		let res =
			rivet_api_peer::actors::reschedule::reschedule(ctx.into(), path, query, body).await?;

		Ok(Json(res).into_response())
	} else {
		request_remote_datacenter_raw(
			&ctx,
			path.actor_id.label(),
			&format!("/actors/{}/reschedule", path.actor_id),
			axum::http::Method::POST,
			Some(&query),
			Some(&body),
		)
		.await
	}
}
