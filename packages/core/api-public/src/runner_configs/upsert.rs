use std::collections::HashMap;

use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use rivet_api_peer::runner_configs::*;
use rivet_api_util::request_remote_datacenter;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::utils;
use crate::ctx::ApiCtx;

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerConfigsUpsertRequestBody)]
pub struct UpsertRequest {
	pub datacenters: HashMap<String, rivet_api_types::namespaces::runner_configs::RunnerConfig>,
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
#[tracing::instrument(skip_all)]
pub async fn upsert(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<UpsertPath>,
	Query(query): Query<UpsertQuery>,
	Json(body): Json<UpsertRequest>,
) -> Response {
	match upsert_inner(ctx, path, query, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn upsert_inner(
	ctx: ApiCtx,
	path: UpsertPath,
	query: UpsertQuery,
	mut body: UpsertRequest,
) -> Result<UpsertResponse> {
	ctx.auth().await?;

	tracing::debug!(runner_name = ?path.runner_name, datacenters_count = body.datacenters.len(), "starting upsert");

	// Store serverless config before processing (since we'll remove from body.datacenters)
	let serverless_config = body
		.datacenters
		.iter()
		.filter_map(|(dc_name, runner_config)| {
			if let rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Serverless {
				url,
				headers,
				..
			} = &runner_config.kind
			{
				Some((url.clone(), headers.clone().unwrap_or_default()))
			} else {
				None
			}
		})
		.next();

	// Apply config
	for dc in &ctx.config().topology().datacenters {
		if let Some(runner_config) = body.datacenters.remove(&dc.name) {
			if ctx.config().dc_label() == dc.datacenter_label {
				rivet_api_peer::runner_configs::upsert(
					ctx.clone().into(),
					path.clone(),
					query.clone(),
					rivet_api_peer::runner_configs::UpsertRequest(runner_config),
				)
				.await?;
			} else {
				request_remote_datacenter::<UpsertResponse>(
					ctx.config(),
					dc.datacenter_label,
					&format!("/runner-configs/{}", path.runner_name),
					axum::http::Method::PUT,
					Some(&query),
					Some(&runner_config),
				)
				.await?;
			}
		} else {
			if ctx.config().dc_label() == dc.datacenter_label {
				rivet_api_peer::runner_configs::delete(
					ctx.clone().into(),
					DeletePath {
						runner_name: path.runner_name.clone(),
					},
					DeleteQuery {
						namespace: query.namespace.clone(),
					},
				)
				.await?;
			} else {
				request_remote_datacenter::<DeleteResponse>(
					ctx.config(),
					dc.datacenter_label,
					&format!("/runner-configs/{}", path.runner_name),
					axum::http::Method::DELETE,
					Some(&query),
					Option::<&()>::None,
				)
				.await?;
			}
		}
	}

	// Update runner metadata
	//
	// This allows us to populate the actor names immediately upon configuring a serverless runner
	if let Some((url, metadata_headers)) = serverless_config {
		// Resolve namespace
		let namespace = ctx
			.op(namespace::ops::resolve_for_name_global::Input {
				name: query.namespace.clone(),
			})
			.await?
			.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

		if let Err(err) = utils::refresh_runner_config_metadata(
			ctx.clone(),
			namespace.namespace_id,
			path.runner_name.clone(),
			url,
			metadata_headers,
		)
		.await
		{
			tracing::warn!(?err, runner_name = ?path.runner_name, "failed to refresh runner config metadata");
		}
	}

	Ok(UpsertResponse {})
}
