use rivet_auth::*;
use anyhow::Result;
use axum::response::{IntoResponse, Response};
use rivet_api_builder::{
	ApiError,
	extract::{Extension, Json, Query},
};
use rivet_api_types::{
	pagination::Pagination,
	runners::{Runner, list::*},
};
use rivet_api_util::fanout_to_datacenters;

use crate::ctx::ApiCtx;

#[utoipa::path(
	get,
	operation_id = "runners_list",
	path = "/runners",
	params(ListQuery),
	responses(
		(status = 200, body = ListResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn list(Extension(ctx): Extension<ApiCtx>, Query(query): Query<ListQuery>) -> Response {
	match list_inner(ctx, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn list_inner(ctx: ApiCtx, query: ListQuery) -> Result<ListResponse> {
	ctx.auth(
		AccessNamespaceScope::Name(query.namespace.clone()),
		ResourceKind::Runner,
		TargetScope::Any,
		if query.runner_key.is_empty() {
			OperationKind::List
		} else {
			OperationKind::Read
		},
	)
	.await?;

	// Fanout to all datacenters
	let mut envoys = fanout_to_datacenters::<
		rivet_api_types::envoys::list::ListResponse,
		_,
		_,
		_,
		_,
		Vec<rivet_types::envoys::Envoy>,
	>(
		&ctx,
		"/envoys",
		rivet_api_types::envoys::list::ListQuery {
			namespace: query.namespace.clone(),
			name: query.name.clone(),
			envoy_key: query.runner_key.clone(),
			limit: query.limit,
			cursor: query.cursor.clone(),
		},
		|ctx, query| async move { rivet_api_peer::envoys::list(ctx, (), query).await },
		|_, res, agg| agg.extend(res.envoys),
	)
	.await?;

	// Sort by create ts desc
	envoys.sort_by_cached_key(|x| std::cmp::Reverse(x.create_ts));

	// Shorten array since returning all envoys from all regions could end up returning `regions *
	// limit` results, which is a lot.
	envoys.truncate(query.limit.unwrap_or(100));

	let cursor = envoys.last().map(|x| x.create_ts.to_string());

	Ok(ListResponse {
		runners: envoys.into_iter().map(Runner::from).collect(),
		pagination: Pagination { cursor },
	})
}
