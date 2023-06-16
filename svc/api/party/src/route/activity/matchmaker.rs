use std::collections::{HashMap, HashSet};

use api_helper::ctx::Ctx;
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;
use rivet_party_server::models;
use serde_json::json;

use crate::{
	auth::Auth,
	fetch::game::{fetch_ns, NamespaceData},
	utils,
};

// MARK: POST /parties/self/activity/matchmaker/lobbies/join
pub async fn join_lobby(
	ctx: Ctx<Auth>,
	body: models::JoinMatchmakerLobbyForPartyRequest,
) -> GlobalResult<models::JoinMatchmakerLobbyForPartyResponse> {
	let (user_id, game_user) = ctx.auth().fetch_game_user(ctx.op_ctx()).await?;
	let namespace_id = internal_unwrap!(game_user.namespace_id).as_uuid();

	let lobby_id = util::uuid::parse(&body.lobby_id)?;

	let party_id = unwrap_with_owned!(
		utils::get_current_party(ctx.op_ctx(), user_id).await?,
		PARTY_IDENTITY_NOT_IN_ANY_PARTY
	);

	utils::assert_party_leader(ctx.op_ctx(), party_id, user_id).await?;
	let ns_data = fetch_ns(&ctx, namespace_id).await?;

	let find_query = party::msg::state_mm_lobby_find::message::Query::Direct(
		backend::matchmaker::query::Direct {
			lobby_id: Some(lobby_id.into()),
		},
	);
	find_inner(&ctx, party_id, &ns_data, find_query, body.captcha).await?;

	Ok(models::JoinMatchmakerLobbyForPartyResponse {})
}

// MARK: POST /parties/self/activity/matchmaker/lobbies/find
pub async fn find_lobby(
	ctx: Ctx<Auth>,
	body: models::FindMatchmakerLobbyForPartyRequest,
) -> GlobalResult<models::FindMatchmakerLobbyForPartyResponse> {
	let (lat, long) = internal_unwrap_owned!(ctx.coords());

	let (user_id, game_user) = ctx.auth().fetch_game_user(ctx.op_ctx()).await?;
	let namespace_id = internal_unwrap!(game_user.namespace_id).as_uuid();

	let party_id = unwrap_with_owned!(
		utils::get_current_party(ctx.op_ctx(), user_id).await?,
		PARTY_IDENTITY_NOT_IN_ANY_PARTY
	);

	utils::assert_party_leader(ctx.op_ctx(), party_id, user_id).await?;

	let ns_data = fetch_ns(&ctx, namespace_id).await?;

	// TODO: This is copy-pasta from api-matchmaker
	// Fetch version data
	let version_res = op!([ctx] mm_config_version_get {
		version_ids: vec![ns_data.version_id.into()],
	})
	.await?;
	let version_data = internal_unwrap_owned!(version_res.versions.first());
	let version_config = internal_unwrap!(version_data.config);
	let version_meta = internal_unwrap!(version_data.config_meta);

	// Find lobby groups that match the requested game modes. This matches the
	// same order as `body.game_modes`.
	let lobby_groups: Vec<(
		&backend::matchmaker::LobbyGroup,
		&backend::matchmaker::LobbyGroupMeta,
	)> = body
		.game_modes
		.iter()
		.map(|name_id| {
			Ok(unwrap_with_owned!(
				version_config
					.lobby_groups
					.iter()
					.zip(version_meta.lobby_groups.iter())
					.find(|(lgc, _)| lgc.name_id == *name_id),
				MATCHMAKER_GAME_MODE_NOT_FOUND
			))
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	// Resolve the region IDs.
	//
	// `region_ids` represents the requested regions in order of priority.
	let region_ids = if let Some(region_name_ids) = body.regions {
		// Resolve the region ID corresponding to the name IDs
		let resolve_res = op!([ctx] region_resolve {
			name_ids: region_name_ids.clone(),
		})
		.await?;

		// Map to region IDs and decide
		let region_ids = region_name_ids
			.iter()
			.flat_map(|name_id| resolve_res.regions.iter().find(|r| r.name_id == *name_id))
			.flat_map(|r| r.region_id.as_ref())
			.map(common::Uuid::as_uuid)
			.collect::<Vec<_>>();

		internal_assert_eq!(region_ids.len(), region_name_ids.len(), "region not found");

		region_ids
	} else {
		// Find all enabled region IDs in all requested lobby groups
		let enabled_region_ids = lobby_groups
			.iter()
			.flat_map(|(lg, _)| {
				lg.regions
					.iter()
					.filter_map(|r| r.region_id.as_ref())
					.map(common::Uuid::as_uuid)
					.collect::<Vec<_>>()
			})
			.collect::<HashSet<Uuid>>()
			.into_iter()
			.map(Into::<common::Uuid>::into)
			.collect::<Vec<_>>();

		// Auto-select the closest region
		let recommend_res = op!([ctx] region_recommend {
			latitude: Some(lat),
			longitude: Some(long),
			region_ids: enabled_region_ids,
			..Default::default()
		})
		.await?;
		let primary_region = internal_unwrap_owned!(recommend_res.regions.first());
		let primary_region_id = internal_unwrap!(primary_region.region_id).as_uuid();

		vec![primary_region_id]
	};

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
		internal_panic!("no valid lobby group and region id pair found for auto-create");
	};

	let find_query = party::msg::state_mm_lobby_find::message::Query::LobbyGroup(
		backend::matchmaker::query::LobbyGroup {
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
		},
	);
	find_inner(&ctx, party_id, &ns_data, find_query, body.captcha).await?;

	Ok(models::FindMatchmakerLobbyForPartyResponse {})
}

// MARK: POST /parties/self/members/self/matchmaker/ready
pub async fn ready(
	ctx: Ctx<Auth>,
	_body: models::MatchmakerSelfReadyRequest,
) -> GlobalResult<models::MatchmakerSelfReadyResponse> {
	let (user_id, game_user) = ctx.auth().fetch_game_user(ctx.op_ctx()).await?;
	let game_user_namespace_id = internal_unwrap!(game_user.namespace_id).as_uuid();

	let party_id = unwrap_with_owned!(
		utils::get_current_party(ctx.op_ctx(), user_id).await?,
		PARTY_IDENTITY_NOT_IN_ANY_PARTY
	);

	// Fetch party
	let party_res = op!([ctx] party_get {
		party_ids: vec![party_id.into()],
	})
	.await?;
	let party = unwrap_with_owned!(party_res.parties.first(), PARTY_IDENTITY_NOT_IN_ANY_PARTY);

	let party_namespace_id = match &party.state {
		Some(
			backend::party::party::State::MatchmakerFindingLobby(
				backend::party::party::StateMatchmakerFindingLobby { namespace_id, .. },
			)
			| backend::party::party::State::MatchmakerLobby(
				backend::party::party::StateMatchmakerLobby { namespace_id, .. },
			),
		) => internal_unwrap!(namespace_id).as_uuid(),
		_ => {
			panic_with!(PARTY_PARTY_NOT_IN_GAME)
		}
	};

	// Validate the party is in the same namespace as the game user
	assert_eq_with!(
		game_user_namespace_id,
		party_namespace_id,
		PARTY_PARTY_NOT_IN_GAME
	);

	msg!([ctx] party::msg::member_state_set_mm_pending(party_id, user_id) {
		party_id: Some(party_id.into()),
		user_id: Some(user_id.into()),
	})
	.await?;

	Ok(models::MatchmakerSelfReadyResponse {})
}

async fn find_inner(
	ctx: &Ctx<Auth>,
	party_id: Uuid,
	game_ns: &NamespaceData,
	query: party::msg::state_mm_lobby_find::message::Query,
	captcha: Option<models::CaptchaConfig>,
) -> GlobalResult<()> {
	// Get version config
	let version_config_res = op!([ctx] mm_config_version_get {
		version_ids: vec![game_ns.version_id.into()],
	})
	.await?;

	let version_config = internal_unwrap_owned!(version_config_res.versions.first());
	let version_config = internal_unwrap!(version_config.config);

	// Validate captcha
	if let Some(captcha_config) = &version_config.captcha {
		if let Some(captcha) = captcha {
			match captcha {
				models::CaptchaConfig::Hcaptcha(_) => {
					// Will throw an error if the captcha is invalid
					op!([ctx] captcha_verify {
						topic: HashMap::<String, String>::from([
							("kind".into(), "mm:find".into()),
							("namespace_id".into(), game_ns.namespace_id.to_string()),
						]),
						remote_address: internal_unwrap!(ctx.remote_address()).to_string(),
						origin_host: ctx
							.origin()
							.and_then(|origin| origin.host_str())
							.map(ToString::to_string),
						captcha_config: Some(captcha_config.clone()),
						client_response: Some(captcha.try_into()?),
					})
					.await?;
				}
				_ => panic_with!(CAPTCHA_CAPTCHA_INVALID),
			}
		} else {
			let required_res = op!([ctx] captcha_request {
				topic: HashMap::<String, String>::from([
					("kind".into(), "mm:find".into()),
					("namespace_id".into(), game_ns.namespace_id.to_string()),
				]),
				captcha_config: Some(captcha_config.clone()),
				remote_address: internal_unwrap!(ctx.remote_address()).to_string(),
			})
			.await?;

			let hcaptcha_config = internal_unwrap!(captcha_config.hcaptcha);
			let hcaptcha_config_res = op!([ctx] captcha_hcaptcha_config_get {
				config: Some(hcaptcha_config.clone()),
			})
			.await?;

			assert_with!(
				!required_res.needs_verification,
				CAPTCHA_CAPTCHA_REQUIRED {
					metadata: json!({
						"hcaptcha": {
							"site_id": hcaptcha_config_res.site_key,
						}
					}),
				}
			);
		}
	}

	msg!([ctx] party::msg::state_mm_lobby_find(party_id) {
		party_id: Some(party_id.into()),
		namespace_id: Some(game_ns.namespace_id.into()),
		query: Some(query),
	})
	.await?;

	Ok(())
}
