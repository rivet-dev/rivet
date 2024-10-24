use api_helper::ctx::Ctx;
use rivet_operation::prelude::*;

use crate::auth::Auth;

/// Validates that a server belongs to the given game ID.
pub async fn server_for_env(
	ctx: &Ctx<Auth>,
	server_id: Uuid,
	game_id: Uuid,
	env_id: Uuid,
) -> GlobalResult<()> {
	let servers_res = ctx
		.op(ds::ops::server::get::Input {
			server_ids: vec![server_id],
		})
		.await?;
	let server = unwrap_with!(servers_res.servers.first(), SERVERS_SERVER_NOT_FOUND);

	// Validate token can access server
	ensure_with!(server.env_id == env_id, SERVERS_SERVER_NOT_FOUND);

	Ok(())
}
