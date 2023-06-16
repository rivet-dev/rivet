use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "team-recommend")]
async fn handle(
	ctx: OperationContext<team::recommend::Request>,
) -> Result<team::recommend::Response, GlobalError> {
	let count = ctx.count;

	// TODO: Find teams with the most friends in it, then order by member count
	let team_ids = sqlx::query_as::<_, (Uuid,)>(indoc!(
		"
		SELECT team_id
		FROM teams
		ORDER BY create_ts DESC
		LIMIT $1
		"
	))
	.bind(count as i64)
	.fetch_all(&ctx.crdb("db-team").await?)
	.await?
	.into_iter()
	.map(|row| row.0.into())
	.collect::<Vec<common::Uuid>>();

	Ok(team::recommend::Response { team_ids })
}
