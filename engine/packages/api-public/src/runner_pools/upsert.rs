use std::collections::HashMap;

use rivet_auth::*;
use anyhow::Result;
use axum::response::{IntoResponse, Response};
use futures_util::{StreamExt, TryStreamExt};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use rivet_api_peer::envoy_pools::*;
use rivet_api_util::request_remote_datacenter;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ctx::ApiCtx;

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerPoolsUpsertRequestBody)]
pub struct UpsertRequest {
	pub datacenters: HashMap<String, rivet_api_types::namespaces::runner_pools::RunnerPool>,
}

#[utoipa::path(
	put,
	operation_id = "runner_pools_upsert",
	path = "/runner-pools/{pool_name}",
	params(
		("pool_name" = String, Path),
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

#[tracing::instrument(level = "debug", skip_all)]
async fn upsert_inner(
	ctx: ApiCtx,
	path: UpsertPath,
	query: UpsertQuery,
	mut body: UpsertRequest,
) -> Result<UpsertResponse> {
	ctx.auth(
		AccessNamespaceScope::Name(query.namespace.clone()),
		ResourceKind::RunnerPool,
		TargetScope::Any,
		OperationKind::Create,
	)
	.await?;

	let dcs = ctx
		.config()
		.topology()
		.datacenters
		.iter()
		.map(|dc| (dc.clone(), body.datacenters.remove(&dc.name)))
		.collect::<Vec<_>>();

	// Check for leftover datacenters in the body, this means those datacenters don't exist
	if !body.datacenters.is_empty() {
		return Err(rivet_api_util::errors::Datacenter::NotFound.build());
	}

	let any_endpoint_config_changed = futures_util::stream::iter(dcs)
		.map(|(dc, runner_pool)| {
			let ctx = ctx.clone();
			let query = query.clone();
			let path = path.clone();
			async move {
				if let Some(runner_pool) = runner_pool {
					let response = if ctx.config().dc_label() == dc.datacenter_label {
						rivet_api_peer::envoy_pools::upsert(
							ctx.clone().into(),
							path.clone(),
							query.clone(),
							rivet_api_peer::envoy_pools::UpsertRequest(runner_pool),
						)
						.await?
					} else {
						request_remote_datacenter::<UpsertResponse>(
							ctx.config(),
							dc.datacenter_label,
							&format!("/runner-pools/{}", path.pool_name),
							axum::http::Method::PUT,
							Some(&query),
							Some(&runner_pool),
						)
						.await?
					};

					anyhow::Ok(response.endpoint_config_changed)
				} else {
					if ctx.config().dc_label() == dc.datacenter_label {
						rivet_api_peer::envoy_pools::delete(
							ctx.clone().into(),
							DeletePath {
								pool_name: path.pool_name.clone(),
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
							&format!("/runner-pools/{}", path.pool_name),
							axum::http::Method::DELETE,
							Some(&query),
							Option::<&()>::None,
						)
						.await?;
					}

					Ok(false)
				}
			}
		})
		.buffer_unordered(16)
		.try_collect::<Vec<_>>()
		// NOTE: We must error when any peer request fails, not all
		.await?
		.into_iter()
		.any(|endpoint_config_changed| endpoint_config_changed);

	// Resolve namespace
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	// Fetch enabled dcs to prewarm epoxy cache
	ctx.op(pegboard::ops::runner::list_envoy_pool_enabled_dcs::Input {
		namespace_id: namespace.namespace_id,
		pool_name: path.pool_name.clone(),
	})
	.await?;

	Ok(UpsertResponse {
		endpoint_config_changed: any_endpoint_config_changed,
	})
}
