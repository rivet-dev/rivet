use api_helper::ctx::Ctx;
use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use rivet_party_server::models;

use crate::{auth::Auth, utils};

pub mod matchmaker;

// MARK: DELETE /parties/self/activity
pub async fn set_idle(ctx: Ctx<Auth>) -> GlobalResult<models::SetPartyToIdleResponse> {
	let (user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	let party_id = unwrap_with_owned!(
		utils::get_current_party(ctx.op_ctx(), user_id).await?,
		PARTY_IDENTITY_NOT_IN_ANY_PARTY
	);

	utils::assert_party_leader(ctx.op_ctx(), party_id, user_id).await?;

	msg!([ctx] party::msg::state_set_idle(party_id) {
		party_id: Some(party_id.into()),
	})
	.await?;

	Ok(models::SetPartyToIdleResponse {})
}

// MARK: DELETE /parties/self/members/self/inactive
pub async fn set_self_inactive(ctx: Ctx<Auth>) -> GlobalResult<models::SetPartyToIdleResponse> {
	let (user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	let party_id = unwrap_with_owned!(
		utils::get_current_party(ctx.op_ctx(), user_id).await?,
		PARTY_IDENTITY_NOT_IN_ANY_PARTY
	);

	msg!([ctx] party::msg::member_state_set_inactive(party_id, user_id) {
		party_id: Some(party_id.into()),
		user_id: Some(user_id.into()),
	})
	.await?;

	Ok(models::SetPartyToIdleResponse {})
}
