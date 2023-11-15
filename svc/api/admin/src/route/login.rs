use api_helper::ctx::Ctx;
use proto::backend::pkg::*;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: POST /login
pub async fn login(
	ctx: Ctx<Auth>,
	body: models::AdminLoginRequest,
) -> GlobalResult<models::AdminLoginResponse> {
	// TODO: Generate access token token

	Ok(models::AdminLoginResponse { url: todo!() })
}
