use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "team-dev-game-list")]
async fn handle(
	ctx: OperationContext<team_dev::game_list::Request>,
) -> GlobalResult<team_dev::game_list::Response> {
	let team_ids = ctx
		.team_ids
		.iter()
		.map(|id| id.as_uuid())
		.collect::<Vec<_>>();

	let games = sqlx::query_as::<_, (Uuid, Uuid)>(indoc!(
		"
		SELECT g.game_id, g.developer_team_id
		FROM unnest($1::UUID[]) AS q
		INNER JOIN db_game.games AS g
		ON g.developer_team_id = q
	"
	))
	.bind(&team_ids)
	.fetch_all(&ctx.crdb().await?)
	.await?;

	let teams = team_ids
		.iter()
		.map(|team_id| team_dev::game_list::response::TeamGames {
			team_id: Some((*team_id).into()),
			game_ids: games
				.iter()
				.filter(|(_, dev_team_id)| dev_team_id == team_id)
				.map(|(game_id, _)| (*game_id).into())
				.collect(),
		})
		.collect();

	Ok(team_dev::game_list::Response { teams })
}
