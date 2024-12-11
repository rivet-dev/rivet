use api_helper::ctx::Ctx;
use ds::types::EndpointType;
use rivet_operation::prelude::*;

use crate::auth::Auth;

/// Validates that a server belongs to the given game ID.
pub async fn server_for_env(
	ctx: &Ctx<Auth>,
	server_id: Uuid,
	_game_id: Uuid,
	env_id: Uuid,
	endpoint_type: Option<EndpointType>,
) -> GlobalResult<ds::types::Server> {
	let servers_res = ctx
		.op(ds::ops::server::get::Input {
			server_ids: vec![server_id],
			endpoint_type,
		})
		.await?;
	let server = unwrap_with!(servers_res.servers.into_iter().next(), ACTOR_NOT_FOUND);

	// Validate token can access server
	ensure_with!(server.env_id == env_id, ACTOR_NOT_FOUND);

	Ok(server)
}
