use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "team-recommend")]
async fn handle(
	ctx: OperationContext<team::recommend::Request>,
) -> GlobalResult<team::recommend::Response> {
	let count = ctx.count;

	// TODO: Find teams with the most friends in it, then order by member count
	let team_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT team_id
		FROM db_team.teams
		ORDER BY create_ts DESC
		LIMIT $1
		",
		count as i64,
	)
	.await?
	.into_iter()
	.map(|row| row.0.into())
	.collect::<Vec<common::Uuid>>();

	Ok(team::recommend::Response { team_ids })
}
