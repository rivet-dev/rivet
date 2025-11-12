use anyhow::{Result, anyhow};
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;

use super::utils::refresh_runner_config_metadata;
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
	pub runner_name: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerConfigsRefreshMetadataRequestBody)]
pub struct RefreshMetadataRequest {}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerConfigsRefreshMetadataResponse)]
pub struct RefreshMetadataResponse {}

#[utoipa::path(
	post,
	operation_id = "runner_configs_refresh_metadata",
	path = "/runner-configs/{runner_name}/refresh-metadata",
	params(
		("runner_name" = String, Path),
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

#[tracing::instrument(skip_all)]
async fn refresh_metadata_inner(
	ctx: ApiCtx,
	path: RefreshMetadataPath,
	query: RefreshMetadataQuery,
	_body: RefreshMetadataRequest,
) -> Result<RefreshMetadataResponse> {
	ctx.auth().await?;

	// Resolve namespace
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	// Fetch runner configs for all datacenters
	let runners: Vec<_> = ctx
		.config()
		.topology()
		.datacenters
		.iter()
		.map(|_dc| (namespace.namespace_id, path.runner_name.clone()))
		.collect();

	let runner_configs = ctx
		.op(pegboard::ops::runner_config::get::Input {
			runners,
			bypass_cache: true,
		})
		.await?;

	// Find first serverless config
	let (url, headers) = runner_configs
		.iter()
		.find_map(|runner_config| {
			if let rivet_types::runner_configs::RunnerConfigKind::Serverless {
				url, headers, ..
			} = &runner_config.config.kind
			{
				Some((url.clone(), headers.clone()))
			} else {
				None
			}
		})
		.ok_or_else(|| anyhow!("no serverless runner config found"))?;

	refresh_runner_config_metadata(ctx, namespace.namespace_id, path.runner_name, url, headers)
		.await?;

	Ok(RefreshMetadataResponse {})
}
