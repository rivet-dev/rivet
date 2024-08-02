use std::convert::TryInto;

use rivet_api::models;
use rivet_operation::prelude::*;
use types_proto::rivet::backend::{self, pkg::*};

use crate::ApiInto;

pub fn handle(team: &backend::team::Team) -> GlobalResult<models::GroupHandle> {
	let team_id = unwrap_ref!(team.team_id).as_uuid();

	Ok(models::GroupHandle {
		group_id: team_id,
		display_name: team.display_name.to_owned(),
		avatar_url: util::route::team_avatar(&team),
		external: Box::new(models::GroupExternalLinks {
			profile: util::route::team_profile(team_id),
		}),
		is_developer: Some(true),
	})
}

pub fn summary(
	team: &backend::team::Team,
	team_member_counts: &[team::member_count::response::Team],
	is_current_identity_member: bool,
) -> GlobalResult<models::GroupSummary> {
	let team_id_proto = unwrap_ref!(team.team_id);

	let member_count = unwrap!(team_member_counts
		.iter()
		.find(|t| t.team_id.as_ref() == Some(team_id_proto)))
	.member_count;

	let team_id = team_id_proto.as_uuid();
	let owner_user_id = unwrap_ref!(team.owner_user_id).as_uuid();

	Ok(models::GroupSummary {
		group_id: team_id,
		display_name: team.display_name.clone(),
		bio: team.bio.clone(),
		avatar_url: util::route::team_avatar(&team),
		external: Box::new(models::GroupExternalLinks {
			profile: util::route::team_profile(team_id),
		}),

		is_current_identity_member,
		publicity: unwrap!(backend::team::Publicity::from_i32(team.publicity)).api_into(),
		member_count: member_count.try_into()?,
		owner_identity_id: owner_user_id,
		is_developer: true,
	})
}
