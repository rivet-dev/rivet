use rivet_api::models;
use rivet_operation::prelude::*;

use crate::convert;

pub async fn summaries(
	ctx: &OperationContext<()>,
	current_user_id: Uuid,
	group_ids: Vec<Uuid>,
) -> GlobalResult<Vec<models::GroupSummary>> {
	if group_ids.is_empty() {
		return Ok(Vec::new());
	}

	let group_ids_proto = group_ids
		.clone()
		.into_iter()
		.map(Into::into)
		.collect::<Vec<_>>();

	// Fetch team metadata
	let (user_team_list_res, teams_res, team_member_count_res, team_dev_res) = tokio::try_join!(
		op!([ctx] user_team_list {
			user_ids: vec![current_user_id.into()],
		}),
		op!([ctx] team_get {
			team_ids: group_ids_proto.clone(),
		}),
		op!([ctx] team_member_count {
			team_ids: group_ids_proto.clone(),
		}),
		op!([ctx] team_dev_get {
			team_ids: group_ids_proto,
		}),
	)?;

	let user_teams = &internal_unwrap_owned!(user_team_list_res.users.first()).teams;

	teams_res
		.teams
		.iter()
		.map(|team| {
			let is_current_identity_member = user_teams.iter().any(|t| t.team_id == team.team_id);

			convert::group::summary(
				team,
				&team_member_count_res.teams,
				&team_dev_res.teams,
				is_current_identity_member,
			)
		})
		.collect::<GlobalResult<Vec<_>>>()
}
