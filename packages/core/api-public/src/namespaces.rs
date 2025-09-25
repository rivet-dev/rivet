use anyhow::Result;
use axum::{
	extract::{Extension, Query},
	http::HeaderMap,
	response::{IntoResponse, Json, Response},
};
use rivet_api_builder::ApiError;
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
)]
pub async fn list(
	Extension(ctx): Extension<ApiCtx>,
	headers: HeaderMap,
	Query(query): Query<ListQuery>,
) -> Response {
	match list_inner(ctx, headers, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn list_inner(ctx: ApiCtx, headers: HeaderMap, query: ListQuery) -> Result<ListResponse> {
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
			headers,
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
)]
pub async fn create(
	Extension(ctx): Extension<ApiCtx>,
	headers: HeaderMap,
	Json(body): Json<CreateRequest>,
) -> Response {
	match create_inner(ctx, headers, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn create_inner(
	ctx: ApiCtx,
	headers: HeaderMap,
	body: CreateRequest,
) -> Result<CreateResponse> {
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
			headers,
			Option::<&()>::None,
			Some(&body),
		)
		.await
	}
}
