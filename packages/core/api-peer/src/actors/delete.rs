use anyhow::Result;
use rivet_api_builder::ApiCtx;
use rivet_util::Id;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Serialize, IntoParams)]
#[serde(deny_unknown_fields)]
#[into_params(parameter_in = Query)]
pub struct DeleteQuery {
	pub namespace: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[schema(as = ActorsDeleteResponse)]
pub struct DeleteResponse {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeletePath {
	pub actor_id: Id,
}

#[utoipa::path(
    delete,
	operation_id = "runners_delete",
    path = "/actors/{actor_id}",
    params(
        ("actor_id" = Id, Path),
        DeleteQuery,
    ),
    responses(
        (status = 200, body = DeleteResponse),
    ),
)]
pub async fn delete(ctx: ApiCtx, path: DeletePath, query: DeleteQuery) -> Result<DeleteResponse> {
	// Get the actor first to verify it exists
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

	ctx.signal(pegboard::workflows::actor::Destroy {})
		.to_workflow::<pegboard::workflows::actor::Workflow>()
		.tag("actor_id", path.actor_id)
		.send()
		.await?;

	Ok(DeleteResponse {})
}
