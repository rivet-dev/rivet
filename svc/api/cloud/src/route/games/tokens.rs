use api_helper::ctx::Ctx;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: POST /games/{}/tokens/cloud
pub async fn create_cloud_token(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	_body: serde_json::Value,
) -> GlobalResult<models::CloudGamesCreateCloudTokenResponse> {
	ctx.auth()
		.check_game_write_or_admin(ctx.op_ctx(), game_id)
		.await?;

	let token_res = op!([ctx] cloud_game_token_create {
		game_id: Some(game_id.into()),
	})
	.await?;

	Ok(models::CloudGamesCreateCloudTokenResponse {
		token: token_res.token,
	})
}
