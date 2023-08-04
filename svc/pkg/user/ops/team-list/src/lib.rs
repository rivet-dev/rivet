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

	let team_members = sqlx::query_as::<_, (Uuid, Uuid)>(indoc!(
		"
		SELECT user_id, team_id
		FROM team_members
		WHERE user_id = ANY($1)
		"
	))
	.bind(&user_ids)
	.fetch_all(&ctx.crdb("db-team").await?)
	.await?;

	let users = user_ids
		.iter()
		.map(|user_id| user::team_list::response::UserTeams {
			user_id: Some((*user_id).into()),
			teams: team_members
				.iter()
				.filter(|(team_user_id, _)| team_user_id == user_id)
				.map(|(_, team_id)| user::team_list::response::TeamMember {
					team_id: Some((*team_id).into()),
				})
				.collect(),
		})
		.collect();

	Ok(user::team_list::Response { users })
}
