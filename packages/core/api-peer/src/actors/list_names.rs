use anyhow::Result;
use rivet_api_builder::ApiCtx;
use rivet_api_types::{actors::list_names::*, pagination::Pagination};
use rivet_types::actors::ActorName;

#[utoipa::path(
    get,
	operation_id = "actors_list_names",
    path = "/actors/names",
    params(ListNamesQuery),
    responses(
        (status = 200, body = ListNamesResponse),
    ),
)]
pub async fn list_names(
	ctx: ApiCtx,
	_path: (),
	query: ListNamesQuery,
) -> Result<ListNamesResponse> {
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace.clone(),
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	let list_names_res = ctx
		.op(pegboard::ops::actor::list_names::Input {
			namespace_id: namespace.namespace_id,
			after_name: query.cursor.clone(),
			limit: query.limit.unwrap_or(100),
		})
		.await?;

	let cursor = list_names_res
		.names
		.last()
		.map(|(name, _)| name.to_string());

	Ok(ListNamesResponse {
		names: list_names_res
			.names
			.into_iter()
			.map(|(name, data)| {
				(
					name,
					ActorName {
						metadata: data.metadata,
					},
				)
			})
			.collect(),
		pagination: Pagination { cursor },
	})
}
