use std::collections::HashMap;

use rivet_api::models;
use rivet_operation::prelude::*;
use types::rivet::{
	backend::{self, pkg::*},
	common,
};

use crate::convert;

pub struct GameState {
	pub prod_config: backend::cdn::NamespaceConfig,
	pub total_player_count: u32,
}

pub async fn summaries(
	ctx: &OperationContext<()>,
	game_ids: Vec<Uuid>,
) -> GlobalResult<Vec<models::GameSummary>> {
	if game_ids.is_empty() {
		return Ok(Vec::new());
	}

	let proto_game_ids = game_ids
		.clone()
		.into_iter()
		.map(Into::into)
		.collect::<Vec<_>>();

	let ((games, dev_teams), states) = tokio::try_join!(
		games_and_dev_teams(ctx, proto_game_ids.clone()),
		state(ctx, proto_game_ids.clone()),
	)?;

	// Convert all data
	games
		.games
		.iter()
		.map(|game| {
			let game_id = unwrap_ref!(game.game_id).as_uuid();
			let state = unwrap!(states.get(&game_id));
			let dev_team = unwrap!(dev_teams.get(&game_id));

			convert::game::summary(game, state, dev_team)
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

pub async fn state(
	ctx: &OperationContext<()>,
	game_ids: Vec<common::Uuid>,
) -> GlobalResult<HashMap<Uuid, GameState>> {
	let namespaces_res = op!([ctx] game_namespace_list {
		game_ids: game_ids,
	})
	.await?;
	let all_namespace_ids = namespaces_res
		.games
		.iter()
		.flat_map(|game| game.namespace_ids.iter().cloned())
		.collect::<Vec<_>>();

	let (game_namespaces_res, player_count_res) = tokio::try_join!(
		op!([ctx] game_namespace_get {
			namespace_ids: all_namespace_ids.clone(),
		}),
		op!([ctx] mm_player_count_for_namespace {
			namespace_ids: all_namespace_ids,
		}),
	)?;

	let mut prod_namespaces = HashMap::new();
	for namespace in &game_namespaces_res.namespaces {
		if &namespace.name_id == "prod" {
			let game_id = unwrap_ref!(namespace.game_id).as_uuid();
			let namespace_id = unwrap!(namespace.namespace_id).as_uuid();

			prod_namespaces.insert(namespace_id, game_id);
		}
	}

	let cdn_namespaces_res = op!([ctx] cdn_namespace_get {
		namespace_ids: prod_namespaces
			.keys()
			.cloned()
			.map(Into::into)
			.collect::<Vec<_>>(),
	})
	.await?;

	let cdn_configs = cdn_namespaces_res
		.namespaces
		.iter()
		.map(|ns| {
			let namespace_id = unwrap_ref!(ns.namespace_id).as_uuid();
			let game_id = unwrap!(prod_namespaces.get(&namespace_id));
			let config = unwrap_ref!(ns.config).clone();

			// Fetch all namespace ids for game
			let game_id_proto = Some(Into::<common::Uuid>::into(*game_id));
			let all_namespace_ids = &unwrap!(namespaces_res
				.games
				.iter()
				.find(|game| game.game_id == game_id_proto))
			.namespace_ids;

			let total_player_count = player_count_res
				.namespaces
				.iter()
				.filter(|ns1| {
					// Make sure this namespace belongs to this game
					all_namespace_ids.iter().any(|ns2_id| {
						ns1.namespace_id
							.as_ref()
							.map_or(false, |ns1_id| ns1_id == ns2_id)
					})
				})
				// Aggregate the player count
				.fold(0u32, |acc, x| acc + x.player_count);

			Ok((
				*game_id,
				GameState {
					prod_config: config,
					total_player_count,
				},
			))
		})
		.collect::<GlobalResult<HashMap<_, _>>>()?;

	Ok(cdn_configs)
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
