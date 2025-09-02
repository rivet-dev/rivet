use anyhow::Result;
use axum::{
	extract::{Extension, Path, Query},
	http::HeaderMap,
	response::{IntoResponse, Json, Response},
};
use rivet_api_builder::{ApiCtx, ApiError};
use rivet_api_client::{fanout_to_datacenters, request_remote_datacenter_raw};
use rivet_api_types::{
	pagination::Pagination,
	runners::{get::*, list::*},
};
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[utoipa::path(
    get,
	operation_id = "runners_get",
    path = "/runners/{runner_id}",
    params(
        ("runner_id" = Id, Path),
        GetQuery,
    ),
    responses(
        (status = 200, body = GetResponse),
    ),
)]
pub async fn get(
	Extension(ctx): Extension<ApiCtx>,
	headers: HeaderMap,
	Path(path): Path<rivet_api_peer::runners::GetPath>,
	Query(query): Query<GetQuery>,
) -> Response {
	match get_inner(ctx, headers, path, query).await {
		Ok(response) => response,
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn get_inner(
	ctx: ApiCtx,
	headers: HeaderMap,
	path: rivet_api_peer::runners::GetPath,
	query: GetQuery,
) -> Result<Response> {
	if path.runner_id.label() == ctx.config().dc_label() {
		let res = rivet_api_peer::runners::get(ctx, path, query).await?;
		Ok(Json(res).into_response())
	} else {
		request_remote_datacenter_raw(
			&ctx,
			path.runner_id.label(),
			&format!("/runners/{}", path.runner_id),
			axum::http::Method::GET,
			headers,
			Some(&query),
			Option::<&()>::None,
		)
		.await
	}
}

#[utoipa::path(
    get,
	operation_id = "runners_list",
    path = "/runners",
    params(ListQuery),
    responses(
        (status = 200, body = ListResponse),
    ),
)]
pub async fn list(
	Extension(ctx): Extension<ApiCtx>,
	headers: HeaderMap,
	Query(query): Query<ListQuery>,
) -> Response {
	match list_inner(ctx, headers, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn list_inner(ctx: ApiCtx, headers: HeaderMap, query: ListQuery) -> Result<ListResponse> {
	// Fanout to all datacenters
	let mut runners =
		fanout_to_datacenters::<ListResponse, _, _, _, _, Vec<rivet_types::runners::Runner>>(
			ctx,
			headers,
			"/runners",
			query.clone(),
			|ctx, query| async move { rivet_api_peer::runners::list(ctx, (), query).await },
			|res, agg| agg.extend(res.runners),
		)
		.await?;

	// Sort by create ts desc
	runners.sort_by_cached_key(|x| std::cmp::Reverse(x.create_ts));

	// Shorten array since returning all runners from all regions could end up returning `regions *
	// limit` results, which is a lot.
	runners.truncate(query.limit.unwrap_or(100));

	let cursor = runners.last().map(|x| x.create_ts.to_string());

	Ok(ListResponse {
		runners,
		pagination: Pagination { cursor },
	})
}

#[derive(Debug, Deserialize, Serialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct ListNamesQuery {
	pub namespace: String,
	pub limit: Option<usize>,
	pub cursor: Option<String>,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnersListNamesResponse)]
pub struct ListNamesResponse {
	pub names: Vec<String>,
	pub pagination: Pagination,
}

/// ## Datacenter Round Trips
///
/// 2 round trips:
/// - GET /runners/names (fanout)
/// - [api-peer] namespace::ops::resolve_for_name_global
#[utoipa::path(
		get,
		operation_id = "runners_list_names",
		path = "/runners/names",
		params(ListNamesQuery),
		responses(
			(status = 200, body = ListNamesResponse),
		),
	)]
pub async fn list_names(
	Extension(ctx): Extension<ApiCtx>,
	headers: HeaderMap,
	Query(query): Query<ListNamesQuery>,
) -> Response {
	match list_names_inner(ctx, headers, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn list_names_inner(
	ctx: ApiCtx,
	headers: HeaderMap,
	query: ListNamesQuery,
) -> Result<ListNamesResponse> {
	// Prepare peer query for local handler
	let peer_query = rivet_api_peer::runners::ListNamesQuery {
		namespace: query.namespace.clone(),
		limit: query.limit,
		cursor: query.cursor.clone(),
	};

	// Fanout to all datacenters
	let mut all_names = fanout_to_datacenters::<
		rivet_api_peer::runners::ListNamesResponse,
		_,
		_,
		_,
		_,
		Vec<String>,
	>(
		ctx,
		headers,
		"/runners/names",
		peer_query,
		|ctx, query| async move { rivet_api_peer::runners::list_names(ctx, (), query).await },
		|res, agg| agg.extend(res.names),
	)
	.await?;

	// Sort by name for consistency
	all_names.sort();

	// Truncate to the requested limit
	all_names.truncate(query.limit.unwrap_or(100));

	let cursor = all_names.last().map(|x: &String| x.to_string());

	Ok(ListNamesResponse {
		names: all_names,
		pagination: Pagination { cursor },
	})
}
