use std::collections::HashMap;

use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Query},
};
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;

use super::utils::{ServerlessMetadataError, fetch_serverless_runner_metadata};
use crate::ctx::ApiCtx;

#[derive(Debug, Serialize, Deserialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ServerlessHealthCheckQuery {
	// NOTE: Only used in ee for ACL
	pub namespace: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerConfigsServerlessHealthCheckRequest)]
pub struct ServerlessHealthCheckRequest {
	pub url: String,
	#[serde(default)]
	pub headers: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = RunnerConfigsServerlessHealthCheckResponse)]
pub enum ServerlessHealthCheckResponse {
	Success { version: String },
	Failure { error: ServerlessMetadataError },
}

#[utoipa::path(
	post,
	operation_id = "runner_configs_serverless_health_check",
	path = "/runner-configs/serverless-health-check",
	params(
		ServerlessHealthCheckQuery,
	),
	request_body(content = ServerlessHealthCheckRequest, content_type = "application/json"),
	responses(
		(status = 200, body = ServerlessHealthCheckResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn serverless_health_check(
	Extension(ctx): Extension<ApiCtx>,
	Query(query): Query<ServerlessHealthCheckQuery>,
	Json(body): Json<ServerlessHealthCheckRequest>,
) -> Response {
	match serverless_health_check_inner(ctx, query, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn serverless_health_check_inner(
	ctx: ApiCtx,
	_query: ServerlessHealthCheckQuery,
	body: ServerlessHealthCheckRequest,
) -> Result<ServerlessHealthCheckResponse> {
	ctx.auth().await?;

	let ServerlessHealthCheckRequest { url, headers } = body;

	match fetch_serverless_runner_metadata(&ctx, url, headers).await {
		Ok(metadata) => Ok(ServerlessHealthCheckResponse::Success {
			version: metadata.version,
		}),
		Err(error) => Ok(ServerlessHealthCheckResponse::Failure { error }),
	}
}
