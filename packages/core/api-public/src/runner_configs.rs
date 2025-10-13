use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use axum::{
	http::HeaderMap,
	response::{IntoResponse, Response},
};
use reqwest::header::{HeaderMap as ReqwestHeaderMap, HeaderName, HeaderValue};
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
	pub runner_configs: HashMap<String, RunnerConfigDatacenters>,
	pub pagination: Pagination,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[schema(as = RunnerConfigsListResponseRunnerConfigsValue)]
pub struct RunnerConfigDatacenters {
	pub datacenters: HashMap<String, rivet_types::runner_configs::RunnerConfig>,
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
		HashMap<String, RunnerConfigDatacenters>,
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

	Ok(DeleteResponse {})
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerConfigsServerlessHealthCheckRequest)]
pub struct ServerlessHealthCheckRequest {
	pub url: String,
	#[serde(default)]
	pub headers: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, ToSchema, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[schema(as = RunnerConfigsServerlessHealthCheckError)]
pub enum ServerlessHealthCheckError {
	InvalidRequest {},
	RequestFailed {},
	RequestTimedOut {},
	NonSuccessStatus { status_code: u16, body: String },
	InvalidResponseJson { body: String },
	InvalidResponseSchema { runtime: String, version: String },
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = RunnerConfigsServerlessHealthCheckResponse)]
pub enum ServerlessHealthCheckResponse {
	Success { version: String },
	Failure { error: ServerlessHealthCheckError },
}

impl ServerlessHealthCheckResponse {
	fn success(version: String) -> Self {
		Self::Success { version }
	}

	fn failure(error: ServerlessHealthCheckError) -> Self {
		Self::Failure { error }
	}
}

const RESPONSE_BODY_MAX_LEN: usize = 1024;

fn truncate_response_body(body: &str) -> String {
	let mut chars = body.chars();
	let mut truncated: String = chars.by_ref().take(RESPONSE_BODY_MAX_LEN).collect();
	if chars.next().is_some() {
		truncated.push_str("...[truncated]");
	}

	truncated
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ServerlessMetadataPayload {
	runtime: String,
	version: String,
}

#[utoipa::path(
	post,
	operation_id = "runner_configs_serverless_health_check",
	path = "/runner-configs/serverless-health-check",
	request_body(content = ServerlessHealthCheckRequest, content_type = "application/json"),
	responses(
		(status = 200, body = ServerlessHealthCheckResponse),
	),
	security(("bearer_auth" = [])),
)]
pub async fn serverless_health_check(
	Extension(ctx): Extension<ApiCtx>,
	Json(body): Json<ServerlessHealthCheckRequest>,
) -> Response {
	match serverless_health_check_inner(ctx, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn serverless_health_check_inner(
	ctx: ApiCtx,
	body: ServerlessHealthCheckRequest,
) -> Result<ServerlessHealthCheckResponse> {
	ctx.auth().await?;

	let ServerlessHealthCheckRequest { url, headers } = body;
	let trimmed_url = url.trim();
	if trimmed_url.is_empty() {
		return Ok(ServerlessHealthCheckResponse::failure(
			ServerlessHealthCheckError::InvalidRequest {},
		));
	}

	let health_url = format!("{}/metadata", trimmed_url.trim_end_matches('/'));

	if reqwest::Url::parse(&health_url).is_err() {
		return Ok(ServerlessHealthCheckResponse::failure(
			ServerlessHealthCheckError::InvalidRequest {},
		));
	}

	let mut header_map = ReqwestHeaderMap::new();
	for (name, value) in headers {
		let header_name = match HeaderName::from_bytes(name.trim().as_bytes()) {
			Ok(name) => name,
			Err(_) => {
				return Ok(ServerlessHealthCheckResponse::failure(
					ServerlessHealthCheckError::InvalidRequest {},
				));
			}
		};

		let header_value = match HeaderValue::from_str(value.trim()) {
			Ok(value) => value,
			Err(_) => {
				return Ok(ServerlessHealthCheckResponse::failure(
					ServerlessHealthCheckError::InvalidRequest {},
				));
			}
		};

		header_map.insert(header_name, header_value);
	}

	let client = match reqwest::Client::builder()
		.timeout(Duration::from_secs(10))
		.build()
	{
		Ok(client) => client,
		Err(_) => {
			return Ok(ServerlessHealthCheckResponse::failure(
				ServerlessHealthCheckError::RequestFailed {},
			));
		}
	};

	let response = match client.get(&health_url).headers(header_map).send().await {
		Ok(response) => response,
		Err(err) => {
			let error = if err.is_timeout() {
				ServerlessHealthCheckError::RequestTimedOut {}
			} else {
				ServerlessHealthCheckError::RequestFailed {}
			};

			return Ok(ServerlessHealthCheckResponse::failure(error));
		}
	};

	let status = response.status();
	let body_raw = response
		.text()
		.await
		.unwrap_or_else(|_| String::from("<failed to read body>"));
	let body_for_user = truncate_response_body(&body_raw);

	if !status.is_success() {
		return Ok(ServerlessHealthCheckResponse::failure(
			ServerlessHealthCheckError::NonSuccessStatus {
				status_code: status.as_u16(),
				body: body_for_user,
			},
		));
	}

	let payload = match serde_json::from_str::<ServerlessMetadataPayload>(&body_raw) {
		Ok(payload) => payload,
		Err(_) => {
			return Ok(ServerlessHealthCheckResponse::failure(
				ServerlessHealthCheckError::InvalidResponseJson {
					body: body_for_user,
				},
			));
		}
	};

	let ServerlessMetadataPayload { runtime, version } = payload;

	let trimmed_version = version.trim();
	if runtime != "rivetkit" || trimmed_version.is_empty() {
		return Ok(ServerlessHealthCheckResponse::failure(
			ServerlessHealthCheckError::InvalidResponseSchema { runtime, version },
		));
	}

	Ok(ServerlessHealthCheckResponse::success(
		trimmed_version.to_owned(),
	))
}
