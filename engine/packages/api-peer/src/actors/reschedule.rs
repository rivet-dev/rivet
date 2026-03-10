use anyhow::Result;
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use rivet_api_types::actors::reschedule::*;
use rivet_util::Id;

#[utoipa::path(
	post,
	operation_id = "actors_reschedule",
	path = "/actors/{actor_id}/reschedule",
	params(
		("actor_id" = Id, Path),
		RescheduleQuery,
	),
	responses(
		(status = 200, body = RescheduleResponse),
	),
)]
#[tracing::instrument(skip_all)]
pub async fn reschedule(
	ctx: ApiCtx,
	path: ReschedulePath,
	query: RescheduleQuery,
	_body: (),
) -> Result<RescheduleResponse> {
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

	let namespace = ctx
		.op(namespace::ops::resolve_for_name_global::Input {
			name: query.namespace,
		})
		.await?
		.ok_or_else(|| namespace::errors::Namespace::NotFound.build())?;

	if actor.namespace_id != namespace.namespace_id || actor.destroy_ts.is_some() {
		return Err(pegboard::errors::Actor::NotFound.build());
	}

	let res = ctx
		.signal(pegboard::workflows::actor::Reschedule {
			reset_rescheduling: true,
		})
		.to_workflow::<pegboard::workflows::actor::Workflow>()
		.tag("actor_id", path.actor_id)
		.graceful_not_found()
		.send()
		.await?;
	if res.is_none() {
		return Err(pegboard::errors::Actor::NotFound.build());
	}

	Ok(RescheduleResponse {})
}
