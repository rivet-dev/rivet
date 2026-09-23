use anyhow::Result;
use axum::{
	extract::Extension,
	response::{IntoResponse, Json, Response},
};
use rivet_api_builder::ApiError;
use rivet_api_types::{datacenters::list::*, pagination::Pagination};
use rivet_auth::{AccessNamespaceScope, OperationKind, ResourceKind, TargetScope};
use rivet_types::datacenters::Datacenter;

use crate::ctx::ApiCtx;

#[utoipa::path(
	get,
	operation_id = "datacenters_list",
	path = "/datacenters",
	responses(
		(status = 200, body = ListResponse),
	),
	security(("bearer_auth" = [])),
)]
#[tracing::instrument(skip_all)]
pub async fn list(Extension(ctx): Extension<ApiCtx>) -> Response {
	match list_inner(ctx).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

#[tracing::instrument(level = "debug", skip_all)]
async fn list_inner(ctx: ApiCtx) -> Result<ListResponse> {
	ctx.auth(
		AccessNamespaceScope::Any,
		ResourceKind::Datacenter,
		TargetScope::Any,
		OperationKind::List,
	)
	.await?;

	Ok(ListResponse {
		datacenters: ctx
			.config()
			.topology()
			.datacenters
			.iter()
			.map(|dc| Datacenter {
				label: dc.datacenter_label,
				name: dc.name.clone(),
				url: dc.public_url.to_string(),
			})
			.collect(),
		pagination: Pagination { cursor: None },
	})
}
