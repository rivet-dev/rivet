use std::collections::HashMap;

use rivet_api::models;
use rivet_operation::prelude::*;
use types::rivet::{
	backend::{self, pkg::*},
	common,
};

use crate::convert;

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

	let ((games, dev_teams), configs) = tokio::try_join!(
		games_and_dev_teams(ctx, proto_game_ids.clone()),
		cdn_config(ctx, proto_game_ids.clone()),
	)?;

	// Convert all data
	games
		.games
		.iter()
		.map(|game| {
			let game_id = unwrap_ref!(game.game_id).as_uuid();
			let cdn_config = unwrap!(configs.get(&game_id));
			let dev_team = unwrap!(dev_teams.get(&game_id));

			convert::game::summary(game, cdn_config, dev_team)
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

pub async fn cdn_config(
	ctx: &OperationContext<()>,
	game_ids: Vec<common::Uuid>,
) -> GlobalResult<HashMap<Uuid, backend::cdn::NamespaceConfig>> {
	let namespaces_res = op!([ctx] game_namespace_list {
		game_ids: game_ids,
	})
	.await?;

	let game_namespaces_res = op!([ctx] game_namespace_get {
		namespace_ids: namespaces_res.games
			.iter()
			.flat_map(|g| g.namespace_ids.iter().cloned())
			.collect::<Vec<_>>(),
	})
	.await?;

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

			Ok((*game_id, config))
		})
		.collect::<GlobalResult<HashMap<_, _>>>()?;

	Ok(cdn_configs)
}
