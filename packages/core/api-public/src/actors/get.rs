use anyhow::Result;
use axum::{
	extract::{Extension, Path, Query},
	http::HeaderMap,
	response::{IntoResponse, Json, Response},
};
use rivet_api_builder::{ApiCtx, ApiError};
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::actors::utils;

#[derive(Debug, Serialize, Deserialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct GetQuery {
	pub namespace: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetPath {
	pub actor_id: Id,
}

#[derive(Serialize, ToSchema)]
#[schema(as = ActorsGetResponse)]
pub struct GetResponse {
	pub actor: rivet_types::actors::Actor,
}

/// ## Datacenter Round Trips
///
/// 2 round trip:
/// - GET /actors/{}
/// - [api-peer] namespace::ops::resolve_for_name_global
#[utoipa::path(
	get,
	operation_id = "actors_get",
	path = "/actors/{actor_id}",
	params(
		("actor_id" = Id, Path),
		GetQuery,
	),
	responses(
		(status = 200, body = GetResponse),
	),
)]
pub async fn get(
	Extension(ctx): Extension<ApiCtx>,
	headers: HeaderMap,
	Path(path): Path<GetPath>,
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
	path: GetPath,
	query: GetQuery,
) -> Result<Response> {
	let actor = utils::fetch_actor_by_id(&ctx, headers, path.actor_id, query.namespace).await?;
	Ok(Json(GetResponse { actor }).into_response())
}
