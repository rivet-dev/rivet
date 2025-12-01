use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Query},
};
use rivet_api_types::{actors::list_names::*, pagination::Pagination};
use rivet_api_util::fanout_to_datacenters;
use rivet_types::actors::ActorName;

use crate::ctx::ApiCtx;

/// ## Datacenter Round Trips
///
/// 2 round trips:
/// - GET /actors/names (fanout)
/// - [api-peer] namespace::ops::resolve_for_name_global
#[utoipa::path(
	get,
	operation_id = "actors_list_names",
	path = "/actors/names",
	params(ListNamesQuery),
	responses(
		(status = 200, body = ListNamesResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn list_names(
	Extension(ctx): Extension<ApiCtx>,
	Query(query): Query<ListNamesQuery>,
) -> Response {
	match list_names_inner(ctx, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn list_names_inner(ctx: ApiCtx, query: ListNamesQuery) -> Result<ListNamesResponse> {
	ctx.auth().await?;

	// Prepare peer query for local handler
	let peer_query = ListNamesQuery {
		namespace: query.namespace.clone(),
		limit: query.limit,
		cursor: query.cursor.clone(),
	};

	// Fanout to all datacenters
	let mut all_names =
		fanout_to_datacenters::<ListNamesResponse, _, _, _, _, Vec<(String, ActorName)>>(
			ctx.into(),
			"/actors/names",
			peer_query,
			|ctx, query| async move {
				rivet_api_peer::actors::list_names::list_names(ctx, (), query).await
			},
			|_, res, agg| agg.extend(res.names),
		)
		.await?;

	// Sort by name for consistency
	all_names.sort_by(|a, b| a.0.cmp(&b.0));

	// Truncate to the requested limit
	all_names.truncate(query.limit.unwrap_or(100));

	let cursor = all_names.last().map(|(name, _)| name.to_string());

	Ok(ListNamesResponse {
		// TODO: Implement ComposeSchema for FakeMap so we don't have to reallocate
		names: all_names.into_iter().collect(),
		pagination: Pagination { cursor },
	})
}
