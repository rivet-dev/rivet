use std::collections::HashMap;

use rivet_auth::*;
use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use rivet_api_types::{
	pagination::Pagination,
	runner_pools::{RunnerPoolResponse, list::*},
};
use rivet_api_util::fanout_to_datacenters;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ctx::ApiCtx;

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerPoolsListResponse)]
pub struct ListResponse {
	pub runner_pools: HashMap<String, RunnerPoolDatacenters>,
	pub pagination: Pagination,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[schema(as = RunnerPoolsListResponseRunnerPoolsValue)]
pub struct RunnerPoolDatacenters {
	pub datacenters: HashMap<String, RunnerPoolResponse>,
}

#[utoipa::path(
	get,
	operation_id = "runner_pools_list",
	path = "/runner-pools",
	params(
		ListQuery,
	),
	responses(
		(status = 200, body = ListResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn list(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<ListPath>,
	Query(query): Query<ListQuery>,
) -> Response {
	match list_inner(ctx, path, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(level = "debug", skip_all)]
async fn list_inner(ctx: ApiCtx, path: ListPath, query: ListQuery) -> Result<ListResponse> {
	ctx.auth(
		AccessNamespaceScope::Name(query.namespace.clone()),
		ResourceKind::RunnerPool,
		TargetScope::Any,
		if query.pool_names.is_none() && query.pool_name.is_empty() {
			OperationKind::List
		} else {
			OperationKind::Read
		},
	)
	.await?;

	let runner_pools = fanout_to_datacenters::<
		rivet_api_types::runner_pools::list::ListResponse,
		_,
		_,
		_,
		_,
		HashMap<String, RunnerPoolDatacenters>,
	>(
		&ctx,
		"/runner-pools",
		query.clone(),
		move |ctx, query| {
			let path = path.clone();
			async move { rivet_api_peer::envoy_pools::list(ctx, path, query).await }
		},
		|dc_label, res, agg| {
			for (pool_name, runner_pool) in res.runner_pools {
				let entry = agg
					.entry(pool_name)
					.or_insert_with(|| RunnerPoolDatacenters {
						datacenters: HashMap::new(),
					});

				entry.datacenters.insert(
					ctx.config()
						.dc_for_label(dc_label)
						.expect("dc should exist")
						.name
						.clone(),
					runner_pool,
				);
			}
		},
	)
	.await?;

	Ok(ListResponse {
		runner_pools,
		pagination: Pagination { cursor: None },
	})
}
