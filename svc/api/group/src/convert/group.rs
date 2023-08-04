use proto::backend::{self, pkg::*};
use rivet_convert::{ApiInto, ApiTryInto};
use rivet_group_server::models;
use rivet_operation::prelude::*;

pub fn handle(team: &backend::team::Team, is_developer: bool) -> GlobalResult<models::GroupHandle> {
	let team_id = internal_unwrap!(team.team_id).as_uuid();

	Ok(models::GroupHandle {
		group_id: team_id.to_string(),
		display_name: team.display_name.to_owned(),
		avatar_url: util::route::team_avatar(
			team.profile_upload_id.as_ref().map(common::Uuid::as_uuid),
			team.profile_file_name.as_ref(),
		),
		external: models::GroupExternalLinks {
			profile: util::route::team_profile(&team_id),
			chat: util::route::team_chat(&team_id),
		},
		is_developer: is_developer.then_some(true),
	})
}

pub fn summary(
	team: &backend::team::Team,
	team_member_counts: &[team::member_count::response::Team],
	dev_teams: &[backend::team::DevTeam],
	is_current_identity_member: bool,
) -> GlobalResult<models::GroupSummary> {
	let team_id_proto = internal_unwrap!(team.team_id);

	let member_count = internal_unwrap_owned!(team_member_counts
		.iter()
		.find(|t| t.team_id.as_ref() == Some(team_id_proto)))
	.member_count;
	let is_developer = dev_teams
		.iter()
		.any(|dev_team| dev_team.team_id == team.team_id);

	let team_id = team_id_proto.as_uuid();
	let owner_user_id = internal_unwrap!(team.owner_user_id).as_uuid();

	Ok(models::GroupSummary {
		group_id: team_id.to_string(),
		display_name: team.display_name.clone(),
		bio: team.bio.clone(),
		avatar_url: util::route::team_avatar(
			team.profile_upload_id.as_ref().map(common::Uuid::as_uuid),
			team.profile_file_name.as_ref(),
		),
		external: models::GroupExternalLinks {
			profile: util::route::team_profile(&team_id),
			chat: util::route::team_chat(&team_id),
		},

		is_current_identity_member,
		publicity: internal_unwrap_owned!(backend::team::Publicity::from_i32(team.publicity))
			.api_into(),
		member_count: member_count.try_into()?,
		owner_identity_id: owner_user_id.to_string(),
		is_developer,
	})
}
