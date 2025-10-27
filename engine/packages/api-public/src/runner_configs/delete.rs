use anyhow::Result;
use axum::response::{IntoResponse, Response};
use futures_util::{StreamExt, TryStreamExt};
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
#[tracing::instrument(skip_all)]
pub async fn delete(
	Extension(ctx): Extension<ApiCtx>,
	Path(path): Path<DeletePath>,
	Query(query): Query<DeleteQuery>,
) -> Response {
	match delete_inner(ctx, path, query).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(skip_all)]
async fn delete_inner(ctx: ApiCtx, path: DeletePath, query: DeleteQuery) -> Result<DeleteResponse> {
	ctx.auth().await?;

	let dcs = ctx.config().topology().datacenters.clone();
	futures_util::stream::iter(dcs)
		.map(|dc| {
			let ctx = ctx.clone();
			let query = query.clone();
			let path = path.clone();
			async move {
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
						Some(&query),
						Option::<&()>::None,
					)
					.await?;
				}

				anyhow::Ok(())
			}
		})
		.buffer_unordered(16)
		.try_collect::<Vec<_>>()
		// NOTE: We must error when any peer request fails, not all
		.await?;

	// Resolve namespace
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	// Purge cache
	ctx.cache()
		.clone()
		.request()
		.purge(
			"namespace.runner_config.get",
			vec![(namespace.namespace_id, path.runner_name.clone())],
		)
		.await?;

	Ok(DeleteResponse {})
}
