use api_helper::ctx::Ctx;
use rivet_api::models;
use rivet_convert::ApiTryFrom;
use rivet_operation::prelude::*;

use crate::auth::Auth;

/// Validates that a server belongs to the given game ID.
pub async fn server_for_env(
	ctx: &Ctx<Auth>,
	server_id: Uuid,
	game_id: Uuid,
	env_id: Uuid,
) -> GlobalResult<()> {
	let get_res = op!([ctx] ds_server_get {
		server_ids: vec![server_id.into()],
	})
	.await?;
	let server = unwrap_with!(get_res.servers.first(), SERVERS_SERVER_NOT_FOUND);

	// Validate token can access server
	ensure_with!(
		unwrap!(server.env_id).as_uuid() == env_id,
		SERVERS_SERVER_NOT_FOUND
	);

	Ok(())
}
