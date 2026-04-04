use anyhow::Result;
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use rivet_api_types::actors::sleep::*;

#[tracing::instrument(skip_all)]
pub async fn sleep(
	ctx: ApiCtx,
	path: SleepPath,
	query: SleepQuery,
	_body: SleepRequest,
) -> Result<SleepResponse> {
	// Get the actor first to verify it exists
	let actors_res = ctx
		.op(pegboard::ops::actor::get::Input {
			actor_ids: vec![path.actor_id],
			fetch_error: false,
		})
		.await?;

	let actor = actors_res
		.actors
		.into_iter()
		.next()
		.ok_or_else(|| pegboard::errors::Actor::NotFound.build())?;

	// Verify the actor belongs to the specified namespace
	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace,
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	if actor.namespace_id != namespace.namespace_id {
		return Err(pegboard::errors::Actor::NotFound.build());
	}

	let res = ctx
		.signal(pegboard::workflows::actor2::Sleep {})
		.to_workflow::<pegboard::workflows::actor2::Workflow>()
		.tag("actor_id", path.actor_id)
		.graceful_not_found()
		.send()
		.await?;

	if res.is_none() {
		tracing::warn!(
			actor_id=?path.actor_id,
			"actor workflow not found, likely already stopped"
		);
	}

	Ok(SleepResponse {})
}
