use api_helper::ctx::Ctx;
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_operation::prelude::*;

use crate::{auth::Auth, convert};

pub async fn users(
	ctx: &Ctx<Auth>,
	user_ids: Vec<common::Uuid>,
) -> GlobalResult<user::get::Response> {
	op!([ctx] user_get {
		user_ids: user_ids,
	})
	.await
}

async fn games(
	ctx: &Ctx<Auth>,
	game_ids: Vec<common::Uuid>,
	namespace_ids: Vec<common::Uuid>,
) -> GlobalResult<(Vec<backend::game::Game>, Vec<convert::GameWithNamespaceIds>)> {
	let game_resolve_res = op!([ctx] game_resolve_namespace_id {
		namespace_ids: namespace_ids,
	})
	.await?;

	let games_res = op!([ctx] game_get {
		game_ids: game_resolve_res
			.games
			.iter()
			.map(|game| Ok(unwrap!(game.game_id)))
			.collect::<GlobalResult<Vec<_>>>()?
			.into_iter()
			.chain(game_ids)
			.collect::<Vec<_>>(),
	})
	.await?;

	// Collects games and their namespace ids together
	let games_with_namespace_ids = game_resolve_res
		.games
		.iter()
		.filter_map(|resolved_game| {
			games_res
				.games
				.iter()
				.find(|game| resolved_game.game_id == game.game_id)
				.map(|game| {
					Ok(convert::GameWithNamespaceIds {
						namespace_ids: resolved_game
							.namespace_ids
							.iter()
							.map(common::Uuid::as_uuid)
							.collect::<Vec<_>>(),
						game: game.clone(),
					})
				})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok((games_res.games, games_with_namespace_ids))
}
