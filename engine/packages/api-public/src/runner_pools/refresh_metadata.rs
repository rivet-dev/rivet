use rivet_auth::*;
use anyhow::{Result, anyhow};
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;

use super::utils::refresh_runner_pool_metadata;
use crate::ctx::ApiCtx;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct RefreshMetadataQuery {
	pub namespace: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshMetadataPath {
	pub pool_name: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerPoolsRefreshMetadataRequestBody)]
pub struct RefreshMetadataRequest {}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerPoolsRefreshMetadataResponse)]
pub struct RefreshMetadataResponse {}

#[utoipa::path(
	post,
	operation_id = "runner_pools_refresh_metadata",
	path = "/runner-pools/{pool_name}/refresh-metadata",
	params(
		("pool_name" = String, Path),
		RefreshMetadataQuery,
	),
	request_body(content = RefreshMetadataRequest, content_type = "application/json"),
	responses(
		(status = 200, body = RefreshMetadataResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn refresh_metadata(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<RefreshMetadataPath>,
	Query(query): Query<RefreshMetadataQuery>,
	Json(body): Json<RefreshMetadataRequest>,
) -> Response {
	match refresh_metadata_inner(ctx, path, query, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(level = "debug", skip_all)]
async fn refresh_metadata_inner(
	ctx: ApiCtx,
	path: RefreshMetadataPath,
	query: RefreshMetadataQuery,
	_body: RefreshMetadataRequest,
) -> Result<RefreshMetadataResponse> {
	ctx.auth(
		AccessNamespaceScope::Name(query.namespace.clone()),
		ResourceKind::RunnerPool,
		TargetScope::Any,
		OperationKind::Create,
	)
	.await?;

	// Resolve namespace
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	// Fetch runner pools for all datacenters
	let pools: Vec<_> = ctx
		.config()
		.topology()
		.datacenters
		.iter()
		.map(|_dc| (namespace.namespace_id, path.pool_name.clone()))
		.collect();

	let runner_pools = ctx
		.op(pegboard::ops::envoy_pool::get::Input {
			pools,
			bypass_cache: true,
		})
		.await?;

	// Find first serverless config
	let (url, headers) = runner_pools
		.iter()
		.find_map(|runner_pool| {
			if let rivet_types::envoy_pools::EnvoyPoolKind::Serverless { url, headers, .. } =
				&runner_pool.config.kind
			{
				Some((url.clone(), headers.clone()))
			} else {
				None
			}
		})
		.ok_or_else(|| anyhow!("no serverless runner pool found"))?;

	refresh_runner_pool_metadata(ctx, namespace.namespace_id, path.pool_name, url, headers).await?;

	Ok(RefreshMetadataResponse {})
}
