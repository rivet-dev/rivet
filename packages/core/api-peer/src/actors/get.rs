use anyhow::Result;
use rivet_api_builder::ApiCtx;
use rivet_api_types::actors::get::{GetQuery, GetResponse};
use rivet_util::Id;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct GetPath {
	pub actor_id: Id,
}

pub async fn get(ctx: ApiCtx, path: GetPath, query: GetQuery) -> Result<GetResponse> {
	let actors_res = ctx
		.op(pegboard::ops::actor::get::Input {
			actor_ids: vec![path.actor_id],
		})
		.await?;

	let actor = actors_res
		.actors
		.into_iter()
		.next()
		.ok_or_else(|| pegboard::errors::Actor::NotFound.build())?;

	// If namespace is provided, verify the actor belongs to it
	if let Some(namespace_name) = query.namespace {
		let namespace = ctx
			.op(namespace::ops::resolve_for_name_global::Input {
				name: namespace_name,
			})
			.await?
			.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

		if actor.namespace_id != namespace.namespace_id {
			return Err(pegboard::errors::Actor::NotFound.build());
		}
	}

	Ok(GetResponse { actor })
}
