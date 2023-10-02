use futures_util::{StreamExt, TryStreamExt};
use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "team-dev-halt")]
async fn handle(
	ctx: OperationContext<team_dev::halt::Request>,
) -> GlobalResult<team_dev::halt::Response> {
	let team_ids = ctx
		.team_ids
		.iter()
		.map(|id| id.as_uuid())
		.collect::<Vec<_>>();

	// Fetch games
	let game_ids = sqlx::query_as::<_, (Uuid,)>(indoc!(
		"
		SELECT game_id
		FROM db_game.games
		WHERE developer_team_id = ANY($1)
		"
	))
	.bind(team_ids)
	.fetch_all(&ctx.crdb().await?)
	.await?
	.into_iter()
	.map(|(game_id,)| Into::<common::Uuid>::into(game_id))
	.collect::<Vec<_>>();

	// Fetch namespaces
	let ns_list_res = op!([ctx] game_namespace_list {
		game_ids: game_ids,
	})
	.await?;
	let all_namespace_ids = ns_list_res
		.games
		.iter()
		.flat_map(|game| game.namespace_ids.iter().cloned())
		.collect::<Vec<_>>();

	// Fetch lobbies
	let lobby_list_res = op!([ctx] mm_lobby_list_for_namespace {
		namespace_ids: all_namespace_ids,
	})
	.await?;
	let all_lobby_ids = lobby_list_res
		.namespaces
		.iter()
		.flat_map(|namespace| namespace.lobby_ids.iter().cloned())
		.map(|id| id.as_uuid())
		.collect::<Vec<_>>();

	tracing::info!(count=?all_lobby_ids.len(), "stopping lobbies");

	futures_util::stream::iter(all_lobby_ids.into_iter().map(|lobby_id| {
		let ctx = ctx.base();

		async move {
			msg!([ctx] mm::msg::lobby_stop(lobby_id) -> mm::msg::lobby_cleanup_complete {
				lobby_id: Some(lobby_id.into()),
			})
			.await
		}
	}))
	.buffer_unordered(32)
	.try_collect::<Vec<_>>()
	.await?;

	Ok(team_dev::halt::Response {})
}
