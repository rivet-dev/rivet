use api_helper::ctx::Ctx;

use rivet_api::models;
use rivet_convert::ApiInto;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: POST /groups/validate
pub async fn validate(
	ctx: Ctx<Auth>,
	body: models::CloudValidateGroupRequest,
) -> GlobalResult<models::CloudValidateGroupResponse> {
	let publicity = unwrap!(std::env::var("RIVET_ACCESS_KIND").ok());
	match publicity.as_str() {
		"public" => {}
		"private" => {
			ctx.auth().admin(ctx.op_ctx()).await?;
		}
		_ => bail!("invalid RIVET_ACCESS_KIND"),
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
