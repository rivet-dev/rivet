use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Team {
	team_id: Uuid,
	create_ts: i64,
}

#[operation(name = "team-search")]
async fn handle(
	ctx: OperationContext<team::search::Request>,
) -> GlobalResult<team::search::Response> {
	let crdb = ctx.crdb().await?;
	let limit = ctx.limit;

	internal_assert!(limit != 0, "limit too low");
	internal_assert!(limit <= 32, "limit too high");

	let res = sqlx::query_as::<_, Team>(indoc!(
		"
		SELECT team_id, create_ts FROM db_team.teams@search_index
		WHERE
			display_name % $1 AND
			is_searchable = TRUE AND
			create_ts <= $2
			ORDER BY create_ts DESC
			LIMIT $3
		"
	))
	.bind(ctx.query.trim())
	.bind(ctx.anchor.unwrap_or_else(util::timestamp::now))
	.bind(limit as i64)
	.fetch_all(&crdb)
	.await?;

	let anchor = res.last().map(|team| team.create_ts);

	Ok(team::search::Response {
		team_ids: res
			.into_iter()
			.map(|team| team.team_id.into())
			.collect::<Vec<_>>(),
		anchor,
	})
}
