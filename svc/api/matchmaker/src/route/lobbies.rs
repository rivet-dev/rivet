use std::{
	collections::{HashMap, HashSet},
	str::FromStr,
};

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_api::models;
use rivet_convert::{ApiInto, ApiTryInto};
use rivet_operation::prelude::*;
use serde::Deserialize;
use serde_json::json;

use crate::{
	auth::Auth,
	fetch::game::{fetch_ns, NamespaceData},
	utils,
};

// MARK: POST /lobbies/ready
pub async fn ready(ctx: Ctx<Auth>, _body: serde_json::Value) -> GlobalResult<serde_json::Value> {
	// Mock response
	if ctx.auth().game_ns_dev_option()?.is_some() {
		return Ok(json!({}));
	}

	let lobby_ent = ctx.auth().lobby()?;

	msg!([ctx] mm::msg::lobby_ready(lobby_ent.lobby_id) {
		lobby_id: Some(lobby_ent.lobby_id.into()),
	})
	.await?;

	Ok(json!({}))
}

// MARK: POST /lobbies/join
pub async fn join(
	ctx: Ctx<Auth>,
	body: models::MatchmakerLobbiesJoinRequest,
) -> GlobalResult<models::MatchmakerJoinLobbyResponse> {
	// Mock response
	if let Some(ns_dev_ent) = ctx.auth().game_ns_dev_option()? {
		let FindResponse {
			lobby,
			ports,
			player,
		} = dev_mock_lobby(&ctx, &ns_dev_ent).await?;
		return Ok(models::MatchmakerJoinLobbyResponse {
			lobby,
			ports,
			player,
		});
	}

	let game_ns = ctx.auth().game_ns(&ctx).await?;
	let ns_data = fetch_ns(&ctx, &game_ns).await?;
	let lobby_id = Uuid::from_str(body.lobby_id.as_str())?;

	let find_query =
		mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
			lobby_id: Some(lobby_id.into()),
		});
	let FindResponse {
		lobby,
		ports,
		player,
	} = find_inner(
		&ctx,
		&ns_data,
		find_query,
		None,
		body.captcha,
		&HashMap::new(),
		None,
		VerificationType::UserData(body.verification_data.flatten()),
	)
	.await?;

	Ok(models::MatchmakerJoinLobbyResponse {
		lobby,
		ports,
		player,
	})
}

// MARK: POST /lobbies/find
pub async fn find(
	ctx: Ctx<Auth>,
	body: models::MatchmakerLobbiesFindRequest,
) -> GlobalResult<models::MatchmakerFindLobbyResponse> {
	let coords = ctx.coords();

	// Mock response
	if let Some(ns_dev_ent) = ctx.auth().game_ns_dev_option()? {
		let FindResponse {
			lobby,
			ports,
			player,
		} = dev_mock_lobby(&ctx, &ns_dev_ent).await?;
		return Ok(models::MatchmakerFindLobbyResponse {
			lobby,
			ports,
			player,
		});
	}

	ensure_with!(
		!body.game_modes.is_empty(),
		MATCHMAKER_NO_GAME_MODE_PROVIDED
	);

	let game_ns = ctx.auth().game_ns(&ctx).await?;

	let (ns_data, game_resolve_res) = tokio::try_join!(
		fetch_ns(&ctx, &game_ns),
		op!([ctx] game_resolve_namespace_id {
			namespace_ids: vec![game_ns.namespace_id.into()],
		}),
	)?;
	let game = unwrap!(game_resolve_res.games.first());
	let game_id = unwrap_ref!(game.game_id);

	// Fetch version data
	let version_res = op!([ctx] mm_config_version_get {
		version_ids: vec![ns_data.version_id.into()],
	})
	.await?;
	let version_data = unwrap!(version_res.versions.first());
	let version_config = unwrap_ref!(version_data.config);
	let version_meta = unwrap_ref!(version_data.config_meta);

	// Find lobby groups that match the requested game modes. This matches the
	// same order as `body.game_modes`.
	let lobby_groups: Vec<(
		&backend::matchmaker::LobbyGroup,
		&backend::matchmaker::LobbyGroupMeta,
	)> = body
		.game_modes
		.iter()
		.map(|name_id| {
			Ok(unwrap_with!(
				version_config
					.lobby_groups
					.iter()
					.zip(version_meta.lobby_groups.iter())
					.find(|(lgc, _)| lgc.name_id == *name_id),
				MATCHMAKER_GAME_MODE_NOT_FOUND
			))
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	// Validate that each lobby is taggable
	let tags = body.tags.unwrap_or_default();
	if !tags.is_empty() {
		ensure_with!(tags.len() <= 8, MATCHMAKER_TOO_MANY_TAGS);

		for (tag_name, tag) in &tags {
			ensure_with!(tag_name.len() <= 128, MATCHMAKER_TAG_NAME_TOO_LONG);
			ensure_with!(tag.len() <= 512, MATCHMAKER_TAG_TOO_LONG);
		}

		for (lgc, _) in &lobby_groups {
			ensure_with!(
				lgc.taggable,
				MATCHMAKER_TAGS_DISABLED,
				game_mode = lgc.name_id
			);
		}
	}

	// Validate player count
	if let Some(max_players) = body.max_players {
		ensure_with!(max_players >= 1, MATCHMAKER_DYNAMIC_MAX_PLAYERS_INVALID);

		for (lgc, _) in &lobby_groups {
			ensure_with!(
				lgc.allow_dynamic_max_players,
				MATCHMAKER_DYNAMIC_MAX_PLAYERS_DISABLED,
				game_mode = lgc.name_id
			);

			let max = lgc.max_players_normal.max(lgc.max_players_direct) as i32;
			ensure_with!(
				max_players <= max,
				MATCHMAKER_DYNAMIC_MAX_PLAYERS_INVALID,
				game_mode = lgc.name_id,
				max = max
			);
		}
	}

	// Resolve region IDs
	let region_ids =
		resolve_region_ids(&ctx, coords, body.regions.as_ref(), game_id, &lobby_groups).await?;

	// Validate that there is a lobby group and region pair that is valid.
	//
	// We also derive the auto create config at the same time, since the
	// auto-create config is the first pair of lobby group and regions that are
	// valid.
	//
	// If an auto-create configuration can't be derived, then there's also no
	// existing lobbies that can exist.
	let mut auto_create = None;
	'lg: for (lgc, lgm) in &lobby_groups {
		// Parse the region IDs for the lobby group
		let lobby_group_region_ids = lgc
			.regions
			.iter()
			.filter_map(|x| x.region_id.as_ref())
			.map(common::Uuid::as_uuid)
			.collect::<Vec<_>>();

		// Find the first region that matches this lobby group
		if let Some(region_id) = region_ids
			.iter()
			.find(|region_id| lobby_group_region_ids.contains(region_id))
		{
			auto_create = Some(backend::matchmaker::query::AutoCreate {
				lobby_group_id: lgm.lobby_group_id,
				region_id: Some((*region_id).into()),
			});
			break 'lg;
		}

		tracing::info!(
			?lgc,
			?lobby_group_region_ids,
			"no regions match the lobby group"
		);
	}

	// Unwrap the auto-create value
	let auto_create = if let Some(auto_create) = auto_create {
		auto_create
	} else {
		bail_with!(MATCHMAKER_AUTO_CREATE_FAILED);
	};

	// Build query and find lobby
	let find_query =
		mm::msg::lobby_find::message::Query::LobbyGroup(backend::matchmaker::query::LobbyGroup {
			lobby_group_ids: lobby_groups
				.iter()
				.filter_map(|(_, lgm)| lgm.lobby_group_id)
				.collect(),
			region_ids: region_ids
				.iter()
				.cloned()
				.map(Into::<common::Uuid>::into)
				.collect(),
			auto_create: if body.prevent_auto_create_lobby == Some(true) {
				None
			} else {
				Some(auto_create)
			},
		});
	let FindResponse {
		lobby,
		ports,
		player,
	} = find_inner(
		&ctx,
		&ns_data,
		find_query,
		None,
		body.captcha,
		&tags,
		body.max_players,
		VerificationType::UserData(body.verification_data.flatten()),
	)
	.await?;

	Ok(models::MatchmakerFindLobbyResponse {
		lobby,
		ports,
		player,
	})
}

// MARK: POST /lobbies/create
pub async fn create(
	ctx: Ctx<Auth>,
	body: models::MatchmakerLobbiesCreateRequest,
) -> GlobalResult<models::MatchmakerCreateLobbyResponse> {
	let coords = ctx.coords();

	// Mock response
	if let Some(ns_dev_ent) = ctx.auth().game_ns_dev_option()? {
		let FindResponse {
			lobby,
			ports,
			player,
		} = dev_mock_lobby(&ctx, &ns_dev_ent).await?;
		return Ok(models::MatchmakerCreateLobbyResponse {
			lobby,
			ports,
			player,
		});
	}

	// Verify bearer auth and get user ID
	let (game_ns, user_id) = tokio::try_join!(
		ctx.auth().game_ns(&ctx),
		ctx.auth().fetch_game_user_option(ctx.op_ctx()),
	)?;
	let (ns_data, game_resolve_res) = tokio::try_join!(
		fetch_ns(&ctx, &game_ns),
		op!([ctx] game_resolve_namespace_id {
			namespace_ids: vec![game_ns.namespace_id.into()],
		}),
	)?;
	let game = unwrap!(game_resolve_res.games.first());
	let game_id = unwrap_ref!(game.game_id);

	// Fetch version config
	let version_config_res = op!([ctx] mm_config_version_get {
		version_ids: vec![ns_data.version_id.into()],
	})
	.await?;
	let version_data = unwrap!(version_config_res.versions.first());
	let version_config = unwrap_ref!(version_data.config);
	let version_meta = unwrap_ref!(version_data.config_meta);

	// Find lobby groups that match the requested game mode
	let (lobby_group, lobby_group_meta) = unwrap_with!(
		version_config
			.lobby_groups
			.iter()
			.zip(version_meta.lobby_groups.iter())
			.find(|(lgc, _)| lgc.name_id == body.game_mode),
		MATCHMAKER_GAME_MODE_NOT_FOUND
	);

	// Validate that each lobby is taggable
	let tags = body.tags.unwrap_or_default();
	if !tags.is_empty() {
		ensure_with!(tags.len() <= 8, MATCHMAKER_TOO_MANY_TAGS);

		for (tag_name, tag) in &tags {
			ensure_with!(tag_name.len() <= 128, MATCHMAKER_TAG_NAME_TOO_LONG);
			ensure_with!(tag.len() <= 512, MATCHMAKER_TAG_TOO_LONG);
		}

		ensure_with!(
			lobby_group.taggable,
			MATCHMAKER_TAGS_DISABLED,
			game_mode = lobby_group.name_id
		);
	}

	// Validate player count
	if let Some(max_players) = body.max_players {
		ensure_with!(max_players >= 1, MATCHMAKER_DYNAMIC_MAX_PLAYERS_INVALID);

		ensure_with!(
			lobby_group.allow_dynamic_max_players,
			MATCHMAKER_DYNAMIC_MAX_PLAYERS_DISABLED,
			game_mode = lobby_group.name_id
		);

		let max = lobby_group
			.max_players_normal
			.max(lobby_group.max_players_direct) as i32;
		ensure_with!(
			max_players <= max,
			MATCHMAKER_DYNAMIC_MAX_PLAYERS_INVALID,
			game_mode = lobby_group.name_id,
			max = max
		);
	}

	let publicity = match body.publicity {
		Some(publicity) => ApiInto::api_into(publicity),
		// Default publicity to public if enabled, otherwise private
		None => {
			if let Some(backend::matchmaker::lobby_group::Actions {
				create:
					Some(backend::matchmaker::CreateConfig {
						enable_public: true,
						..
					}),
				..
			}) = lobby_group.actions.as_ref()
			{
				backend::matchmaker::lobby::Publicity::Public
			} else {
				backend::matchmaker::lobby::Publicity::Private
			}
		}
	};

	let dynamic_max_players = body.max_players.map(ApiTryInto::api_try_into).transpose()?;

	// Verify that lobby creation is enabled and user can create a lobby
	util_mm::verification::verify_config(
		ctx.op_ctx(),
		&util_mm::verification::VerifyConfigOpts {
			kind: util_mm::verification::ConnectionKind::Create,
			namespace_id: ns_data.namespace_id,
			user_id,
			client_info: vec![ctx.client_info()],
			tags: &tags,
			dynamic_max_players,

			lobby_groups: &[lobby_group.clone()],
			lobby_group_meta: &[lobby_group_meta.clone()],
			lobby_info: None,
			lobby_state_json: None,
			lobby_config_json: body
				.lobby_config
				.as_ref()
				.and_then(|o| o.as_ref().map(serde_json::to_string))
				.transpose()?
				.as_deref(),

			verification_data_json: body
				.verification_data
				.as_ref()
				.and_then(|o| o.as_ref().map(serde_json::to_string))
				.transpose()?
				.as_deref(),
			custom_lobby_publicity: Some(publicity),
		},
	)
	.await?;

	// Resolve region IDs
	let region_ids = resolve_region_ids(
		&ctx,
		coords,
		body.region.map(|r| vec![r]).as_ref(),
		game_id,
		&[(lobby_group, lobby_group_meta)],
	)
	.await?;
	let region_id = *unwrap_with!(region_ids.first(), MATCHMAKER_REGION_NOT_FOUND);

	let lobby_id = Uuid::new_v4();
	let lobby_create_res =
		msg!([ctx] mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_create_complete {
			lobby_id: Some(lobby_id.into()),
			namespace_id: Some(ns_data.namespace_id.into()),
			lobby_group_id: lobby_group_meta.lobby_group_id,
			region_id: Some(region_id.into()),
			create_ray_id: Some(ctx.op_ctx().ray_id().into()),
			preemptively_created: false,

			creator_user_id: user_id.map(Into::into),
			is_custom: true,
			publicity: Some(publicity as i32),
			lobby_config_json: body.lobby_config
				.as_ref()
				.map(serde_json::to_string)
				.transpose()?,
			tags: tags,
			dynamic_max_players: dynamic_max_players,
			parameters: Vec::new(),
		})
		.await?;

	// Join the lobby that was just created
	let find_query =
		mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
			lobby_id: lobby_create_res.lobby_id,
		});
	let FindResponse {
		lobby,
		ports,
		player,
	} = find_inner(
		&ctx,
		&ns_data,
		find_query,
		Some(version_config.clone()),
		body.captcha,
		&HashMap::new(),
		body.max_players,
		// Bypassing join verification because this user created the lobby (create verification
		// already happened)
		VerificationType::Bypass,
	)
	.await?;

	// TODO: Cleanup lobby if find failed

	Ok(models::MatchmakerCreateLobbyResponse {
		lobby,
		ports,
		player,
	})
}

#[derive(Deserialize)]
pub struct ListQuery {
	#[serde(default)]
	include_state: bool,
}

// MARK: GET /lobbies/list
pub async fn list(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: ListQuery,
) -> GlobalResult<models::MatchmakerListLobbiesResponse> {
	let coords = ctx.coords();

	// Mock response
	if let Some(ns_dev_ent) = ctx.auth().game_ns_dev_option()? {
		return dev_mock_lobby_list(&ctx, &ns_dev_ent, query.include_state).await;
	}

	let game_ns = ctx.auth().game_ns(&ctx).await?;

	// TODO: Cache this

	// Fetch version config and lobbies
	let (meta, lobbies) = tokio::try_join!(
		fetch_lobby_list_meta(ctx.op_ctx(), game_ns.namespace_id, coords),
		fetch_lobby_list(ctx.op_ctx(), game_ns.namespace_id, query.include_state),
	)?;

	let regions = meta
		.regions
		.iter()
		.map(|(region, recommend)| utils::build_region_openapi(region, recommend.as_ref()))
		.collect::<GlobalResult<Vec<_>>>()?;

	let game_modes = meta
		.lobby_groups
		.iter()
		.map(|(gm, _)| models::MatchmakerGameModeInfo {
			game_mode_id: gm.name_id.clone(),
		})
		.collect();

	// Count lobbies by lobby group id
	let mut lobbies_by_lobby_group_id: HashMap<Uuid, usize> = HashMap::new();
	for lobby in &lobbies {
		let lobby_group_id = unwrap_ref!(lobby.lobby.lobby_group_id).as_uuid();
		let entry = lobbies_by_lobby_group_id.entry(lobby_group_id).or_default();
		*entry += 1;
	}

	let lobbies = lobbies
		.into_iter()
		// Join with lobby group
		.filter_map(|lobby| {
			if let Some((lobby_group, _)) = meta
				.lobby_groups
				.iter()
				.find(|(_, lg)| lg.lobby_group_id == lobby.lobby.lobby_group_id)
			{
				Some((lobby, lobby_group))
			} else {
				// Lobby is outdated
				None
			}
		})
		// Filter out empty and unlistable lobbies
		.filter(|(lobby, lobby_group)| {
			// Hide if not listable
			if !lobby_group.listable {
				return false;
			}

			// Keep if lobby not empty
			if lobby.player_count.registered_player_count != 0 {
				return true;
			}

			// Keep if this is the only lobby in this lobby group (even if its empty)
			if let Some(lobby_group_id) = lobby.lobby.lobby_group_id {
				if *lobbies_by_lobby_group_id
					.get(&lobby_group_id.as_uuid())
					.unwrap_or(&0) == 1
				{
					return true;
				}
			}

			// This lobby is empty (i.e. idle) and should not be listed
			false
		})
		// Build response model
		.map(|(lobby, lobby_group)| {
			let (region, _) = unwrap!(meta
				.regions
				.iter()
				.find(|(r, _)| r.region_id == lobby.lobby.region_id));

			GlobalResult::Ok(models::MatchmakerLobbyInfo {
				region_id: region.name_id.clone(),
				game_mode_id: lobby_group.name_id.clone(),
				lobby_id: unwrap_ref!(lobby.lobby.lobby_id).as_uuid(),
				max_players_normal: ApiTryInto::api_try_into(lobby.lobby.max_players_normal)?,
				max_players_direct: ApiTryInto::api_try_into(lobby.lobby.max_players_direct)?,
				max_players_party: ApiTryInto::api_try_into(lobby.lobby.max_players_party)?,
				total_player_count: ApiTryInto::api_try_into(
					lobby.player_count.registered_player_count,
				)?,
				state: lobby.state.map(Some),
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::MatchmakerListLobbiesResponse {
		game_modes,
		regions,
		lobbies,
	})
}

struct FetchLobbyListMeta {
	lobby_groups: Vec<(
		backend::matchmaker::LobbyGroup,
		backend::matchmaker::LobbyGroupMeta,
	)>,
	regions: Vec<(
		backend::region::Region,
		Option<region::recommend::response::Region>,
	)>,
}

/// Fetches lobby group & region data in order to build the lobby list response.
async fn fetch_lobby_list_meta(
	ctx: &OperationContext<()>,
	namespace_id: Uuid,
	coords: Option<(f64, f64)>,
) -> GlobalResult<FetchLobbyListMeta> {
	let ns_res = op!([ctx] game_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let ns_data = unwrap!(ns_res.namespaces.first());
	let version_id = unwrap_ref!(ns_data.version_id).as_uuid();

	// Read the version config
	let version_res = op!([ctx] mm_config_version_get {
		version_ids: vec![version_id.into()],
	})
	.await?;
	let version = unwrap!(
		version_res.versions.first(),
		"no matchmaker config for namespace"
	);
	let version_config = unwrap_ref!(version.config);
	let version_meta = unwrap_ref!(version.config_meta);
	let lobby_groups = version_config
		.lobby_groups
		.iter()
		.cloned()
		.zip(version_meta.lobby_groups.iter().cloned())
		.collect::<Vec<_>>();

	// Fetch all regions
	let region_ids = version_config
		.lobby_groups
		.iter()
		.flat_map(|lg| lg.regions.iter())
		.filter_map(|r| r.region_id.as_ref())
		.map(common::Uuid::as_uuid)
		.collect::<HashSet<Uuid>>();
	let region_ids_proto = region_ids
		.iter()
		.cloned()
		.map(Into::<common::Uuid>::into)
		.collect::<Vec<_>>();
	let (region_res, recommend_res) = tokio::try_join!(
		// List regions
		op!([ctx] region_get {
			region_ids: region_ids_proto.clone(),
		}),
		// Fetch recommended region if coords are provided
		async {
			if let Some((lat, long)) = coords {
				let res = op!([ctx] region_recommend {
					region_ids: region_ids_proto.clone(),
					coords: Some(backend::net::Coordinates {
						latitude: lat,
						longitude: long,
					}),
					..Default::default()
				})
				.await?;
				GlobalResult::Ok(Some(res))
			} else {
				Ok(None)
			}
		}
	)?;

	Ok(FetchLobbyListMeta {
		lobby_groups,
		regions: region_res
			.regions
			.iter()
			.map(|region| {
				let recommend_region = if let Some(res) = &recommend_res {
					Some(unwrap!(res
						.regions
						.iter()
						.find(|recommend| recommend.region_id == region.region_id)))
				} else {
					None
				};

				GlobalResult::Ok((region.clone(), recommend_region.cloned()))
			})
			.collect::<GlobalResult<Vec<_>>>()?,
	})
}

struct FetchLobbyListEntry {
	lobby: backend::matchmaker::Lobby,
	player_count: mm::lobby_player_count::response::Lobby,
	state: Option<serde_json::Value>,
}

/// Fetches all the lobbies and their associated player counts.
async fn fetch_lobby_list(
	ctx: &OperationContext<()>,
	namespace_id: Uuid,
	include_state: bool,
) -> GlobalResult<Vec<FetchLobbyListEntry>> {
	// Fetch lobby IDs
	let lobby_ids = {
		let lobby_list_res = op!([ctx] mm_lobby_list_for_namespace {
			namespace_ids: vec![namespace_id.into()],
		})
		.await?;
		let lobby_ids = unwrap!(lobby_list_res.namespaces.first());

		lobby_ids.lobby_ids.clone()
	};

	// Fetch all lobbies
	let lobbies = {
		let (lobby_get_res, player_count_res, lobby_states) = tokio::try_join!(
			op!([ctx] mm_lobby_get {
				lobby_ids: lobby_ids.clone(),
				include_stopped: false,
			}),
			op!([ctx] mm_lobby_player_count {
				lobby_ids: lobby_ids.clone(),
			}),
			async {
				if include_state {
					let lobbies_res = op!([ctx] mm_lobby_state_get {
						lobby_ids: lobby_ids.clone(),
					})
					.await?;

					Ok(lobbies_res.lobbies)
				} else {
					Ok(Vec::new())
				}
			},
		)?;

		// Match lobby data with player counts and states
		lobby_get_res
			.lobbies
			.iter()
			.filter(|x| {
				matches!(
					backend::matchmaker::lobby::Publicity::from_i32(x.publicity),
					Some(backend::matchmaker::lobby::Publicity::Public)
				)
			})
			.map(|lobby| {
				let player_count = player_count_res
					.lobbies
					.iter()
					.find(|pc| pc.lobby_id == lobby.lobby_id);
				let state = lobby_states.iter().find(|ls| ls.lobby_id == lobby.lobby_id);

				if let Some(player_count) = player_count {
					Ok(Some(FetchLobbyListEntry {
						lobby: lobby.clone(),
						player_count: player_count.clone(),
						state: state
							.and_then(|s| s.state_json.as_ref())
							.map(|s| serde_json::from_str::<serde_json::Value>(s.as_str()))
							.transpose()?,
					}))
				} else {
					tracing::warn!(?lobby, "lobby without player count");
					Ok(None)
				}
			})
			.filter_map(|res| res.transpose())
			.collect::<GlobalResult<Vec<_>>>()?
	};

	Ok(lobbies)
}

// MARK: PUT /lobbies/closed
pub async fn set_closed(
	ctx: Ctx<Auth>,
	body: models::MatchmakerLobbiesSetClosedRequest,
) -> GlobalResult<serde_json::Value> {
	// Mock response
	if ctx.auth().game_ns_dev_option()?.is_some() {
		return Ok(json!({}));
	}

	let lobby_ent = ctx.auth().lobby()?;

	msg!([ctx] mm::msg::lobby_closed_set(lobby_ent.lobby_id) {
		lobby_id: Some(lobby_ent.lobby_id.into()),
		is_closed: body.is_closed,
	})
	.await?;

	Ok(json!({}))
}

// MARK: PUT /lobbies/state
pub async fn set_state(ctx: Ctx<Auth>, body: bytes::Bytes) -> GlobalResult<serde_json::Value> {
	// Mock response
	if ctx.auth().game_ns_dev_option()?.is_some() {
		return Ok(json!({}));
	}

	let lobby_ent = ctx.auth().lobby()?;
	let state = if !body.is_empty() {
		let parsed = serde_json::from_slice::<serde_json::Value>(&body[..])?;

		Some(serde_json::to_string(&parsed)?)
	} else {
		None
	};

	msg!([ctx] mm::msg::lobby_state_set(lobby_ent.lobby_id) {
		lobby_id: Some(lobby_ent.lobby_id.into()),
		state_json: state,
	})
	.await?;

	Ok(json!({}))
}

// MARK: GET /lobbies/{}/state
pub async fn get_state(
	ctx: Ctx<Auth>,
	lobby_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<Option<serde_json::Value>> {
	// Mock response
	if ctx.auth().game_ns_dev_option()?.is_some() {
		return Ok(Some(json!({})));
	}

	let _lobby_ent = ctx.auth().lobby()?;

	let lobbies_res = op!([ctx] mm_lobby_state_get {
		lobby_ids: vec![lobby_id.into()],
	})
	.await?;
	let lobby = unwrap!(lobbies_res.lobbies.first());

	let state = lobby
		.state_json
		.as_ref()
		.map(|state_json| serde_json::from_str::<serde_json::Value>(state_json))
		.transpose()?;

	Ok(state)
}

// MARK: Utilities
struct FindResponse {
	lobby: Box<models::MatchmakerJoinLobby>,
	ports: HashMap<String, models::MatchmakerJoinPort>,
	player: Box<models::MatchmakerJoinPlayer>,
}

#[derive(Debug)]
enum VerificationType {
	UserData(Option<serde_json::Value>),
	Bypass,
}

#[tracing::instrument(err, skip(ctx, game_ns))]
async fn find_inner(
	ctx: &Ctx<Auth>,
	game_ns: &NamespaceData,
	query: mm::msg::lobby_find::message::Query,
	version_config: Option<backend::matchmaker::VersionConfig>,
	captcha: Option<Box<models::CaptchaConfig>>,
	tags: &HashMap<String, String>,
	dynamic_max_players: Option<i32>,
	verification: VerificationType,
) -> GlobalResult<FindResponse> {
	let (version_config, user_id) = tokio::try_join!(
		// Fetch version config if it was not passed as an argument
		async {
			if let Some(version_config) = version_config {
				Ok(version_config)
			} else {
				let version_config_res = op!([ctx] mm_config_version_get {
					version_ids: vec![game_ns.version_id.into()],
				})
				.await?;

				let version_config = unwrap!(version_config_res.versions.first());
				Ok(unwrap_ref!(version_config.config).clone())
			}
		},
		ctx.auth().fetch_game_user_option(ctx.op_ctx()),
	)?;

	// Validate captcha
	if let Some(captcha_config) = &version_config.captcha {
		let origin_host = ctx
			.origin()
			.and_then(|origin| origin.host_str())
			.map(ToString::to_string);

		if let Some(captcha) = captcha {
			// Will throw an error if the captcha is invalid
			op!([ctx] captcha_verify {
				topic: HashMap::<String, String>::from([
					("kind".into(), "mm:find".into()),
				]),
				remote_address: unwrap_ref!(ctx.remote_address()).to_string(),
				origin_host: origin_host,
				captcha_config: Some(captcha_config.clone()),
				client_response: Some((*captcha).api_try_into()?),
				namespace_id: Some(game_ns.namespace_id.into()),
			})
			.await?;
		} else {
			let required_res = op!([ctx] captcha_request {
				topic: HashMap::<String, String>::from([
					("kind".into(), "mm:find".into()),
				]),
				captcha_config: Some(captcha_config.clone()),
				remote_address: unwrap_ref!(ctx.remote_address()).to_string(),
				namespace_id: Some(game_ns.namespace_id.into()),
			})
			.await?;

			if let Some(hcaptcha_config) = &captcha_config.hcaptcha {
				let hcaptcha_config_res = op!([ctx] captcha_hcaptcha_config_get {
					config: Some(hcaptcha_config.clone()),
				})
				.await?;

				ensure_with!(
					!required_res.needs_verification,
					CAPTCHA_CAPTCHA_REQUIRED {
						metadata: json!({
							"hcaptcha": {
								// Deprecated
								"site_id": hcaptcha_config_res.site_key,
								"site_key": hcaptcha_config_res.site_key,
							}
						}),
					}
				);
			} else if let Some(turnstile_config) = &captcha_config.turnstile {
				let turnstile_config_res = op!([ctx] captcha_turnstile_config_get {
					origin_host: origin_host,
					config: Some(turnstile_config.clone()),
				})
				.await?;

				ensure_with!(
					!required_res.needs_verification,
					CAPTCHA_CAPTCHA_REQUIRED {
						metadata: json!({
							"turnstile": {
								"site_key": turnstile_config_res.site_key,
							}
						}),
					}
				);
			} else {
				bail!("invalid captcha config for version");
			}
		}
	}

	// Create token
	let player_id = Uuid::new_v4();
	let token_res = op!([ctx] token_create {
		issuer: "api-matchmaker".into(),
		token_config: Some(token::create::request::TokenConfig {
			// Has to be greater than the player register time since this
			// token is used in the player disconnect too.
			ttl: util::duration::days(90),
		}),
		refresh_token_config: None,
		client: Some(ctx.client_info()),
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(
					  proto::claims::entitlement::Kind::MatchmakerPlayer(proto::claims::entitlement::MatchmakerPlayer {
						  player_id: Some(player_id.into()),
					  })
				  )
				}
			],
		})),
		label: Some("player".into()),
		..Default::default()
	})
	.await?;
	let token = unwrap_ref!(token_res.token);
	let token_session_id = unwrap_ref!(token_res.session_id).as_uuid();

	// Find lobby
	let query_id = Uuid::new_v4();
	let find_res = msg!([ctx] @notrace mm::msg::lobby_find(game_ns.namespace_id, query_id)
		-> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail>
	{
		namespace_id: Some(game_ns.namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: vec![mm::msg::lobby_find::Player {
			player_id: Some(player_id.into()),
			token_session_id: Some(token_session_id.into()),
			client_info: Some(ctx.client_info()),
		}],
		query: Some(query),
		user_id: user_id.map(Into::into),
		verification_data_json: if let VerificationType::UserData(verification_data) = &verification {
			verification_data
			.as_ref()
			.map(|data| serde_json::to_string(&data))
			.transpose()?
		} else {
			None
		},
		bypass_verification: matches!(verification, VerificationType::Bypass),
		tags: tags.clone(),
		dynamic_max_players: dynamic_max_players
			.map(ApiTryInto::api_try_into)
			.transpose()?,
		debug: None,
	})
	.await?;
	let lobby_id = match find_res {
		Ok(res) => unwrap_ref!(res.lobby_id).as_uuid(),
		Err(err) => {
			use backend::matchmaker::lobby_find::ErrorCode::{self, *};

			let code = unwrap!(
				ErrorCode::from_i32(err.error_code),
				"failed to parse find error code"
			);

			match code {
				Unknown => bail!("unknown find error code"),
				StaleMessage => bail_with!(CHIRP_STALE_MESSAGE),
				TooManyPlayersFromSource => bail_with!(MATCHMAKER_TOO_MANY_PLAYERS_FROM_SOURCE),

				LobbyStopped | LobbyStoppedPrematurely => bail_with!(MATCHMAKER_LOBBY_STOPPED),
				LobbyClosed => bail_with!(MATCHMAKER_LOBBY_CLOSED),
				LobbyNotFound => bail_with!(MATCHMAKER_LOBBY_NOT_FOUND),
				NoAvailableLobbies => bail_with!(MATCHMAKER_NO_AVAILABLE_LOBBIES),
				LobbyFull => bail_with!(MATCHMAKER_LOBBY_FULL),
				LobbyCountOverMax => bail_with!(MATCHMAKER_TOO_MANY_LOBBIES),
				RegionNotEnabled => bail_with!(MATCHMAKER_REGION_NOT_ENABLED_FOR_GAME_MODE),

				DevTeamInvalidStatus => bail_with!(GROUP_DEACTIVATED),

				FindDisabled => bail_with!(MATCHMAKER_FIND_DISABLED),
				JoinDisabled => bail_with!(MATCHMAKER_JOIN_DISABLED),
				VerificationFailed => bail_with!(MATCHMAKER_VERIFICATION_FAILED),
				VerificationRequestFailed => bail_with!(MATCHMAKER_VERIFICATION_REQUEST_FAILED),
				IdentityRequired => bail_with!(MATCHMAKER_IDENTITY_REQUIRED),
				RegistrationRequired => bail_with!(MATCHMAKER_REGISTRATION_REQUIRED),
			};
		}
	};

	// Fetch lobby data
	let lobby_res = op!([ctx] mm_lobby_get {
		lobby_ids: vec![lobby_id.into()],
		..Default::default()
	})
	.await?;
	let lobby = if let Some(lobby) = lobby_res.lobbies.first() {
		lobby
	} else {
		// We should never reach this point, since we preemptively create
		// players in mm-lobby-find which will ensure the lobby is not removed.
		//
		// This will only happen if the lobby manually stops/exits in the middle
		// of a find request.
		tracing::error!("lobby not found in race condition");
		bail!("lobby not found");
	};
	let region_id = unwrap_ref!(lobby.region_id);
	let lobby_group_id = unwrap_ref!(lobby.lobby_group_id);
	let run_id = unwrap_ref!(lobby.run_id);

	// Fetch lobby run data
	let (run_res, version) = tokio::try_join!(
		// Fetch the job run
		async {
			op!([ctx] job_run_get {
				run_ids: vec![*run_id],
			})
			.await
			.map_err(Into::<GlobalError>::into)
		},
		// Fetch the version
		async {
			let version_res = op!([ctx] mm_config_lobby_group_resolve_version {
				lobby_group_ids: vec![*lobby_group_id],
			})
			.await?;

			// NOTE: The matchmaker config is fetched again to account for outdated lobbies
			let version_id = unwrap!(version_res.versions.first());
			let version_id = unwrap_ref!(version_id.version_id);
			let version_res = op!([ctx] mm_config_version_get {
				version_ids: vec![*version_id],
			})
			.await?;
			let version = unwrap!(version_res.versions.first());

			GlobalResult::Ok(version.clone())
		}
	)?;

	// Match the version
	let version_config = unwrap_ref!(version.config);
	let version_meta = unwrap_ref!(version.config_meta);
	let (lobby_group_config, _lobby_group_meta) = unwrap!(version_config
		.lobby_groups
		.iter()
		.zip(version_meta.lobby_groups.iter())
		.find(|(_, meta)| meta.lobby_group_id.as_ref() == Some(lobby_group_id)));
	let lobby_runtime = unwrap_ref!(lobby_group_config.runtime);
	#[allow(clippy::infallible_destructuring_match)]
	let docker_runtime = match unwrap_ref!(lobby_runtime.runtime) {
		backend::matchmaker::lobby_runtime::Runtime::Docker(x) => x,
	};

	// Convert the ports to client-friendly ports
	let run = unwrap!(run_res.runs.first());
	let ports = docker_runtime
		.ports
		.iter()
		.map(|port| build_port(run, port))
		.filter_map(|x| x.transpose())
		.collect::<GlobalResult<HashMap<_, _>>>()?;

	// Fetch region data
	let region_res = op!([ctx] region_get {
		region_ids: vec![*region_id],
	})
	.await?;
	let region_proto = unwrap!(region_res.regions.first());
	let region = Box::new(models::MatchmakerJoinRegion {
		region_id: region_proto.name_id.clone(),
		display_name: region_proto.region_display_name.clone(),
	});

	let player = Box::new(models::MatchmakerJoinPlayer {
		token: token.token.clone(),
	});

	// TODO: Gracefully catch errors from this

	// Also see svc/api-identity/src/route/events.rs for fetching the lobby
	Ok(FindResponse {
		lobby: Box::new(models::MatchmakerJoinLobby {
			lobby_id,
			region,
			ports: ports.clone(),
			player: player.clone(),
		}),
		ports,
		player,
	})
}

async fn resolve_region_ids(
	ctx: &Ctx<Auth>,
	coords: Option<(f64, f64)>,
	regions: Option<&Vec<String>>,
	game_id: &common::Uuid,
	lobby_groups: &[(
		&backend::matchmaker::LobbyGroup,
		&backend::matchmaker::LobbyGroupMeta,
	)],
) -> GlobalResult<Vec<Uuid>> {
	let region_ids = if let Some(region_name_ids) = regions {
		// Resolve the region ID corresponding to the name IDs
		let resolve_res = op!([ctx] region_resolve_for_game {
			game_id: Some(*game_id),
			name_ids: region_name_ids.clone(),
		})
		.await?;

		// Map to region IDs and decide
		let region_ids = resolve_res
			.regions
			.iter()
			.map(|r| Ok(unwrap_ref!(r.region_id).as_uuid()))
			.collect::<GlobalResult<Vec<_>>>()?;

		ensure_eq_with!(
			region_ids.len(),
			region_name_ids.len(),
			MATCHMAKER_REGION_NOT_FOUND
		);

		// Order of regions is not preserved. Furthermore, this list will be used as a filter instead of a
		// priority list.
		region_ids
	} else {
		// Find all enabled region IDs in all requested lobby groups
		let mut enabled_region_ids = lobby_groups
			.iter()
			.flat_map(|(lg, _)| {
				lg.regions
					.iter()
					.filter_map(|r| r.region_id.as_ref())
					.map(common::Uuid::as_uuid)
					.collect::<Vec<_>>()
			})
			.collect::<Vec<Uuid>>();
		enabled_region_ids.sort();
		let enabled_region_ids = enabled_region_ids
			.into_iter()
			.map(Into::<common::Uuid>::into)
			.collect::<Vec<_>>();

		// Auto-select the closest region
		if let Some((lat, long)) = coords {
			let recommend_res = op!([ctx] region_recommend {
				region_ids: enabled_region_ids,
				coords: Some(backend::net::Coordinates {
					latitude: lat,
					longitude: long,
				}),
				..Default::default()
			})
			.await?;
			let primary_region = unwrap!(recommend_res.regions.first());
			let primary_region_id = unwrap_ref!(primary_region.region_id).as_uuid();

			vec![primary_region_id]
		} else {
			tracing::warn!("coords not provided to select region");
			let region_id = unwrap!(enabled_region_ids.first()).as_uuid();

			vec![region_id]
		}
	};

	Ok(region_ids)
}

#[tracing::instrument(err, skip(ctx))]
async fn dev_mock_lobby(
	ctx: &Ctx<Auth>,
	ns_dev_ent: &rivet_claims::ent::GameNamespaceDevelopment,
) -> GlobalResult<FindResponse> {
	// Issue development player
	let player_id = Uuid::new_v4();
	let token = op!([ctx] mm_dev_player_token_create {
		namespace_id: Some(ns_dev_ent.namespace_id.into()),
		player_id: Some(player_id.into()),
	})
	.await?;

	// Find the port to connect to
	let ports = ns_dev_ent
		.lobby_ports
		.iter()
		.map(|port| {
			GlobalResult::Ok((
				port.label.clone(),
				models::MatchmakerJoinPort {
					host: port
						.target_port
						.map(|port| format!("{}:{port}", ns_dev_ent.hostname)),
					hostname: ns_dev_ent.hostname.clone(),
					port: port.target_port.map(|x| x.api_try_into()).transpose()?,
					port_range: port
						.port_range
						.as_ref()
						.map(|x| {
							GlobalResult::Ok(models::MatchmakerJoinPortRange {
								min: x.min.api_try_into()?,
								max: x.max.api_try_into()?,
							})
						})
						.transpose()?
						.map(Box::new),
					is_tls: matches!(
						port.proxy_protocol,
						rivet_claims::ent::DevelopmentProxyProtocol::Https
							| rivet_claims::ent::DevelopmentProxyProtocol::TcpTls
					),
				},
			))
		})
		.collect::<GlobalResult<HashMap<_, _>>>()?;

	let player = Box::new(models::MatchmakerJoinPlayer {
		token: token.player_jwt,
	});

	Ok(FindResponse {
		lobby: Box::new(models::MatchmakerJoinLobby {
			lobby_id: Uuid::nil(),
			region: Box::new(models::MatchmakerJoinRegion {
				region_id: util_mm::consts::DEV_REGION_ID.into(),
				display_name: util_mm::consts::DEV_REGION_NAME.into(),
			}),
			ports: ports.clone(),
			player: player.clone(),
		}),
		ports,
		player,
	})
}

async fn dev_mock_lobby_list(
	ctx: &Ctx<Auth>,
	ns_dev_ent: &rivet_claims::ent::GameNamespaceDevelopment,
	include_state: bool,
) -> GlobalResult<models::MatchmakerListLobbiesResponse> {
	// Read the version config
	let ns_res = op!([ctx] game_namespace_get {
		namespace_ids: vec![ns_dev_ent.namespace_id.into()],
	})
	.await?;
	let ns_data = unwrap!(ns_res.namespaces.first());
	let version_id = unwrap_ref!(ns_data.version_id).as_uuid();

	let version_res = op!([ctx] mm_config_version_get {
		version_ids: vec![version_id.into()],
	})
	.await?;
	let version = unwrap!(
		version_res.versions.first(),
		"no matchmaker config for namespace"
	);
	let version_config = unwrap_ref!(version.config);

	// Create fake region
	let region = models::MatchmakerRegionInfo {
		region_id: util_mm::consts::DEV_REGION_ID.into(),
		provider_display_name: util_mm::consts::DEV_PROVIDER_NAME.into(),
		region_display_name: util_mm::consts::DEV_REGION_NAME.into(),
		datacenter_coord: Box::new(models::GeoCoord {
			latitude: 0.0,
			longitude: 0.0,
		}),
		datacenter_distance_from_client: Box::new(models::GeoDistance {
			kilometers: 0.0,
			miles: 0.0,
		}),
	};

	// List game modes
	let game_modes = version_config
		.lobby_groups
		.iter()
		.map(|lg| models::MatchmakerGameModeInfo {
			game_mode_id: lg.name_id.clone(),
		})
		.collect();

	// Create a fake lobby in each game mode
	let lobbies = version_config
		.lobby_groups
		.iter()
		.map(|lg| {
			GlobalResult::Ok(models::MatchmakerLobbyInfo {
				region_id: util_mm::consts::DEV_REGION_ID.into(),
				game_mode_id: lg.name_id.clone(),
				lobby_id: Uuid::nil(),
				max_players_normal: std::convert::TryInto::try_into(lg.max_players_normal)?,
				max_players_direct: std::convert::TryInto::try_into(lg.max_players_direct)?,
				max_players_party: std::convert::TryInto::try_into(lg.max_players_party)?,
				total_player_count: 0,
				state: include_state.then(|| Some(json!({ "foo": "bar" }))),
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::MatchmakerListLobbiesResponse {
		regions: vec![region],
		game_modes,
		lobbies,
	})
}

// TODO: Copied to api-identity
fn build_port(
	run: &backend::job::Run,
	port: &backend::matchmaker::lobby_runtime::Port,
) -> GlobalResult<Option<(String, models::MatchmakerJoinPort)>> {
	use backend::{
		job::ProxyProtocol as JobProxyProtocol,
		matchmaker::lobby_runtime::{ProxyKind as MmProxyKind, ProxyProtocol as MmProxyProtocol},
	};

	let proxy_kind = unwrap!(MmProxyKind::from_i32(port.proxy_kind));
	let mm_proxy_protocol = unwrap!(MmProxyProtocol::from_i32(port.proxy_protocol));

	let join_info_port = match (proxy_kind, mm_proxy_protocol) {
		(
			MmProxyKind::GameGuard,
			MmProxyProtocol::Http
			| MmProxyProtocol::Https
			| MmProxyProtocol::Tcp
			| MmProxyProtocol::TcpTls
			| MmProxyProtocol::Udp,
		) => {
			run.proxied_ports
				.iter()
				// Decode the proxy protocol
				.filter_map(|proxied_port| {
					match JobProxyProtocol::from_i32(proxied_port.proxy_protocol) {
						Some(x) => Some((proxied_port, x)),
						None => {
							tracing::error!(?proxied_port, "could not decode job proxy protocol");
							None
						}
					}
				})
				// Match the matchmaker port with the job port that matches the same
				// port and protocol
				.filter(|(proxied_port, job_proxy_protocol)| {
					test_mm_and_job_proxy_protocol_eq(mm_proxy_protocol, *job_proxy_protocol)
						&& proxied_port.target_nomad_port_label
							== Some(util_mm::format_nomad_port_label(&port.label))
				})
				// Extract the port's host. This should never be `None`.
				.filter_map(|(proxied_port, _)| {
					proxied_port
						.ingress_hostnames
						.iter()
						// NOTE: Selects the primary ingress hostname (has no path segments)
						.find(|hostname| !hostname.contains('/'))
						.map(|hostname| (proxied_port, hostname))
				})
				.map(|(proxied_port, hostname)| {
					GlobalResult::Ok(models::MatchmakerJoinPort {
						host: Some(format!("{}:{}", hostname, proxied_port.ingress_port)),
						hostname: hostname.clone(),
						port: Some(proxied_port.ingress_port.api_try_into()?),
						port_range: None,
						is_tls: matches!(
							mm_proxy_protocol,
							MmProxyProtocol::Https | MmProxyProtocol::TcpTls
						),
					})
				})
				.next()
				.transpose()?
		}
		(MmProxyKind::None, MmProxyProtocol::Tcp | MmProxyProtocol::Udp) => {
			let port_range = unwrap_ref!(port.port_range);

			let run_meta = unwrap_ref!(run.run_meta);
			let Some(backend::job::run_meta::Kind::Nomad(run_meta_nomad)) = &run_meta.kind else {
				bail!("invalid nomad run meta kind")
			};
			let node_public_ipv4 = unwrap_ref!(run_meta_nomad.node_public_ipv4);

			Some(models::MatchmakerJoinPort {
				host: None,
				hostname: node_public_ipv4.clone(),
				port: None,
				port_range: Some(Box::new(models::MatchmakerJoinPortRange {
					min: port_range.min.api_try_into()?,
					max: port_range.max.api_try_into()?,
				})),
				is_tls: false,
			})
		}
		(
			MmProxyKind::None,
			MmProxyProtocol::Http | MmProxyProtocol::Https | MmProxyProtocol::TcpTls,
		) => {
			bail!("invalid http proxy protocol with host network")
		}
	};

	GlobalResult::Ok(join_info_port.map(|x| (port.label.clone(), x)))
}

fn test_mm_and_job_proxy_protocol_eq(
	mm_proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol,
	job_proxy_protocol: backend::job::ProxyProtocol,
) -> bool {
	use backend::{job::ProxyProtocol as JPP, matchmaker::lobby_runtime::ProxyProtocol as MPP};

	match (mm_proxy_protocol, job_proxy_protocol) {
		(MPP::Http, JPP::Http) => true,
		(MPP::Https, JPP::Https) => true,
		(MPP::Tcp, JPP::Tcp) => true,
		(MPP::TcpTls, JPP::TcpTls) => true,
		(MPP::Udp, JPP::Udp) => true,
		_ => false,
	}
}
