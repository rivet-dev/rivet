use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /auth/inspect
pub async fn inspect(
	ctx: Ctx<Auth>,
	_watch_index_query: WatchIndexQuery,
) -> GlobalResult<models::CloudInspectResponse> {
	let claims = ctx.auth().claims()?;

	let agent = if let Ok(user_ent) = claims.as_user() {
		models::CloudAuthAgent {
			identity: Some(Box::new(models::CloudAuthAgentIdentity {
				identity_id: user_ent.user_id,
			})),
			..Default::default()
		}
	} else if let Ok(cloud_ent) = claims.as_game_cloud() {
		models::CloudAuthAgent {
			game_cloud: Some(Box::new(models::CloudAuthAgentGameCloud {
				game_id: cloud_ent.game_id,
			})),
			..Default::default()
		}
	} else {
		bail_with!(CLAIMS_MISSING_ENTITLEMENT, entitlements = "User, GameCloud");
	};

	Ok(models::CloudInspectResponse {
		agent: Box::new(agent),
	})
}
