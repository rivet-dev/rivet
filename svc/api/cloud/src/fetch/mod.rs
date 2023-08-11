// This module fetches information used to convert proto data into smithy models. It's important to separate
// fetching from building models so that we can convert already existing data without having to re-fetch it.

use rivet_cloud_server::models;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;

pub mod group;

pub async fn game_handle_fetch_one(
	ctx: &OperationContext<()>,
	game_id: Uuid,
) -> GlobalResult<models::GameHandle> {
	let game = game_handle_fetch(ctx, vec![game_id])
		.await?
		.into_iter()
		.next();
	Ok(internal_unwrap_owned!(game))
}

pub async fn game_handle_fetch(
	ctx: &OperationContext<()>,
	game_ids: Vec<Uuid>,
) -> GlobalResult<Vec<models::GameHandle>> {
	// Fetch namespaces
	let games_res = op!([ctx] game_get {
		game_ids: game_ids
			.iter()
			.cloned()
			.map(Into::<common::Uuid>::into)
			.collect::<Vec<_>>(),
	})
	.await?;

	games_res
		.games
		.into_iter()
		.map(ApiTryInto::try_into)
		.collect::<Result<Vec<_>, _>>()
}

pub async fn game_summary_fetch_one(
	ctx: &OperationContext<()>,
	game_id: Uuid,
) -> GlobalResult<Option<models::GameSummary>> {
	let game = game_summary_fetch(ctx, vec![game_id])
		.await?
		.into_iter()
		.next();
	Ok(game)
}

pub async fn game_summary_fetch(
	ctx: &OperationContext<()>,
	game_ids: Vec<Uuid>,
) -> GlobalResult<Vec<models::GameSummary>> {
	// Fetch namespaces
	let ns_list_res = op!([ctx] game_namespace_list {
		game_ids: game_ids
			.iter()
			.cloned()
			.map(Into::<common::Uuid>::into)
			.collect::<Vec<_>>(),
	})
	.await?;
	let all_namespace_ids = &ns_list_res
		.games
		.iter()
		.flat_map(|game| game.namespace_ids.clone())
		.collect::<Vec<_>>();

	// Fetch game data
	let game_ids_proto = game_ids
		.iter()
		.cloned()
		.map(Into::<common::Uuid>::into)
		.collect::<Vec<_>>();
	let (game_configs_res, games_res, player_count_res) = tokio::try_join!(
		op!([ctx] cloud_game_config_get {
			game_ids: game_ids_proto.clone(),
		}),
		op!([ctx] game_get {
			game_ids: game_ids_proto.clone(),
		}),
		op!([ctx] mm_player_count_for_namespace {
			namespace_ids: all_namespace_ids.clone(),
		}),
	)?;

	// Compile game configs and games together. If there is no matching game config for a given
	// game, we exclude the game altogether.
	let mut games = Vec::new();
	for game_config in &game_configs_res.game_configs {
		// Get game data
		let game_id = internal_unwrap!(game_config.game_id).as_uuid();
		let namespace_ids_proto = internal_unwrap_owned!(ns_list_res
			.games
			.iter()
			.find(|game| game.game_id == game_config.game_id));
		let namespace_ids_proto = &namespace_ids_proto.namespace_ids;
		let game = internal_unwrap_owned!(games_res
			.games
			.iter()
			.find(|g| g.game_id == game_config.game_id));
		let developer_team_id = internal_unwrap!(game.developer_team_id).as_uuid();

		// Get the total player count across all namespaces
		let total_player_count = player_count_res
			.namespaces
			.iter()
			.filter(|ns1| {
				// Make sure this namespace belongs to this game
				namespace_ids_proto.iter().any(|ns2_id| {
					ns1.namespace_id
						.as_ref()
						.map_or(false, |ns1_id| ns1_id == ns2_id)
				})
			})
			// Aggregate the player count
			.fold(0u32, |acc, x| acc + x.player_count);

		games.push(models::GameSummary {
			game_id: game_id.to_string(),
			create_ts: util::timestamp::to_chrono(game.create_ts)?,
			name_id: game.name_id.clone(),
			display_name: game.display_name.clone(),
			developer_group_id: developer_team_id.to_string(),
			total_player_count: total_player_count.try_into()?,
			logo_url: util::route::game_logo(
				&game),
			banner_url: util::route::game_banner(
				&game),
		});
	}

	Ok(games)
}

pub async fn region_summary_fetch_all(
	ctx: &OperationContext<()>,
) -> GlobalResult<Vec<models::RegionSummary>> {
	let list_res = op!([ctx] region_list {
		..Default::default()
	})
	.await?;

	let get_res = op!([ctx] region_get {
		region_ids: list_res.region_ids.clone(),
	})
	.await?;

	get_res
		.regions
		.iter()
		.map(|r| {
			GlobalResult::Ok(models::RegionSummary {
				region_id: internal_unwrap!(r.region_id).as_uuid().to_string(),
				region_name_id: r.name_id.clone(),
				provider: r.provider.clone(),
				universal_region: r.universal_region as i16,
				provider_display_name: r.provider_display_name.clone(),
				region_display_name: r.region_display_name.clone(),
			})
		})
		.collect()
}
