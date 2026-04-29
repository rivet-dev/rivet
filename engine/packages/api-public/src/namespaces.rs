use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use rivet_api_peer::namespaces::*;
use rivet_api_util::request_remote_datacenter;
use rivet_api_types::namespaces::list::*;

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

#[utoipa::path(
	get,
	operation_id = "namespaces_get_sqlite_config",
	path = "/namespaces/{ns_id}/sqlite-config",
	params(
		("ns_id" = rivet_util::Id, Path),
	),
	responses(
		(status = 200, body = namespace::types::SqliteNamespaceConfig),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn get_sqlite_config(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<SqliteConfigPath>,
) -> Response {
	match get_sqlite_config_inner(ctx, path).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn get_sqlite_config_inner(
	ctx: ApiCtx,
	path: SqliteConfigPath,
) -> Result<namespace::types::SqliteNamespaceConfig> {
	ctx.auth().await?;

	if ctx.config().is_leader() {
		rivet_api_peer::namespaces::get_sqlite_config(ctx.into(), path, ()).await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter::<namespace::types::SqliteNamespaceConfig>(
			ctx.config(),
			leader_dc.datacenter_label,
			&format!("/namespaces/{}/sqlite-config", path.ns_id),
			axum::http::Method::GET,
			Option::<&()>::None,
			Option::<&()>::None,
		)
		.await
	}
}

#[utoipa::path(
	put,
	operation_id = "namespaces_put_sqlite_config",
	path = "/namespaces/{ns_id}/sqlite-config",
	params(
		("ns_id" = rivet_util::Id, Path),
	),
	request_body(content = namespace::types::SqliteNamespaceConfig, content_type = "application/json"),
	responses(
		(status = 200, body = namespace::types::SqliteNamespaceConfig),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn put_sqlite_config(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<SqliteConfigPath>,
	Json(body): Json<namespace::types::SqliteNamespaceConfig>,
) -> Response {
	match put_sqlite_config_inner(ctx, path, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn put_sqlite_config_inner(
	ctx: ApiCtx,
	path: SqliteConfigPath,
	body: namespace::types::SqliteNamespaceConfig,
) -> Result<namespace::types::SqliteNamespaceConfig> {
	ctx.auth().await?;

	if ctx.config().is_leader() {
		rivet_api_peer::namespaces::put_sqlite_config(ctx.into(), path, (), body).await
	} else {
		let leader_dc = ctx.config().leader_dc()?;
		request_remote_datacenter::<namespace::types::SqliteNamespaceConfig>(
			ctx.config(),
			leader_dc.datacenter_label,
			&format!("/namespaces/{}/sqlite-config", path.ns_id),
			axum::http::Method::PUT,
			Option::<&()>::None,
			Some(&body),
		)
		.await
	}
}
