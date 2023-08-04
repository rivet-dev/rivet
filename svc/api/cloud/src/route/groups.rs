use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend;
use rivet_cloud_server::models;
use rivet_convert::ApiInto;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};

use crate::auth::Auth;

// MARK: POST /groups/{}/convert
pub async fn convert(
	_ctx: Ctx<Auth>,
	_group_id: Uuid,
	_body: models::ConvertGroupRequest,
) -> GlobalResult<models::ConvertGroupResponse> {
	// Disabled until we go public
	panic_with!(API_FORBIDDEN, reason = "Closed beta");

	// ctx.auth().check_team_owner(ctx.op_ctx(), group_id).await?;

	// msg!([ctx] team_dev::msg::create(group_id) -> team::msg::update {
	// 	team_id: Some(group_id.into()),
	// })
	// .await?;

	// Ok(models::ConvertGroupResponse {})
}

// MARK: POST /groups/validate
pub async fn validate(
	ctx: Ctx<Auth>,
	body: models::ValidateGroupRequest,
) -> GlobalResult<models::ValidateGroupResponse> {
	let res = op!([ctx] team_validate {
		display_name: body.display_name,
	})
	.await?;

	Ok(models::ValidateGroupResponse {
		errors: res
			.errors
			.into_iter()
			.map(ApiInto::api_into)
			.collect::<Vec<_>>(),
	})
}
