use api_helper::ctx::Ctx;
use pegboard::types::EndpointType;
use rivet_operation::prelude::*;

use crate::auth::Auth;

/// Validates that an actor belongs to the given game ID.
pub async fn actor_for_env(
	ctx: &Ctx<Auth>,
	actor_id: Uuid,
	_game_id: Uuid,
	env_id: Uuid,
	endpoint_type: Option<EndpointType>,
) -> GlobalResult<pegboard::types::Actor> {
	let actors_res = ctx
		.op(pegboard::ops::actor::get::Input {
			actor_ids: vec![actor_id],
			endpoint_type,
		})
		.await?;
	let actor = unwrap_with!(actors_res.actors.into_iter().next(), ACTOR_NOT_FOUND);

	// Validate token can access actor
	ensure_with!(actor.env_id == env_id, ACTOR_NOT_FOUND);

	Ok(actor)
}
