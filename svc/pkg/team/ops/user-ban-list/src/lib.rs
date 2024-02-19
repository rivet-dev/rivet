use proto::backend::pkg::*;
use rivet_operation::prelude::*;

// TEMP:
const MAX_BANS: i64 = 256;

#[derive(sqlx::FromRow)]
struct BannedUser {
	team_id: Uuid,
	user_id: Uuid,
	ban_ts: i64,
}

#[operation(name = "team-user-ban-list")]
async fn handle(
	ctx: OperationContext<team::user_ban_list::Request>,
) -> GlobalResult<team::user_ban_list::Response> {
	let team_ids = ctx
		.team_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Fetch all users
	let banned_users: Vec<BannedUser> = sql_fetch_all!(
		[ctx, BannedUser]
		"
		SELECT team_id, user_id, ban_ts
		FROM db_team.banned_users
		WHERE team_id = ANY($1)
		LIMIT $2
		",
		&team_ids,
		MAX_BANS,
	)
	.await?;

	// Group in to teams
	let teams = team_ids
		.iter()
		.map(|team_id| team::user_ban_list::response::Team {
			team_id: Some((*team_id).into()),
			banned_users: banned_users
				.iter()
				.filter(|banned_member| banned_member.team_id == *team_id)
				.map(|banned_member| team::user_ban_list::response::BannedUser {
					user_id: Some(banned_member.user_id.into()),
					ban_ts: banned_member.ban_ts,
				})
				.collect::<Vec<_>>(),
		})
		.collect::<Vec<_>>();

	Ok(team::user_ban_list::Response { teams })
}
