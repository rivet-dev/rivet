use anyhow::Result;
use rivet_api_builder::ApiCtx;
use rivet_api_types::{envoys::list::*, pagination::Pagination};

#[utoipa::path(
	get,
	operation_id = "envoys_list",
	path = "/envoys",
	params(ListQuery),
	responses(
		(status = 200, body = ListResponse),
	),
)]
#[tracing::instrument(skip_all)]
pub async fn list(ctx: ApiCtx, _path: (), query: ListQuery) -> Result<ListResponse> {
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	if !query.envoy_key.is_empty() {
		let envoys = ctx
			.op(pegboard::ops::envoy::get::Input {
				namespace_id: namespace.namespace_id,
				envoy_keys: query.envoy_key.clone(),
			})
			.await?
			.envoys;

		Ok(ListResponse {
			envoys,
			pagination: Pagination { cursor: None },
		})
	} else {
		let list_res = ctx
			.op(pegboard::ops::envoy::list::Input {
				namespace_id: namespace.namespace_id,
				pool_name: query.name,
				created_before: query
					.cursor
					.as_deref()
					.map(|c| c.parse::<i64>())
					.transpose()?,
				limit: query.limit.unwrap_or(100),
			})
			.await?;

		let cursor = list_res.envoys.last().map(|x| x.create_ts.to_string());

		Ok(ListResponse {
			envoys: list_res.envoys,
			pagination: Pagination { cursor },
		})
	}
}
