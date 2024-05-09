use std::{
	collections::{HashMap, HashSet},
	convert::TryInto,
};

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend::pkg::*;
use rivet_api::models;
use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;
use serde::Deserialize;
use serde_json::json;

use crate::auth::Auth;

// MARK: POST /players/connected
pub async fn connected(
	ctx: Ctx<Auth>,
	body: models::MatchmakerPlayersConnectedRequest,
) -> GlobalResult<serde_json::Value> {
	// Mock response
	if ctx.auth().game_ns_dev_option()?.is_some() {
		let _player_ent = decode_player_token_dev(&body.player_token)?;
		return Ok(json!({}));
	}

	let lobby_ent = ctx.auth().lobby()?;
	let player_ent = decode_player_token(&body.player_token)?;

	let res = msg!([ctx] mm::msg::player_register(player_ent.player_id) -> Result<mm::msg::player_register_complete, mm::msg::player_register_fail> {
		player_id: Some(player_ent.player_id.into()),
		lobby_id: Some(lobby_ent.lobby_id.into()),
	})
	.await?;

	match res {
		Ok(_) => Ok(json!({})),
		Err(msg) => {
			use mm::msg::player_register_fail::ErrorCode;

			match ErrorCode::from_i32(msg.error_code) {
				Some(ErrorCode::PlayerAlreadyRegistered) => {
					bail_with!(MATCHMAKER_PLAYER_ALREADY_CONNECTED);
				}
				Some(ErrorCode::PlayerRemoved) => {
					bail_with!(MATCHMAKER_PLAYER_REMOVED);
				}
				Some(ErrorCode::RegistrationExpired) => {
					bail_with!(MATCHMAKER_PLAYER_REGISTRATION_EXPIRED);
				}
				Some(ErrorCode::PlayerInDifferentLobby) => {
					bail_with!(MATCHMAKER_PLAYER_IN_DIFFERENT_LOBBY);
				}
				Some(ErrorCode::DeprecatedPlayerNotFound) | Some(ErrorCode::Unknown) | None => {
					tracing::error!("unknown player register error {:?}", msg);
					bail!("unknown player register error");
				}
			}
		}
	}
}

// MARK: POST /players/disconnected
pub async fn disconnected(
	ctx: Ctx<Auth>,
	body: models::MatchmakerPlayersConnectedRequest,
) -> GlobalResult<serde_json::Value> {
	// Mock response
	if ctx.auth().game_ns_dev_option()?.is_some() {
		let _player_ent = decode_player_token_dev(&body.player_token)?;
		return Ok(json!({}));
	}

	let lobby_ent = ctx.auth().lobby()?;
	let player_ent = decode_player_token(&body.player_token)?;

	let res = msg!([ctx] mm::msg::player_remove(player_ent.player_id) -> Result<mm::msg::player_remove_complete, mm::msg::player_remove_fail> {
		player_id: Some(player_ent.player_id.into()),
		lobby_id: Some(lobby_ent.lobby_id.into()),
		..Default::default()
	})
	.await?;

	match res {
		Ok(_) => Ok(json!({})),
		Err(msg) => {
			use mm::msg::player_remove_fail::ErrorCode;

			match ErrorCode::from_i32(msg.error_code) {
				Some(ErrorCode::PlayerInDifferentLobby) => {
					bail_with!(MATCHMAKER_PLAYER_IN_DIFFERENT_LOBBY);
				}
				Some(ErrorCode::DeprecatedPlayerNotFound) | Some(ErrorCode::Unknown) | None => {
					tracing::error!("unknown player remove error {:?}", msg);
					bail!("unknown player remove error");
				}
			}
		}
	}
}

fn decode_player_token(token: &str) -> GlobalResult<rivet_claims::ent::MatchmakerPlayer> {
	let player_claims = rivet_claims::decode(token)
		.map_err(|_| err_code!(TOKEN_ERROR, error = "Malformed player token"))?
		.map_err(|_| err_code!(TOKEN_ERROR, error = "Invalid player token"))?;

	let player_ent = player_claims.as_matchmaker_player()?;
	Ok(player_ent)
}

fn decode_player_token_dev(
	token: &str,
) -> GlobalResult<rivet_claims::ent::MatchmakerDevelopmentPlayer> {
	let player_claims = rivet_claims::decode(token)
		.map_err(|_| err_code!(TOKEN_ERROR, error = "Malformed player token"))?
		.map_err(|_| err_code!(TOKEN_ERROR, error = "Invalid player token"))?;

	let player_ent = player_claims.as_matchmaker_development_player()?;
	Ok(player_ent)
}

// MARK: GET /players/statistics
#[derive(Debug, Deserialize)]
pub struct GetStatisticsQuery {
	#[serde(default)]
	exclude_outdated: bool,
}

pub async fn statistics(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: GetStatisticsQuery,
) -> GlobalResult<models::MatchmakerGetStatisticsResponse> {
	// Mock response
	if ctx.auth().game_ns_dev_option()?.is_some() {
		return Ok(models::MatchmakerGetStatisticsResponse {
			player_count: 0,
			game_modes: HashMap::new(),
		});
	}

	let game_ns = ctx.auth().game_ns(&ctx).await?;
	let namespace_id = game_ns.namespace_id;
	let exclude_outdated = query.exclude_outdated;

	// Fetch lobbies and namespace info
	let (lobbies_res, namespaces_res) = tokio::try_join!(
		op!([ctx] mm_lobby_list_for_namespace {
			namespace_ids: vec![namespace_id.into()],
		}),
		op!([ctx] game_namespace_get {
			namespace_ids: vec![namespace_id.into()],
		}),
	)?;
	let namespace = unwrap!(namespaces_res.namespaces.first());
	let lobby_ids = unwrap!(lobbies_res.namespaces.first()).lobby_ids.clone();
	let latest_version_id = unwrap!(namespace.version_id);

	// Fetch lobby info, player counts, and version configs (if needed)
	let (lobbies_res, lobby_player_counts_res) = tokio::try_join!(
		op!([ctx] mm_lobby_get {
			lobby_ids: lobby_ids.clone(),
		}),
		op!([ctx] mm_lobby_player_count {
			lobby_ids: lobby_ids,
		}),
	)?;

	// Fetch version (and lobby group configs)
	let versions_res = if exclude_outdated {
		op!([ctx] mm_config_version_get {
			version_ids: vec![latest_version_id],
		})
		.await?
	} else {
		// Fetch lobby group
		let all_lobby_group_ids = lobbies_res
			.lobbies
			.iter()
			.map(|lobby| Ok(unwrap!(lobby.lobby_group_id).as_uuid()))
			.collect::<GlobalResult<HashSet<_>>>()?
			.into_iter()
			.map(Into::into)
			.collect::<Vec<_>>();
		let lobby_group_versions_res = op!([ctx] mm_config_lobby_group_resolve_version {
			lobby_group_ids: all_lobby_group_ids,
		})
		.await?;

		op!([ctx] mm_config_version_get {
			version_ids: lobby_group_versions_res.versions
				.iter()
				.map(|v| Ok(unwrap!(v.version_id)))
				.collect::<GlobalResult<Vec<_>>>()?,
		})
		.await?
	};

	let all_lobby_groups = versions_res
		.versions
		.iter()
		.map(|v| Ok((unwrap_ref!(v.config), unwrap_ref!(v.config_meta))))
		.collect::<GlobalResult<Vec<_>>>()?
		.iter()
		.flat_map(|(v, v_meta)| v.lobby_groups.iter().zip(v_meta.lobby_groups.iter()))
		.map(|(lg, lgm)| Ok((unwrap_ref!(lgm.lobby_group_id).as_uuid(), lg)))
		.collect::<GlobalResult<HashMap<_, _>>>()?;

	// Fetch all region configs
	let all_region_ids = all_lobby_groups
		.iter()
		.flat_map(|(_, lg)| lg.regions.iter())
		.map(|r| Ok(unwrap_ref!(r.region_id).as_uuid()))
		.collect::<GlobalResult<HashSet<_>>>()?
		.into_iter()
		.map(Into::into)
		.collect::<Vec<_>>();
	let regions_res = op!([ctx] region_get {
		region_ids: all_region_ids,
	})
	.await?;

	let mut total_player_count = 0i64;
	let mut per_game_mode = HashMap::new();

	for lobby in &lobbies_res.lobbies {
		// Skip if outdated
		if exclude_outdated {
			let is_not_outdated = versions_res.versions.iter().any(|v| {
				v.config_meta
					.as_ref()
					.map(|c| {
						c.lobby_groups
							.iter()
							.any(|lg| lg.lobby_group_id == lobby.lobby_group_id)
					})
					.unwrap_or_default()
			});

			if !is_not_outdated {
				continue;
			}
		}

		let player_count = TryInto::<i64>::try_into(
			unwrap!(lobby_player_counts_res
				.lobbies
				.iter()
				.find(|l| l.lobby_id == lobby.lobby_id))
			.total_player_count,
		)?;
		let lobby_group_id = unwrap_ref!(lobby.lobby_group_id).as_uuid();
		let lobby_group = unwrap!(all_lobby_groups.get(&lobby_group_id));
		let region = unwrap!(regions_res
			.regions
			.iter()
			.find(|lg| lg.region_id == lobby.region_id));

		// Aggregate total
		total_player_count += player_count;

		// Aggregate lobby group subtotal
		let gm_entry = per_game_mode
			.entry(lobby_group.name_id.clone())
			.or_insert_with(|| models::MatchmakerGameModeStatistics {
				player_count: 0,
				regions: HashMap::with_capacity(1),
			});
		gm_entry.player_count += player_count;

		// Aggregate region subtotal
		let region_entry = gm_entry
			.regions
			.entry(region.name_id.clone())
			.or_insert_with(|| models::MatchmakerRegionStatistics { player_count: 0 });
		region_entry.player_count += player_count;
	}

	// Fill in empty game modes and regions
	for lg in all_lobby_groups.values() {
		let game_mode = per_game_mode.entry(lg.name_id.clone()).or_insert_with(|| {
			models::MatchmakerGameModeStatistics {
				player_count: 0,
				regions: HashMap::with_capacity(1),
			}
		});

		for region in &regions_res.regions {
			game_mode
				.regions
				.entry(region.name_id.clone())
				.or_insert_with(|| models::MatchmakerRegionStatistics { player_count: 0 });
		}
	}

	Ok(models::MatchmakerGetStatisticsResponse {
		player_count: total_player_count,
		game_modes: per_game_mode,
	})
}
