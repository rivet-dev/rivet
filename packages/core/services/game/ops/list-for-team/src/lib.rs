use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-list-for-team")]
pub async fn handle(
	ctx: OperationContext<game::list_for_team::Request>,
) -> GlobalResult<game::list_for_team::Response> {
	let team_ids = ctx
		.team_ids
		.iter()
		.map(|id| id.as_uuid())
		.collect::<Vec<_>>();

	let games = sql_fetch_all!(
		[ctx, (Uuid, Uuid)]
		"
		SELECT g.game_id, g.developer_team_id
		FROM unnest($1::UUID[]) AS q
		INNER JOIN db_game.games AS g
		ON g.developer_team_id = q
	",
		&team_ids,
	)
	.await?;

	let teams = team_ids
		.iter()
		.map(|team_id| game::list_for_team::response::TeamGames {
			team_id: Some((*team_id).into()),
			game_ids: games
				.iter()
				.filter(|(_, dev_team_id)| dev_team_id == team_id)
				.map(|(game_id, _)| (*game_id).into())
				.collect(),
		})
		.collect();

	Ok(game::list_for_team::Response { teams })
}
