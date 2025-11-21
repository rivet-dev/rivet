use anyhow::{Result, bail};
use rivet_api_builder::ApiCtx;
use rivet_api_types::{actors::list::*, pagination::Pagination};

#[utoipa::path(
	get,
	operation_id = "actors_list",
	path = "/actors",
	params(ListQuery),
	responses(
		(status = 200, body = ListResponse),
	),
)]
#[tracing::instrument(skip_all)]
pub async fn list(ctx: ApiCtx, _path: (), query: ListQuery) -> Result<ListResponse> {
	let key = query.key;
	let actor_ids = [
		query.actor_id,
		query
			.actor_ids
			.map(|x| {
				x.split(',')
					.filter_map(|s| s.trim().parse::<rivet_util::Id>().ok())
					.collect::<Vec<_>>()
			})
			.unwrap_or_default(),
	]
	.concat();
	let include_destroyed = query.include_destroyed.unwrap_or(false);

	// TODO: Update api-peer to require including the reservation ID in the query if querying with
	// key in order to assert the request was sent to the correct datacenter

	// If actor_ids are provided, fetch actors directly by ID
	if !actor_ids.is_empty() {
		// Resolve namespace to verify actors belong to it
		let namespace = ctx
			.op(namespace::ops::resolve_for_name_global::Input {
				name: query.namespace.clone(),
			})
			.await?
			.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

		// Fetch actors by their IDs
		let get_res = ctx
			.op(pegboard::ops::actor::get::Input { actor_ids })
			.await?;

		// Filter actors by namespace
		let mut actors: Vec<rivet_types::actors::Actor> = get_res
			.actors
			.into_iter()
			.filter(|actor| actor.namespace_id == namespace.namespace_id)
			.collect();

		// Sort by create ts desc
		actors.sort_by_cached_key(|x| std::cmp::Reverse(x.create_ts));

		// Apply limit
		actors.truncate(query.limit.unwrap_or(100));

		let cursor = actors.last().map(|x| x.create_ts.to_string());

		Ok(ListResponse {
			actors,
			pagination: Pagination { cursor },
		})
	} else {
		// Original list logic for name/key
		if query.name.is_none() {
			bail!("name is required when not using actor_ids")
		}

		let namespace = ctx
			.op(namespace::ops::resolve_for_name_global::Input {
				name: query.namespace.clone(),
			})
			.await?
			.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

		let list_res = ctx
			.op(pegboard::ops::actor::list_for_ns::Input {
				namespace_id: namespace.namespace_id,
				name: query.name.unwrap(),
				key,
				include_destroyed,
				created_before: query
					.cursor
					.as_deref()
					.map(|c| c.parse::<i64>())
					.transpose()?,
				limit: query.limit.unwrap_or(100),
			})
			.await?;

		let cursor = list_res.actors.last().map(|x| x.create_ts.to_string());

		Ok(ListResponse {
			actors: list_res.actors,
			pagination: Pagination { cursor },
		})
	}
}
