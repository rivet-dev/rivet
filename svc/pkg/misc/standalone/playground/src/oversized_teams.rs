use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use uuid::Uuid;

pub const MAX_TEAM_SIZE: i64 = 999999999;

#[tracing::instrument(skip_all)]
pub async fn run(pools: rivet_pools::Pools, ctx: OperationContext<()>) -> GlobalResult<()> {
	let crdb = pools.crdb("db-team")?;

	let teams = sqlx::query_as::<_, (Uuid, i64)>(
		"SELECT team_id, COUNT(*) FROM team_members GROUP BY team_id",
	)
	.fetch_all(&crdb)
	.await?;

	tracing::info!(?teams, "fetched team sizes");

	for (team_id, size) in teams {
		if size > MAX_TEAM_SIZE {
			let remove_count = size - MAX_TEAM_SIZE;
			tracing::info!(?remove_count, "removing members");

			let users = sqlx::query_as::<_, (Uuid,)>("SELECT user_id FROM team_members WHERE team_id = $1 ORDER BY join_ts DESC LIMIT $2")
				.bind(team_id)
				.bind(remove_count)
				.fetch_all(&crdb)
				.await?;

			for (user_id,) in users {
				tracing::info!(?team_id, ?user_id, "removing user");
				msg!([ctx] team::msg::member_remove(team_id, user_id) {
					team_id: Some(team_id.into()),
					user_id: Some(user_id.into()),
					silent: false,
				})
				.await?;
			}
		}
	}

	Ok(())
}
