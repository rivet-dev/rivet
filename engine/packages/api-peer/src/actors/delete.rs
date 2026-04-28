use anyhow::Result;
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use rivet_api_types::actors::delete::*;
use rivet_util::Id;

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
#[tracing::instrument(skip_all)]
pub async fn delete(ctx: ApiCtx, path: DeletePath, query: DeleteQuery) -> Result<DeleteResponse> {
	// Subscribe before fetching actor data
	let (mut destroy_sub, mut destroy_sub2) = tokio::try_join!(
		ctx.subscribe::<pegboard::workflows::actor::DestroyComplete>(("actor_id", path.actor_id)),
		ctx.subscribe::<pegboard::workflows::actor2::DestroyComplete>(("actor_id", path.actor_id)),
	)?;

	let (actors_res, namespace_res) = tokio::try_join!(
		// Get the actor to verify it exists
		ctx.op(pegboard::ops::actor::get::Input {
			actor_ids: vec![path.actor_id],
			fetch_error: false,
		}),
		ctx.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace,
		}),
	)?;

	let namespace = namespace_res.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	let actor = actors_res
		.actors
		.into_iter()
		.next()
		.ok_or_else(|| pegboard::errors::Actor::NotFound.build())?;

	// Already destroyed: succeed idempotently
	if actor.destroy_ts.is_some() {
		return Ok(DeleteResponse {});
	}

	// Verify the actor belongs to the specified namespace
	if actor.namespace_id != namespace.namespace_id {
		return Err(pegboard::errors::Actor::NotFound.build());
	}

	// Try actor2 first, then fallback to actor
	let res = ctx
		.signal(pegboard::workflows::actor2::Destroy {})
		.to_workflow::<pegboard::workflows::actor2::Workflow>()
		.tag("actor_id", path.actor_id)
		.graceful_not_found()
		.send()
		.await?;
	if res.is_none() {
		let res = ctx
			.signal(pegboard::workflows::actor::Destroy {})
			.to_workflow::<pegboard::workflows::actor::Workflow>()
			.tag("actor_id", path.actor_id)
			.graceful_not_found()
			.send()
			.await?;

		if res.is_none() {
			tracing::warn!(
				actor_id=?path.actor_id,
				"actor workflow not found, likely already stopped"
			);
		} else {
			destroy_sub.next().await?;
		}
	} else {
		destroy_sub2.next().await?;
	}

	Ok(DeleteResponse {})
}
