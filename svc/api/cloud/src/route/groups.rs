use api_helper::ctx::Ctx;
use rivet_api::models;
use rivet_config::config::rivet::RivetAccessKind;
use rivet_convert::ApiInto;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: POST /groups/validate
pub async fn validate(
	ctx: Ctx<Auth>,
	body: models::CloudValidateGroupRequest,
) -> GlobalResult<models::CloudValidateGroupResponse> {
	if ctx.config().server()?.rivet.auth.access_kind != RivetAccessKind::Public {
		ctx.auth().admin(ctx.op_ctx()).await?;
	}

	let res = op!([ctx] team_validate {
		display_name: body.display_name,
	})
	.await?;

	Ok(models::CloudValidateGroupResponse {
		errors: res
			.errors
			.into_iter()
			.map(ApiInto::api_into)
			.collect::<Vec<_>>(),
	})
}
