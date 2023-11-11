use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_claims::ClaimsDecode;
use rivet_cloud_server::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /auth/inspect
pub async fn inspect(
	ctx: Ctx<Auth>,
	_watch_index_query: WatchIndexQuery,
) -> GlobalResult<models::InspectResponse> {
	let claims = ctx.auth().claims()?;

	let agent = if let Ok(user_ent) = claims.as_user() {
		models::AuthAgent::Identity(models::AuthAgentIdentity {
			identity_id: user_ent.user_id.to_string(),
		})
	} else if let Ok(cloud_ent) = claims.as_game_cloud() {
		models::AuthAgent::GameCloud(models::AuthAgentGameCloud {
			game_id: cloud_ent.game_id.to_string(),
		})
	} else {
		bail_with!(
			API_UNAUTHORIZED,
			reason = "Token is missing one of the following entitlements: user, game_cloud"
		);
	};

	Ok(models::InspectResponse { agent })
}
