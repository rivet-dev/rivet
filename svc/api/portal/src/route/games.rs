use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use rivet_operation::prelude::*;
use rivet_portal_server::models;

use crate::{auth::Auth, build, convert};

// TODO: Implement watching game update events
// MARK: GET /games/{}/profile
pub async fn profile(
	ctx: Ctx<Auth>,
	game_name_id: String,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetGameProfileResponse> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	// Resolve game ID
	let game_resolve_res = op!([ctx] game_resolve_name_id {
		name_ids: vec![game_name_id.to_owned()],
	})
	.await?;
	let game_id = unwrap_ref!(game_resolve_res.games.first()).game_id;
	let game_id = unwrap_ref!(game_id).as_uuid();

	// Fetch game data
	let game_res = op!([ctx] game_get {
		game_ids: vec![game_id.into()],
	})
	.await?;
	let game = unwrap!(game_res.games.first());
	let team_id = unwrap_ref!(game.developer_team_id);

	// Fetch developer team
	let developer_team = build::group_summaries(&ctx, user_ent.user_id.into(), &[*team_id]).await?;
	let developer_team = unwrap!(developer_team.first());

	Ok(models::GetGameProfileResponse {
		game: models::GameProfile {
			game_id: game_id.to_string(),
			name_id: game.name_id.clone(),
			display_name: game.display_name.clone(),
			logo_url: util::route::game_logo(&game),
			banner_url: util::route::game_banner(&game),

			url: game.url.clone(),
			developer: developer_team.clone(),
			tags: game.tags.clone(),

			description: game.description.clone(),
			platforms: Vec::new(),                       // TODO:
			recommended_groups: Vec::new(),              // TODO:
			identity_leaderboard_categories: Vec::new(), // TODO:
			group_leaderboard_categories: Vec::new(),    // TODO:
		},
		watch: convert::watch_response(WatchResponse::new(ctx.op_ctx().ts())),
	})
}

// MARK: GET /games
pub async fn get_suggested_games(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetSuggestedGamesResponse> {
	let _user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	// TODO:

	Ok(models::GetSuggestedGamesResponse {
		games: Vec::new(),
		watch: convert::watch_response(WatchResponse::new(ctx.op_ctx().ts())),
	})
}
