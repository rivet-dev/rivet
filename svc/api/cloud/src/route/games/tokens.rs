use api_helper::ctx::Ctx;
use rivet_cloud_server::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: POST /games/{}/tokens/cloud
pub async fn create_cloud_token(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	_body: models::CreateCloudTokenRequest,
) -> GlobalResult<models::CreateCloudTokenResponse> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;

	let token_res = op!([ctx] cloud_game_token_create {
		game_id: Some(game_id.into()),
	})
	.await?;

	Ok(models::CreateCloudTokenResponse {
		token: token_res.token,
	})
}
