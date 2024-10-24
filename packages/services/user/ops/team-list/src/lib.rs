use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "user-team-list")]
async fn handle(
	ctx: OperationContext<user::team_list::Request>,
) -> GlobalResult<user::team_list::Response> {
	let user_ids = ctx
		.user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let users = ctx
		.cache()
		.fetch_all_proto("user_team_list", user_ids, {
			let ctx = ctx.clone();
			move |mut cache, user_ids| {
				let ctx = ctx.clone();
				async move {
					let team_members = sql_fetch_all!(
						[ctx, (Uuid, Uuid)]
						"
					SELECT user_id, team_id
					FROM db_team.team_members
					WHERE user_id = ANY($1)
					",
						&user_ids,
					)
					.await?;

					for user_id in user_ids {
						// Aggregate user teams
						let user_teams = user::team_list::response::UserTeams {
							user_id: Some(user_id.into()),
							teams: team_members
								.iter()
								.filter(|(team_user_id, _)| *team_user_id == user_id)
								.map(|(_, team_id)| user::team_list::response::TeamMember {
									team_id: Some((*team_id).into()),
								})
								.collect(),
						};

						cache.resolve(&user_id.clone(), user_teams);
					}

					Ok(cache)
				}
			}
		})
		.await?;

	Ok(user::team_list::Response { users })
}
