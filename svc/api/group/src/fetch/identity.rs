use std::collections::{HashMap, HashSet};

use api_helper::ctx::Ctx;
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_operation::prelude::*;

use crate::{auth::Auth, convert, fetch};

pub struct PresencesCtx {
	pub res: user_presence::get::Response,
	pub games: Vec<backend::game::Game>,
	pub games_with_namespace_ids: Vec<convert::GameWithNamespaceIds>,
}

pub async fn users(
	ctx: &Ctx<Auth>,
	user_ids: Vec<common::Uuid>,
) -> GlobalResult<user::get::Response> {
	op!([ctx] user_get {
		user_ids: user_ids,
	})
	.await
}

pub async fn presence_data(
	ctx: &Ctx<Auth>,
	current_user_id: Uuid,
	user_ids: Vec<common::Uuid>,
	summary_info: bool,
) -> GlobalResult<PresencesCtx> {
	let ((presences_res, game_ids),) = tokio::try_join!(presences_and_game_ids(ctx, user_ids),)?;
	let (games, games_with_namespace_ids) = games(
		ctx,
		game_ids
			.into_iter()
			.map(Into::<common::Uuid>::into)
			.collect::<Vec<_>>(),
		Vec::new(),
	)
	.await?;

	Ok(PresencesCtx {
		res: presences_res,
		games,
		games_with_namespace_ids,
	})
}

async fn presences_and_game_ids(
	ctx: &Ctx<Auth>,
	user_ids: Vec<common::Uuid>,
) -> GlobalResult<(user_presence::get::Response, Vec<Uuid>)> {
	let user_presences_res = op!([ctx] user_presence_get {
		user_ids: user_ids,
	})
	.await?;

	// Fetch game ids from game activities
	let game_ids = user_presences_res
		.users
		.iter()
		.filter_map(|user_presence| {
			if let Some(backend::user::Presence {
				game_activity:
					Some(backend::user::presence::GameActivity {
						game_id: Some(game_id),
						..
					}),
				..
			}) = &user_presence.presence
			{
				Some(game_id.as_uuid())
			} else {
				None
			}
		})
		.collect::<HashSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();

	Ok((user_presences_res, game_ids))
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
			.map(|game| Ok(internal_unwrap_owned!(game.game_id)))
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

pub async fn follows(
	ctx: &Ctx<Auth>,
	current_user_id: Uuid,
	raw_user_ids: Vec<Uuid>,
) -> GlobalResult<user_follow::get::Response> {
	// Converts to hashmap to remove duplicate queries
	let queries = raw_user_ids
		.clone()
		.into_iter()
		.flat_map(|user_id| [(current_user_id, user_id), (user_id, current_user_id)])
		.collect::<HashSet<_>>()
		.into_iter()
		.map(|(user_a_id, user_b_id)| user_follow::get::request::Query {
			follower_user_id: Some(user_a_id.into()),
			following_user_id: Some(user_b_id.into()),
		})
		.collect::<Vec<_>>();

	op!([ctx] user_follow_get {
		queries: queries,
	})
	.await
}
