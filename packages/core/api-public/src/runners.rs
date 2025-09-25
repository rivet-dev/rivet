use anyhow::Result;
use axum::{
	extract::{Extension, Query},
	http::HeaderMap,
	response::{IntoResponse, Json, Response},
};
use rivet_api_builder::ApiError;
use rivet_api_types::{pagination::Pagination, runners::list::*};
use rivet_api_util::fanout_to_datacenters;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::ctx::ApiCtx;

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
	ctx.auth().await?;

	// Fanout to all datacenters
	let mut runners =
		fanout_to_datacenters::<ListResponse, _, _, _, _, Vec<rivet_types::runners::Runner>>(
			ctx.into(),
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
	ctx.auth().await?;

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
		ctx.into(),
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
