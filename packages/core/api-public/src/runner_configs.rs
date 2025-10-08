use std::collections::HashMap;

use anyhow::Result;
use axum::{
	http::HeaderMap,
	response::{IntoResponse, Response},
};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Path, Query},
};
use rivet_api_peer::runner_configs::*;
use rivet_api_types::{pagination::Pagination, runner_configs::list::*};
use rivet_api_util::{fanout_to_datacenters, request_remote_datacenter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ctx::ApiCtx;

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerConfigsListResponse)]
pub struct ListResponse {
	pub runner_configs: HashMap<String, HashMap<String, rivet_types::runner_configs::RunnerConfig>>,
	pub pagination: Pagination,
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

	let runner_configs = fanout_to_datacenters::<
		rivet_api_types::runner_configs::list::ListResponse,
		_,
		_,
		_,
		_,
		HashMap<String, HashMap<String, rivet_types::runner_configs::RunnerConfig>>,
	>(
		ctx.clone().into(),
		headers,
		"/runner-configs",
		query.clone(),
		move |ctx, query| {
			let path = path.clone();
			async move { rivet_api_peer::runner_configs::list(ctx, path, query).await }
		},
		|dc_label, res, agg| {
			for (runner_name, runner_config) in res.runner_configs {
				let entry = agg.entry(runner_name).or_insert_with(HashMap::new);

				entry.insert(
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

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerConfigsUpsertRequest)]
pub struct UpsertRequest(
	#[schema(inline)] HashMap<String, rivet_types::runner_configs::RunnerConfig>,
);

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
	mut body: UpsertRequest,
) -> Result<UpsertResponse> {
	ctx.auth().await?;

	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	// Upsert default to epoxy
	if let Some(default_config) = body.0.remove("default") {
		ctx.op(namespace::ops::runner_config::upsert_default::Input {
			namespace_id: namespace.namespace_id,
			name: path.runner_name.clone(),
			config: default_config,
		})
		.await?;
	}

	for dc in &ctx.config().topology().datacenters {
		if let Some(runner_config) = body.0.remove(&dc.name) {
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
					headers.clone(),
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
					headers.clone(),
					Some(&query),
					Option::<&()>::None,
				)
				.await?;
			}
		}
	}

	Ok(UpsertResponse {})
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

	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	for dc in &ctx.config().topology().datacenters {
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
				headers.clone(),
				Some(&query),
				Option::<&()>::None,
			)
			.await?;
		}
	}

	// Delete default from epoxy
	ctx.op(namespace::ops::runner_config::delete_default::Input {
		namespace_id: namespace.namespace_id,
		name: path.runner_name.clone(),
	})
	.await?;

	Ok(DeleteResponse {})
}
