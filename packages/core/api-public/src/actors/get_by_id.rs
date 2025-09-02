use anyhow::Result;
use axum::{
	extract::{Extension, Query},
	response::{IntoResponse, Json, Response},
};
use rivet_api_builder::{ApiCtx, ApiError};
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct GetByIdQuery {
	pub namespace: String,
	pub name: String,
	pub key: String,
}

#[derive(Serialize, ToSchema)]
#[schema(as = ActorsGetByIdResponse)]
pub struct GetByIdResponse {
	pub actor_id: Option<Id>,
}

/// ## Datacenter Round Trips
///
/// 1 round trip:
/// - namespace::ops::resolve_for_name_global
///
/// This does not require another round trip since we use stale consistency for the get_id_for_key.
#[utoipa::path(
    get,
	operation_id = "actors_get_by_id",
    path = "/actors/by-id",
    params(GetByIdQuery),
    responses(
        (status = 200, body = GetByIdResponse),
    ),
)]
pub async fn get_by_id(
	Extension(ctx): Extension<ApiCtx>,
	Query(query): Query<GetByIdQuery>,
) -> Response {
	match get_by_id_inner(ctx, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn get_by_id_inner(ctx: ApiCtx, query: GetByIdQuery) -> Result<GetByIdResponse> {
	// Resolve namespace
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	// Get the actor for the key
	// This operation uses global consistency and handles datacenter routing
	let actor = ctx
		.op(pegboard::ops::actor::get_for_key::Input {
			namespace_id: namespace.namespace_id,
			name: query.name,
			key: query.key,
		})
		.await?
		.actor;

	Ok(GetByIdResponse {
		actor_id: actor.map(|x| x.actor_id),
	})
}
