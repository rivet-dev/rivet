use std::collections::HashMap;

use futures_util::{StreamExt, TryStreamExt};
use http::StatusCode;
use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct ExternalVerificationRequest {
	pub verification_data: Option<serde_json::Value>,
	pub game: Game,
	pub clients: HashMap<String, Client>,
	pub join_kind: JoinKind,
	pub kind: ConnectionKind,
}

#[derive(Serialize)]
pub struct Game {
	pub namespace_id: Uuid,
	pub game_mode_id: Uuid,
	pub game_mode_name_id: String,

	pub lobby: Option<Lobby>,
	pub state: Option<serde_json::Value>,
	pub config: Option<serde_json::Value>,
	pub tags: HashMap<String, String>,
	pub dynamic_max_players: Option<u32>,
}

#[derive(Serialize)]
pub struct Lobby {
	pub lobby_id: Uuid,
	pub region_id: Uuid,
	pub region_name_id: String,
	pub create_ts: String,
	pub is_closed: bool,
}

#[derive(Serialize)]
pub struct Client {
	pub user_agent: Option<String>,
	pub latitude: Option<f64>,
	pub longitude: Option<f64>,
}

#[derive(Serialize)]
pub enum JoinKind {
	Normal,
	Party,
}

#[derive(Copy, Clone, Serialize)]
pub enum ConnectionKind {
	Find,
	Join,
	Create,
}

pub struct VerifyConfigOpts<'a> {
	pub kind: ConnectionKind,
	pub namespace_id: Uuid,
	pub user_id: Option<Uuid>,
	pub client_info: Vec<backend::net::ClientInfo>,
	pub tags: &'a HashMap<String, String>,
	pub dynamic_max_players: Option<u32>,

	pub lobby_groups: &'a [backend::matchmaker::LobbyGroup],
	pub lobby_group_meta: &'a [backend::matchmaker::LobbyGroupMeta],
	pub lobby_info: Option<&'a backend::matchmaker::Lobby>,
	pub lobby_state_json: Option<&'a str>,

	pub verification_data_json: Option<&'a str>,
	pub lobby_config_json: Option<&'a str>,
	pub custom_lobby_publicity: Option<backend::matchmaker::lobby::Publicity>,
}

struct ExternalRequestConfigAndLobby<'a> {
	pub lobby_group: &'a backend::matchmaker::LobbyGroup,
	pub lobby_group_meta: &'a backend::matchmaker::LobbyGroupMeta,
	external_request_config: backend::net::ExternalRequestConfig,
}

/// Verifies everything required to make a find request or create a custom lobby.
pub async fn verify_config(
	ctx: &OperationContext<()>,
	opts: &VerifyConfigOpts<'_>,
) -> GlobalResult<()> {
	let mut highest_identity_requirement = backend::matchmaker::IdentityRequirement::None;
	let mut external_request_configs = Vec::new();

	// Collect all external request configs and identity requirement
	for (lobby_group, lobby_group_meta) in opts.lobby_groups.iter().zip(opts.lobby_group_meta) {
		let (identity_requirement, external_request_config) = match (
			opts.kind,
			lobby_group.actions.as_ref().and_then(|a| a.find.as_ref()),
			lobby_group.actions.as_ref().and_then(|a| a.join.as_ref()),
			lobby_group.actions.as_ref().and_then(|a| a.create.as_ref()),
		) {
			(ConnectionKind::Find, Some(find_config), _, _) => {
				if !find_config.enabled {
					bail_with!(MATCHMAKER_FIND_DISABLED);
				}

				(
					unwrap!(
						backend::matchmaker::IdentityRequirement::from_i32(
							find_config.identity_requirement
						),
						"invalid identity requirement variant"
					),
					find_config.verification.as_ref().map(|config| {
						backend::net::ExternalRequestConfig {
							url: config.url.clone(),
							method: backend::net::HttpMethod::Post as i32,
							headers: config.headers.clone(),
						}
					}),
				)
			}
			(ConnectionKind::Join, _, Some(join_config), _) => {
				if !join_config.enabled {
					bail_with!(MATCHMAKER_JOIN_DISABLED);
				}

				(
					unwrap!(
						backend::matchmaker::IdentityRequirement::from_i32(
							join_config.identity_requirement
						),
						"invalid identity requirement variant"
					),
					join_config.verification.as_ref().map(|config| {
						backend::net::ExternalRequestConfig {
							url: config.url.clone(),
							method: backend::net::HttpMethod::Post as i32,
							headers: config.headers.clone(),
						}
					}),
				)
			}
			(ConnectionKind::Create, _, _, Some(create_config)) => {
				let publicity = unwrap!(opts.custom_lobby_publicity);

				// Verify publicity
				match (
					publicity,
					create_config.enable_public,
					create_config.enable_private,
				) {
					(backend::matchmaker::lobby::Publicity::Public, allowed, _) => {
						ensure_with!(
							allowed,
							MATCHMAKER_CUSTOM_LOBBY_CONFIG_INVALID,
							reason = r#""public" publicity not allowed with this custom game mode"#
						);
					}
					(backend::matchmaker::lobby::Publicity::Private, _, allowed) => {
						ensure_with!(
							allowed,
							MATCHMAKER_CUSTOM_LOBBY_CONFIG_INVALID,
							reason = r#""private" publicity not allowed with this custom game mode"#
						);
					}
					_ => {
						bail_with!(
							MATCHMAKER_CUSTOM_LOBBY_CONFIG_INVALID,
							reason = "given publicity not allowed"
						);
					}
				}

				// Verify lobby count
				if let (Some(max_lobbies_per_identity), Some(user_id)) =
					(create_config.max_lobbies_per_identity, opts.user_id)
				{
					let lobbies_res = op!([ctx] mm_lobby_list_for_user_id {
						user_ids: vec![user_id.into()],
					})
					.await?;
					let user = unwrap!(lobbies_res.users.first());
					ensure_with!(
						(user.lobby_ids.len() as u64) < max_lobbies_per_identity,
						MATCHMAKER_CUSTOM_LOBBY_LIMIT_REACHED
					);
				}

				(
					unwrap!(
						backend::matchmaker::IdentityRequirement::from_i32(
							create_config.identity_requirement
						),
						"invalid identity requirement variant"
					),
					create_config.verification.as_ref().map(|config| {
						backend::net::ExternalRequestConfig {
							url: config.url.clone(),
							method: backend::net::HttpMethod::Post as i32,
							headers: config.headers.clone(),
						}
					}),
				)
			}
			(ConnectionKind::Create, _, _, None) => {
				bail_with!(MATCHMAKER_CUSTOM_LOBBIES_DISABLED);
			}
			_ => (backend::matchmaker::IdentityRequirement::None, None),
		};

		// Updated highest requirement
		match highest_identity_requirement {
			backend::matchmaker::IdentityRequirement::None => {
				highest_identity_requirement = identity_requirement;
			}
			backend::matchmaker::IdentityRequirement::Guest => {
				if matches!(
					identity_requirement,
					backend::matchmaker::IdentityRequirement::Registered
				) {
					highest_identity_requirement = identity_requirement;
				}
			}
			backend::matchmaker::IdentityRequirement::Registered => {}
		}

		if let Some(external_request_config) = external_request_config {
			external_request_configs.push(ExternalRequestConfigAndLobby {
				lobby_group,
				lobby_group_meta,
				external_request_config,
			});
		}
	}

	// Verify identity requirement
	match (highest_identity_requirement, opts.user_id) {
		(backend::matchmaker::IdentityRequirement::Registered, Some(user_id)) => {
			let user_identities_res = op!([ctx] user_identity_get {
				user_ids: vec![user_id.into()],
			})
			.await?;
			let user = unwrap!(
				user_identities_res.users.first(),
				"could not find user identities"
			);
			let is_registered = !user.identities.is_empty();

			if !is_registered {
				bail_with!(MATCHMAKER_REGISTRATION_REQUIRED);
			}
		}
		(
			backend::matchmaker::IdentityRequirement::Guest
			| backend::matchmaker::IdentityRequirement::Registered,
			None,
		) => {
			bail_with!(MATCHMAKER_IDENTITY_REQUIRED);
		}
		_ => {}
	}

	// Verify lobby config
	if let Some(lobby_config_json) = opts.lobby_config_json {
		ensure_with!(
			lobby_config_json.len() as u64 <= util::file_size::kibibytes(16),
			MATCHMAKER_CUSTOM_LOBBY_CONFIG_INVALID,
			reason = "too large (> 16KiB)"
		);
	}

	// Verify user data externally
	for external_request_config_and_lobby in external_request_configs {
		let ExternalRequestConfigAndLobby {
			lobby_group,
			lobby_group_meta,
			external_request_config,
		} = external_request_config_and_lobby;

		// Build lobby info
		let lobby = if let Some(l) = &opts.lobby_info {
			// Fetch region data for readable name
			let region_id = unwrap!(l.region_id);
			let regions_res = op!([ctx] region_get {
				region_ids: vec![region_id],
			})
			.await?;
			let region = unwrap!(regions_res.regions.first());

			Some(Lobby {
				lobby_id: unwrap_ref!(l.lobby_id).as_uuid(),
				region_id: region_id.as_uuid(),
				region_name_id: region.name_id.clone(),
				create_ts: util::timestamp::to_string(l.create_ts)?,
				is_closed: l.is_closed,
			})
		} else {
			None
		};

		// Fetch IP info
		let clients = futures_util::stream::iter(
			opts.client_info
				.iter()
				.filter_map(|client_info| {
					client_info
						.remote_address
						.as_ref()
						.map(|ip| (ip.clone(), client_info.user_agent.clone()))
				})
				.collect::<Vec<_>>(),
		)
		.map(|(ip, user_agent)| async move {
			let ip_res = op!([ctx] ip_info {
				ip: ip.clone(),
			})
			.await?;
			let (latitude, longitude) = ip_res
				.ip_info
				.and_then(|ip_info| ip_info.coords)
				.map(|coords| (coords.latitude, coords.longitude))
				.unzip();

			GlobalResult::Ok((
				ip.clone(),
				Client {
					user_agent: user_agent.clone(),
					longitude,
					latitude,
				},
			))
		})
		.buffer_unordered(16)
		.try_collect::<HashMap<_, _>>()
		.await?;

		// Build body
		let body = ExternalVerificationRequest {
			verification_data: opts
				.verification_data_json
				.as_ref()
				.map(|json| serde_json::from_str::<serde_json::Value>(json))
				.transpose()?,
			game: Game {
				game_mode_id: unwrap_ref!(lobby_group_meta.lobby_group_id).as_uuid(),
				game_mode_name_id: lobby_group.name_id.clone(),
				namespace_id: opts.namespace_id,

				lobby,
				state: opts
					.lobby_state_json
					.as_ref()
					.map(|json| serde_json::from_str::<serde_json::Value>(json))
					.transpose()?,
				config: opts
					.lobby_config_json
					.as_ref()
					.map(|json| serde_json::from_str::<serde_json::Value>(json))
					.transpose()?,
				tags: opts.tags.clone(),
				dynamic_max_players: opts.dynamic_max_players,
			},
			clients,
			join_kind: JoinKind::Normal,
			kind: opts.kind,
		};

		// Send request
		let request_id = Uuid::new_v4();
		let external_res = msg!([ctx] external::msg::request_call(request_id)
		-> Result<external::msg::request_call_complete, external::msg::request_call_fail>
		{
			request_id: Some(request_id.into()),
			config: Some(external_request_config),
			timeout: util::duration::seconds(10) as u64,
			body: Some(serde_json::to_vec(&body)?),
			..Default::default()
		})
		.await?;

		// Handle status code
		if let Ok(res) = external_res {
			let status = StatusCode::from_u16(res.status_code as u16)?;

			tracing::info!(?status, "user verification response");

			if !status.is_success() {
				bail_with!(MATCHMAKER_VERIFICATION_FAILED);
			}
		} else {
			bail_with!(MATCHMAKER_VERIFICATION_REQUEST_FAILED);
		}
	}

	Ok(())
}
