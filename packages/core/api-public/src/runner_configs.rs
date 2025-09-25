use anyhow::Result;
use axum::{
	extract::{Extension, Path, Query},
	http::HeaderMap,
	response::{IntoResponse, Json, Response},
};
use rivet_api_builder::ApiError;

use rivet_api_peer::runner_configs::*;
use rivet_api_util::request_remote_datacenter;

use crate::ctx::ApiCtx;

#[utoipa::path(
	get,
	operation_id = "runner_configs_list",
	path = "/runner-configs",
	params(
		ListQuery,
	),
	responses(
		(status = 200, body = ListResponse),
	),
	security(("bearer_auth" = [])),
)]
pub async fn list(
	Extension(ctx): Extension<ApiCtx>,
	headers: HeaderMap,
	Path(path): Path<ListPath>,
	Query(query): Query<ListQuery>,
) -> Response {
	match list_inner(ctx, headers, path, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn list_inner(
	ctx: ApiCtx,
	headers: HeaderMap,
	path: ListPath,
	query: ListQuery,
) -> Result<ListResponse> {
	ctx.auth().await?;

	if ctx.config().is_leader() {
		rivet_api_peer::runner_configs::list(ctx.into(), path, query).await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter::<ListResponse>(
			ctx.config(),
			leader_dc.datacenter_label,
			"/runner-configs",
			axum::http::Method::GET,
			headers,
			Some(&query),
			Option::<&()>::None,
		)
		.await
	}
}

#[utoipa::path(
	put,
	operation_id = "runner_configs_upsert",
	path = "/runner-configs/{runner_name}",
	params(
		("runner_name" = String, Path),
		UpsertQuery,
	),
	request_body(content = UpsertRequest, content_type = "application/json"),
	responses(
		(status = 200, body = UpsertResponse),
	),
	security(("bearer_auth" = [])),
)]
pub async fn upsert(
	Extension(ctx): Extension<ApiCtx>,
	headers: HeaderMap,
	Path(path): Path<UpsertPath>,
	Query(query): Query<UpsertQuery>,
	Json(body): Json<UpsertRequest>,
) -> Response {
	match upsert_inner(ctx, headers, path, query, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn upsert_inner(
	ctx: ApiCtx,
	headers: HeaderMap,
	path: UpsertPath,
	query: UpsertQuery,
	body: UpsertRequest,
) -> Result<UpsertResponse> {
	ctx.auth().await?;

	if ctx.config().is_leader() {
		rivet_api_peer::runner_configs::upsert(ctx.into(), path, query, body).await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter::<UpsertResponse>(
			ctx.config(),
			leader_dc.datacenter_label,
			&format!("/runner-configs/{}", path.runner_name),
			axum::http::Method::PUT,
			headers,
			Some(&query),
			Some(&body),
		)
		.await
	}
}

#[utoipa::path(
	delete,
	operation_id = "runner_configs_delete",
	path = "/runner-configs/{runner_name}",
	params(
		("runner_name" = String, Path),
		DeleteQuery,
	),
	responses(
		(status = 200, body = DeleteResponse),
	),
	security(("bearer_auth" = [])),
)]
pub async fn delete(
	Extension(ctx): Extension<ApiCtx>,
	headers: HeaderMap,
	Path(path): Path<DeletePath>,
	Query(query): Query<DeleteQuery>,
) -> Response {
	match delete_inner(ctx, headers, path, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn delete_inner(
	ctx: ApiCtx,
	headers: HeaderMap,
	path: DeletePath,
	query: DeleteQuery,
) -> Result<DeleteResponse> {
	ctx.auth().await?;

	if ctx.config().is_leader() {
		rivet_api_peer::runner_configs::delete(ctx.into(), path, query).await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter::<DeleteResponse>(
			ctx.config(),
			leader_dc.datacenter_label,
			&format!("/runner-configs/{}", path.runner_name),
			axum::http::Method::DELETE,
			headers,
			Some(&query),
			Option::<&()>::None,
		)
		.await
	}
}
