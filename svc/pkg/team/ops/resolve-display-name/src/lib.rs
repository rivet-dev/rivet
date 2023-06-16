use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "team-resolve-display-name")]
async fn handle(
	ctx: OperationContext<team::resolve_display_name::Request>,
) -> GlobalResult<team::resolve_display_name::Response> {
	let teams = sqlx::query_as::<_, (String, Uuid)>(indoc!(
		"
		SELECT display_name, team_id
		FROM teams
		WHERE display_name = ANY($1)
	"
	))
	.bind(&ctx.display_names)
	.fetch_all(&ctx.crdb("db-team").await?)
	.await?
	.into_iter()
	.map(
		|(display_name, team_id)| team::resolve_display_name::response::Team {
			display_name,
			team_id: Some(team_id.into()),
		},
	)
	.collect::<Vec<_>>();

	Ok(team::resolve_display_name::Response { teams })
}
