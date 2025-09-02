use anyhow::Result;
use axum::{
	extract::{Extension, Path, Query},
	http::HeaderMap,
	response::{IntoResponse, Json, Response},
};
use rivet_api_builder::{ApiCtx, ApiError};
use rivet_util::Id;

use rivet_api_client::{request_remote_datacenter, request_remote_datacenter_raw};
use rivet_api_peer::namespaces::*;

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
	if ctx.config().is_leader() {
		rivet_api_peer::namespaces::list(ctx, (), query).await
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
	get,
	operation_id = "namespaces_get",
	path = "/namespaces/{namespace_id}",
	params(
		("namespace_id" = Id, Path),
		GetQuery,
	),
	responses(
		(status = 200, body = GetResponse),
	),
)]
pub async fn get(
	Extension(ctx): Extension<ApiCtx>,
	headers: HeaderMap,
	Path(path): Path<GetPath>,
	Query(query): Query<GetQuery>,
) -> Response {
	match get_inner(ctx, headers, path, query).await {
		Ok(response) => response,
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn get_inner(
	ctx: ApiCtx,
	headers: HeaderMap,
	path: GetPath,
	query: GetQuery,
) -> Result<Response> {
	if ctx.config().is_leader() {
		let res = rivet_api_peer::namespaces::get(ctx, path, query).await?;
		Ok(Json(res).into_response())
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter_raw(
			&ctx,
			leader_dc.datacenter_label,
			&format!("/namespaces/{}", path.namespace_id),
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
	if ctx.config().is_leader() {
		rivet_api_peer::namespaces::create(ctx, (), (), body).await
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
