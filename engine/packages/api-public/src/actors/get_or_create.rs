use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Query},
};
use rivet_api_types::actors::get_or_create::{
	GetOrCreateQuery, GetOrCreateRequest, GetOrCreateResponse,
};
use rivet_api_util::request_remote_datacenter;

use crate::ctx::ApiCtx;

/// ## Datacenter Round Trips
///
/// **If actor exists**
///
/// 2 round trips:
/// - namespace::ops::resolve_for_name_global
/// - GET /actors/{}
///
/// **If actor does not exist and is created in the current datacenter:**
///
/// 2 round trips:
/// - namespace::ops::resolve_for_name_global
/// - [pegboard::workflows::actor] Create actor workflow (includes Epoxy key allocation)
///
/// **If actor does not exist and is created in a different datacenter:**
///
/// 3 round trips:
/// - namespace::ops::resolve_for_name_global
/// - POST /actors to remote datacenter
/// - [pegboard::workflows::actor] Create actor workflow (includes Epoxy key allocation)
///
/// actor::get will always be in the same datacenter.
///
/// ## Optimized Alternative Routes
#[utoipa::path(
	put,
	operation_id = "actors_get_or_create",
	path = "/actors",
	params(GetOrCreateQuery),
	request_body(content = GetOrCreateRequest, content_type = "application/json"),
	responses(
		(status = 200, body = GetOrCreateResponse),
	),
)]
pub async fn get_or_create(
	Extension(ctx): Extension<ApiCtx>,
	Query(query): Query<GetOrCreateQuery>,
	Json(body): Json<GetOrCreateRequest>,
) -> Response {
	match get_or_create_inner(ctx, query, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn get_or_create_inner(
	ctx: ApiCtx,
	query: GetOrCreateQuery,
	body: GetOrCreateRequest,
) -> Result<GetOrCreateResponse> {
	ctx.skip_auth();

	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	let target_dc_label = super::utils::find_dc_for_actor_creation(
		&ctx,
		namespace.namespace_id,
		&query.namespace,
		&body.runner_name_selector,
		body.datacenter.as_ref().map(String::as_str),
	)
	.await?;

	let query = GetOrCreateQuery {
		namespace: query.namespace,
	};

	if target_dc_label == ctx.config().dc_label() {
		rivet_api_peer::actors::get_or_create::get_or_create(ctx.into(), (), query, body).await
	} else {
		request_remote_datacenter::<GetOrCreateResponse>(
			ctx.config(),
			target_dc_label,
			"/actors",
			axum::http::Method::PUT,
			Some(&query),
			Some(&body),
		)
		.await
	}
}
