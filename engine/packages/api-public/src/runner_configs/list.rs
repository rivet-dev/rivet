use std::collections::HashMap;

use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use rivet_api_types::{
	pagination::Pagination,
	runner_configs::{RunnerConfigResponse, list::*},
};
use rivet_api_util::fanout_to_datacenters;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ctx::ApiCtx;

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerConfigsListResponse)]
pub struct ListResponse {
	pub runner_configs: HashMap<String, RunnerConfigDatacenters>,
	pub pagination: Pagination,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[schema(as = RunnerConfigsListResponseRunnerConfigsValue)]
pub struct RunnerConfigDatacenters {
	pub datacenters: HashMap<String, RunnerConfigResponse>,
}

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

#[tracing::instrument(skip_all)]
async fn list_inner(ctx: ApiCtx, path: ListPath, query: ListQuery) -> Result<ListResponse> {
	ctx.auth().await?;

	let runner_configs = fanout_to_datacenters::<
		rivet_api_types::runner_configs::list::ListResponse,
		_,
		_,
		_,
		_,
		HashMap<String, RunnerConfigDatacenters>,
	>(
		ctx.clone().into(),
		"/runner-configs",
		query.clone(),
		move |ctx, query| {
			let path = path.clone();
			async move { rivet_api_peer::runner_configs::list(ctx, path, query).await }
		},
		|dc_label, res, agg| {
			for (runner_name, runner_config) in res.runner_configs {
				let entry = agg
					.entry(runner_name)
					.or_insert_with(|| RunnerConfigDatacenters {
						datacenters: HashMap::new(),
					});

				entry.datacenters.insert(
					ctx.config()
						.dc_for_label(dc_label)
						.expect("dc should exist")
						.name
						.clone(),
					runner_config,
				);
			}
		},
	)
	.await?;

	Ok(ListResponse {
		runner_configs,
		pagination: Pagination { cursor: None },
	})
}
