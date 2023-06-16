use std::collections::HashMap;

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "team-billing-aggregate")]
async fn handle(
	ctx: OperationContext<team::billing_aggregate::Request>,
) -> GlobalResult<team::billing_aggregate::Response> {
	// TODO: Assert no duplicate team IDs

	let regions_res = op!([ctx] region_list { }).await?;

	// Fetch game IDs
	let team_games_res = op!([ctx] team_dev_game_list {
		team_ids: ctx.teams.iter().filter_map(|x| x.team_id).collect(),
	})
	.await?;

	let all_game_ids = team_games_res
		.teams
		.iter()
		.flat_map(|x| x.game_ids.iter().cloned())
		.collect::<Vec<common::Uuid>>();
	let game_ns_res = op!([ctx] game_namespace_list {
		game_ids: all_game_ids,
	})
	.await?;

	// Calculate metrics
	// TODO: Parallelize this
	let mut team_metrics = Vec::new();
	for team_req in &ctx.teams {
		// Find game and namespace IDs for the team
		let game_ids = &internal_unwrap_owned!(
			team_games_res
				.teams
				.iter()
				.find(|x| x.team_id == team_req.team_id),
			"can't find game list for team"
		)
		.game_ids;
		let namespace_ids = game_ns_res
			.games
			.iter()
			.filter(|game| {
				game.game_id
					.as_ref()
					.map_or(false, |game_id| game_ids.contains(game_id))
			})
			.flat_map(|x| x.namespace_ids.clone())
			.collect::<Vec<common::Uuid>>();

		// Aggregate the lobby runtime
		let runtime_res = op!([ctx] mm_lobby_runtime_aggregate {
			namespace_ids: namespace_ids.clone(),
			query_start: team_req.query_start,
			query_end: team_req.query_end
		})
		.await?;

		let games_res = op!([ctx] game_resolve_namespace_id {
			namespace_ids: runtime_res
				.region_tier_times
				.iter()
				.map(|rtt| Ok(internal_unwrap_owned!(rtt.namespace_id)))
				.collect::<GlobalResult<Vec<_>>>()?,
		})
		.await?;

		let mut metrics_by_game =
			HashMap::<Uuid, Vec<mm::lobby_runtime_aggregate::response::RegionTierTime>>::new();

		tracing::info!(games=?games_res.games, rtt=?runtime_res.region_tier_times, "fetched game and regions");

		// Collect times into hashmap by game id
		for region_tier_time in &runtime_res.region_tier_times {
			let namespace_id = internal_unwrap_owned!(region_tier_time.namespace_id);

			let game = internal_unwrap_owned!(
				games_res
					.games
					.iter()
					.find(|game| game.namespace_ids.contains(&namespace_id)),
				"no game found for namespace"
			);

			let entry = metrics_by_game
				.entry(internal_unwrap!(game.game_id).as_uuid())
				.or_insert_with(Vec::new);

			entry.push(region_tier_time.clone());
		}

		// Build game metrics
		let games = metrics_by_game
			.into_iter()
			.map(|(game_id, region_tier_times)| {
				Ok(backend::billing::GameLobbyMetrics {
					game_id: Some(game_id.into()),
					metrics: region_tier_times
						.iter()
						.map(|region_tier_time| {
							let uptime_in_seconds =
								util::div_up!(region_tier_time.total_time, 1_000);

							Ok(backend::billing::RegionTierMetrics {
								namespace_id: region_tier_time.namespace_id,
								region_id: region_tier_time.region_id,
								tier_name_id: region_tier_time.tier_name_id.clone(),
								lobby_group_name_id: region_tier_time.lobby_group_name_id.clone(),
								uptime: uptime_in_seconds,
							})
						})
						.collect::<GlobalResult<Vec<_>>>()?,
				})
			})
			.collect::<GlobalResult<Vec<_>>>()?;

		team_metrics.push(team::billing_aggregate::response::TeamBillingMetrics {
			team_id: team_req.team_id,
			games,
		});
	}

	Ok(team::billing_aggregate::Response {
		teams: team_metrics,
	})
}
