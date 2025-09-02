use anyhow::Result;
use axum::{
	extract::{Extension, Path, Query},
	http::HeaderMap,
	response::{IntoResponse, Json, Response},
};
use rivet_api_builder::{ApiCtx, ApiError};
use rivet_api_client::request_remote_datacenter_raw;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct DeleteQuery {
	pub namespace: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeletePath {
	pub actor_id: Id,
}

#[derive(Serialize, ToSchema)]
#[schema(as = ActorsDeleteResponse)]
pub struct DeleteResponse {}

/// ## Datacenter Round Trips
///
/// 2 round trip:
/// - DELETE /actors/{}
/// - [api-peer] namespace::ops::resolve_for_name_global
#[utoipa::path(
    delete,
	operation_id = "actors_delete",
    path = "/actors/{actor_id}",
    params(
        ("actor_id" = Id, Path),
        DeleteQuery,
    ),
    responses(
        (status = 200, body = DeleteResponse),
    ),
)]
pub async fn delete(
	Extension(ctx): Extension<ApiCtx>,
	headers: HeaderMap,
	Path(path): Path<DeletePath>,
	Query(query): Query<DeleteQuery>,
) -> Response {
	match delete_inner(ctx, headers, path, query).await {
		Ok(response) => response,
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn delete_inner(
	ctx: ApiCtx,
	headers: HeaderMap,
	path: DeletePath,
	query: DeleteQuery,
) -> Result<Response> {
	if path.actor_id.label() == ctx.config().dc_label() {
		let peer_path = rivet_api_peer::actors::delete::DeletePath {
			actor_id: path.actor_id,
		};
		let peer_query = rivet_api_peer::actors::delete::DeleteQuery {
			namespace: query.namespace,
		};
		let res = rivet_api_peer::actors::delete::delete(ctx, peer_path, peer_query).await?;

		Ok(Json(res).into_response())
	} else {
		request_remote_datacenter_raw(
			&ctx,
			path.actor_id.label(),
			&format!("/actors/{}", path.actor_id),
			axum::http::Method::DELETE,
			headers,
			Some(&query),
			Option::<&()>::None,
		)
		.await
	}
}
