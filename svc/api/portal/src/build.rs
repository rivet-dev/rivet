use api_helper::ctx::Ctx;
use proto::{backend, common};
use rivet_convert::{ApiInto, ApiTryInto};
use rivet_operation::prelude::*;
use rivet_portal_server::models;

use crate::auth::Auth;

pub async fn group_summaries(
	ctx: &Ctx<Auth>,
	current_user_id: common::Uuid,
	group_ids: &[common::Uuid],
) -> GlobalResult<Vec<models::GroupSummary>> {
	// Fetch team metadata
	let (user_team_list_res, teams_res, team_member_count_res, team_dev_res) = tokio::try_join!(
		op!([ctx] user_team_list {
			user_ids: vec![current_user_id],
		}),
		op!([ctx] team_get {
			team_ids: group_ids.to_vec(),
		}),
		op!([ctx] team_member_count {
			team_ids: group_ids.to_vec(),
		}),
		op!([ctx] team_dev_get {
			team_ids: group_ids.to_vec(),
		}),
	)?;

	// Build group handles
	let groups = group_ids
		.iter()
		.map(|group_id| {
			let team_data = unwrap!(teams_res
				.teams
				.iter()
				.find(|t| t.team_id.as_ref() == Some(group_id)));
			let is_current_user_member = unwrap!(user_team_list_res.users.first())
				.teams
				.iter()
				.any(|team| team.team_id.as_ref() == Some(group_id));
			let member_count = unwrap!(team_member_count_res
				.teams
				.iter()
				.find(|t| t.team_id.as_ref() == Some(group_id)))
			.member_count;
			let is_developer = team_dev_res
				.teams
				.iter()
				.any(|dev_team| dev_team.team_id == team_data.team_id);

			let team_id = group_id.as_uuid();
			let owner_user_id = unwrap_ref!(team_data.owner_user_id).as_uuid();
			Ok(models::GroupSummary {
				group_id: team_id.to_string(),
				display_name: team_data.display_name.clone(),
				bio: team_data.bio.clone(),
				avatar_url: util::route::team_avatar(&team_data),
				external: models::GroupExternalLinks {
					profile: util::route::team_profile(team_id),
					chat: util::route::team_chat(team_id),
				},

				is_current_identity_member: is_current_user_member,
				publicity: unwrap!(backend::team::Publicity::from_i32(team_data.publicity))
					.api_into(),
				member_count: member_count.try_into()?,
				owner_identity_id: owner_user_id.to_string(),
				is_developer,
			})
		})
		.collect::<GlobalResult<Vec<models::GroupSummary>>>()?;

	Ok(groups)
}
