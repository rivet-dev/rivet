use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend::{self, pkg::*};
use rivet_api::models;
use rivet_convert::ApiInto;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::Auth;

// MARK: POST /groups/{}/convert
pub async fn convert(
	ctx: Ctx<Auth>,
	group_id: Uuid,
	_body: serde_json::Value,
) -> GlobalResult<serde_json::Value> {
	let publicity = unwrap!(std::env::var("RIVET_ACCESS_KIND").ok());
	match publicity.as_str() {
		"public" => {
			ctx.auth().check_team_owner(ctx.op_ctx(), group_id).await?;
		}
		"private" => {
			ctx.auth().admin(ctx.op_ctx()).await?;
		}
		_ => bail!("invalid RIVET_ACCESS_KIND"),
	}

	msg!([ctx] team_dev::msg::create(group_id) -> team::msg::update {
		team_id: Some(group_id.into()),
	})
	.await?;

	Ok(json!({}))
}

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
