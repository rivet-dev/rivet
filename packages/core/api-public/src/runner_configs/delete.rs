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
use rivet_api_util::request_remote_datacenter;

use crate::ctx::ApiCtx;

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
