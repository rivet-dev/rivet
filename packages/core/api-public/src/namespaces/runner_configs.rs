use anyhow::Result;
use axum::{
	extract::{Extension, Path, Query},
	http::HeaderMap,
	response::{IntoResponse, Json, Response},
};
use rivet_api_builder::{ApiCtx, ApiError};
use rivet_util::Id;

use rivet_api_client::request_remote_datacenter;
use rivet_api_peer::namespaces::runner_configs::*;

#[utoipa::path(
	get,
	operation_id = "namespaces_runner_configs_get",
	path = "/namespaces/{namespace_id}/runner-configs/{runner_name}",
	params(
		("namespace_id" = Id, Path),
		("runner_name" = String, Path),
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
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn get_inner(
	ctx: ApiCtx,
	headers: HeaderMap,
	path: GetPath,
	query: GetQuery,
) -> Result<GetResponse> {
	if ctx.config().is_leader() {
		rivet_api_peer::namespaces::runner_configs::get(ctx, path, query).await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter::<GetResponse>(
			ctx.config(),
			leader_dc.datacenter_label,
			&format!(
				"/namespaces/{}/runner-configs/{}",
				path.namespace_id, path.runner_name
			),
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
	operation_id = "namespaces_runner_configs_list",
	path = "/namespaces/{namespace_id}/runner-configs",
	params(
		("namespace_id" = Id, Path),
		ListQuery,
	),
	responses(
		(status = 200, body = ListResponse),
	),
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
	if ctx.config().is_leader() {
		rivet_api_peer::namespaces::runner_configs::list(ctx, path, query).await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter::<ListResponse>(
			ctx.config(),
			leader_dc.datacenter_label,
			&format!("/namespaces/{}/runner-configs", path.namespace_id),
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
	operation_id = "namespaces_runner_configs_upsert",
	path = "/namespaces/{namespace_id}/runner-configs/{runner_name}",
	params(
		("namespace_id" = Id, Path),
		("runner_name" = String, Path),
		UpsertQuery,
	),
	request_body(content = UpsertRequest, content_type = "application/json"),
	responses(
		(status = 200, body = UpsertResponse),
	),
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
	if ctx.config().is_leader() {
		rivet_api_peer::namespaces::runner_configs::upsert(ctx, path, query, body).await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter::<UpsertResponse>(
			ctx.config(),
			leader_dc.datacenter_label,
			&format!(
				"/namespaces/{}/runner-configs/{}",
				path.namespace_id, path.runner_name
			),
			axum::http::Method::PUT,
			headers,
			Option::<&()>::None,
			Some(&body),
		)
		.await
	}
}

#[utoipa::path(
	delete,
	operation_id = "namespaces_runner_configs_delete",
	path = "/namespaces/{namespace_id}/runner-configs/{runner_name}",
	params(
		("namespace_id" = Id, Path),
		("runner_name" = String, Path),
		DeleteQuery,
	),
	responses(
		(status = 200, body = DeleteResponse),
	),
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
	if ctx.config().is_leader() {
		rivet_api_peer::namespaces::runner_configs::delete(ctx, path, query).await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter::<DeleteResponse>(
			ctx.config(),
			leader_dc.datacenter_label,
			&format!(
				"/namespaces/{}/runner-configs/{}",
				path.namespace_id, path.runner_name
			),
			axum::http::Method::DELETE,
			headers,
			Some(&query),
			Option::<&()>::None,
		)
		.await
	}
}
