use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Query},
};
use rivet_api_peer::namespaces::*;
use rivet_api_types::namespaces::list::*;
use rivet_api_util::request_remote_datacenter;

use crate::ctx::ApiCtx;

#[utoipa::path(
	get,
	operation_id = "namespaces_list",
	path = "/namespaces",
	params(ListQuery),
	responses(
		(status = 200, body = ListResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn list(Extension(ctx): Extension<ApiCtx>, Query(query): Query<ListQuery>) -> Response {
	match list_inner(ctx, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn list_inner(ctx: ApiCtx, query: ListQuery) -> Result<ListResponse> {
	ctx.auth().await?;

	if ctx.config().is_leader() {
		rivet_api_peer::namespaces::list(ctx.into(), (), query).await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter::<ListResponse>(
			ctx.config(),
			leader_dc.datacenter_label,
			"/namespaces",
			axum::http::Method::GET,
			Some(&query),
			Option::<&()>::None,
		)
		.await
	}
}

#[utoipa::path(
	post,
	operation_id = "namespaces_create",
	path = "/namespaces",
	request_body(content = CreateRequest, content_type = "application/json"),
	responses(
		(status = 200, body = CreateResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn create(
	Extension(ctx): Extension<ApiCtx>,
	Json(body): Json<CreateRequest>,
) -> Response {
	match create_inner(ctx, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn create_inner(ctx: ApiCtx, body: CreateRequest) -> Result<CreateResponse> {
	ctx.auth().await?;

	if ctx.config().is_leader() {
		rivet_api_peer::namespaces::create(ctx.into(), (), (), body).await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter::<CreateResponse>(
			ctx.config(),
			leader_dc.datacenter_label,
			"/namespaces",
			axum::http::Method::POST,
			Option::<&()>::None,
			Some(&body),
		)
		.await
	}
}
