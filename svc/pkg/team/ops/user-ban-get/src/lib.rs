use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct BannedUser {
	team_id: Uuid,
	user_id: Uuid,
	ban_ts: i64,
}

#[operation(name = "team-user-ban-get")]
async fn handle(
	ctx: OperationContext<team::user_ban_get::Request>,
) -> GlobalResult<team::user_ban_get::Response> {
	// Map pairs
	let queries = ctx
		.members
		.iter()
		.map(|member| -> GlobalResult<(Uuid, Uuid)> {
			Ok((
				internal_unwrap!(member.team_id).as_uuid(),
				internal_unwrap!(member.user_id).as_uuid(),
			))
		})
		.collect::<GlobalResult<Vec<(Uuid, Uuid)>>>()?;

	let banned_users = sqlx::query_as::<_, BannedUser>(&formatdoc!(
		"
		SELECT team_id, user_id, ban_ts
		FROM banned_users
		INNER JOIN jsonb_array_elements($1::JSONB) AS q
		ON team_id = (q->>0)::UUID AND user_id = (q->>1)::UUID
		"
	))
	.bind(serde_json::to_string(&queries)?)
	.fetch_all(&ctx.crdb("db-team").await?)
	.await?;

	Ok(team::user_ban_get::Response {
		banned_users: banned_users
			.into_iter()
			.map(|banned_member| team::user_ban_get::response::BannedUser {
				team_id: Some(banned_member.team_id.into()),
				user_id: Some(banned_member.user_id.into()),
				ban_ts: banned_member.ban_ts,
			})
			.collect::<Vec<_>>(),
	})
}
