use api_helper::ctx::Ctx;
use rivet_operation::prelude::*;

use crate::auth::Auth;

/// Validates that a given user ID is the member of a group.
pub async fn group_member(ctx: &Ctx<Auth>, group_id: Uuid, user_id: Uuid) -> GlobalResult<bool> {
	// Fetch team members
	let members_res = op!([ctx] team_member_list {
		team_ids: vec![group_id.into()],
		limit: None,
		anchor: None,
	})
	.await?;

	// Check if member
	let team = unwrap!(members_res.teams.first()).clone();
	let is_member = team
		.members
		.iter()
		.map(|m| Ok(unwrap_ref!(m.user_id).as_uuid()))
		.collect::<GlobalResult<Vec<_>>>()?
		.contains(&user_id);

	Ok(is_member)
}
