use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "team-resolve-display-name")]
async fn handle(
	ctx: OperationContext<team::resolve_display_name::Request>,
) -> GlobalResult<team::resolve_display_name::Response> {
	let teams = sql_fetch_all!(
		[ctx, (String, Uuid)]
		"
		SELECT display_name, team_id
		FROM db_team.teams
		WHERE display_name = ANY($1)
	",
		&ctx.display_names,
	)
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
