use anyhow::Result;
use axum::response::{IntoResponse, Json, Response};
use rivet_api_builder::{ApiError, extract::Extension};
use rivet_api_types::{datacenters::list::*, pagination::Pagination};
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
pub async fn list(Extension(ctx): Extension<ApiCtx>) -> Response {
	match list_inner(ctx).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => ApiError::from(err).into_response(),
	}
}

async fn list_inner(ctx: ApiCtx) -> Result<ListResponse> {
	ctx.auth().await?;

	Ok(ListResponse {
		datacenters: ctx
			.config()
			.topology()
			.datacenters
			.iter()
			.map(|dc| Datacenter {
				datacenter_label: dc.datacenter_label,
				name: dc.name.clone(),
			})
			.collect(),
		pagination: Pagination { cursor: None },
	})
}
