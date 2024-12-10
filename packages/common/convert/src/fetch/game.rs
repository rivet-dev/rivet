use std::collections::HashMap;

use rivet_api::models;
use rivet_operation::prelude::*;
use types_proto::rivet::{
	backend::{self, pkg::*},
	common,
};

use crate::convert;

pub async fn summaries(
	ctx: &OperationContext<()>,
	game_ids: Vec<Uuid>,
) -> GlobalResult<Vec<models::GameGameSummary>> {
	if game_ids.is_empty() {
		return Ok(Vec::new());
	}

	let proto_game_ids = game_ids
		.clone()
		.into_iter()
		.map(Into::into)
		.collect::<Vec<_>>();

	let (games, dev_teams) = games_and_dev_teams(ctx, proto_game_ids.clone()).await?;

	// Convert all data
	games
		.games
		.iter()
		.map(|game| {
			let game_id = unwrap_ref!(game.game_id).as_uuid();
			let dev_team = unwrap!(dev_teams.get(&game_id));

			convert::game::summary(ctx.config(), game, dev_team)
		})
		.collect::<GlobalResult<Vec<_>>>()
}

pub async fn games_and_dev_teams(
	ctx: &OperationContext<()>,
	game_ids: Vec<common::Uuid>,
) -> GlobalResult<(game::get::Response, HashMap<Uuid, backend::team::Team>)> {
	let games_res = op!([ctx] game_get {
		game_ids: game_ids,
	})
	.await?;

	let dev_teams_res = op!([ctx] team_get {
		team_ids: games_res
			.games
			.iter()
			.map(|g| Ok(unwrap!(g.developer_team_id)))
			.collect::<GlobalResult<_>>()?,
	})
	.await?;

	let dev_teams = games_res
		.games
		.iter()
		.map(|game| {
			let team = unwrap!(dev_teams_res
				.teams
				.iter()
				.find(|team| team.team_id == game.developer_team_id));
			let game_id = unwrap_ref!(game.game_id).as_uuid();

			Ok((game_id, team.clone()))
		})
		.collect::<GlobalResult<HashMap<_, _>>>()?;

	Ok((games_res, dev_teams))
}

pub async fn region_summaries(
	ctx: &OperationContext<()>,
	game_id: Uuid,
) -> GlobalResult<Vec<models::CloudRegionSummary>> {
	let list_res = op!([ctx] region_list_for_game {
		game_ids: vec![game_id.into()],
	})
	.await?;

	let get_res = op!([ctx] region_get {
		region_ids: list_res.region_ids.clone(),
	})
	.await?;

	get_res
		.regions
		.iter()
		.map(convert::game::region_summary)
		.collect()
}
