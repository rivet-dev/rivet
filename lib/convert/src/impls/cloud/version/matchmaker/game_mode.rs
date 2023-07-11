use std::collections::HashMap;

use proto::backend::{self, pkg::*};
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{ApiInto, ApiTryFrom, ApiTryInto};

pub fn game_mode_to_proto(
	name_id: String,
	game_mode: &models::CloudVersionMatchmakerGameMode,
	matchmaker: &models::CloudVersionMatchmakerConfig,
	regions_data: &[region::resolve::response::Region],
) -> GlobalResult<backend::matchmaker::LobbyGroup> {
	// Derive regions
	let regions = game_mode
		.regions
		.clone()
		.or_else(|| matchmaker.regions.clone())
		.unwrap_or_default();

	// Derive max players
	let max_players_normal = game_mode
		.max_players
		.or(matchmaker.max_players)
		.unwrap_or(util_mm::defaults::MAX_PLAYERS_NORMAL as i32);
	let max_players_direct = game_mode
		.max_players_direct
		.or(matchmaker.max_players_direct)
		.unwrap_or(max_players_normal);
	let max_players_party = game_mode
		.max_players_party
		.or(matchmaker.max_players_party)
		.unwrap_or(max_players_normal);

	// TODO: Make this return a 400 error instead
	internal_assert!(max_players_normal >= 0);
	internal_assert!(max_players_direct >= 0);
	internal_assert!(max_players_party >= 0);

	// Derive runtime
	let runtime = if let Some(either_runtime) = game_mode.docker.as_ref().or(matchmaker.docker.as_ref()) {
		let args = either_runtime.args.clone().unwrap_or_default();

		let mut env_vars = HashMap::<String, String>::new();
		if let Some(env) = matchmaker.docker.as_ref().and_then(|x| x.env.clone()) {
			env_vars.extend(env);
		}
		if let Some(env) = game_mode.docker.as_ref().and_then(|x| x.env.clone()) {
			env_vars.extend(env);
		}

		let network_mode = either_runtime.network_mode
			.unwrap_or(models::CloudVersionMatchmakerNetworkMode::Bridge);

		let ports = either_runtime.ports.clone().unwrap_or_default();

		Some(backend::matchmaker::LobbyRuntime {
			runtime: Some(backend::matchmaker::lobby_runtime::Runtime::Docker(
				backend::matchmaker::lobby_runtime::Docker {
					build_id: either_runtime.image_id.map(Into::into),
					args,
					env_vars: env_vars
						.into_iter()
						.map(|(key, value)| backend::matchmaker::lobby_runtime::EnvVar {
							key,
							value,
						})
						.collect(),
					network_mode:
						ApiInto::<backend::matchmaker::lobby_runtime::NetworkMode>::api_into(
							network_mode,
						) as i32,
					ports: ports
						.into_iter()
						.map(|(label, value)| {
							let proxy_protocol = value
								.protocol
								.unwrap_or(models::CloudVersionMatchmakerPortProtocol::Https);
							let proxy_kind = value
								.proxy
								.unwrap_or(models::CloudVersionMatchmakerProxyKind::GameGuard);

							GlobalResult::Ok(backend::matchmaker::lobby_runtime::Port {
								label,
								target_port: value.port.map(|x| x as u32),
								port_range: value.port_range.map(|x| (*x).api_into()),
								proxy_protocol: ApiInto::<
									backend::matchmaker::lobby_runtime::ProxyProtocol,
								>::api_into(proxy_protocol) as i32,
								proxy_kind: ApiInto::<
									backend::matchmaker::lobby_runtime::ProxyKind,
								>::api_into(proxy_kind) as i32,
							})
						})
						.collect::<GlobalResult<_>>()?,
				},
			)),
		})
	} else {
		None
	};

	Ok(backend::matchmaker::LobbyGroup {
		name_id,

		regions: regions
			.iter()
			.map(|(k, v)| region_to_proto(k.clone(), v, game_mode, matchmaker, regions_data))
			.collect::<GlobalResult<_>>()?,
		max_players_normal: max_players_normal as u32,
		max_players_direct: max_players_direct as u32,
		max_players_party: max_players_party as u32,

		runtime,

		find_config: game_mode.find_config.map(ApiTryInto::try_into).transpose()?,
		join_config: game_mode.join_config.map(ApiTryInto::try_into).transpose()?,
})
}

pub fn game_mode_to_openapi(
	value: backend::matchmaker::LobbyGroup,
	regions_data: &[backend::region::Region],
) -> GlobalResult<(String, models::CloudVersionMatchmakerGameMode)> {
	let docker = match internal_unwrap_owned!(
		value.runtime.as_ref().and_then(|x| x.runtime.as_ref()),
		"unknown runtime"
	) {
		backend::matchmaker::lobby_runtime::Runtime::Docker(runtime) => {
			models::CloudVersionMatchmakerGameModeRuntimeDocker {
				image_id: Some(internal_unwrap!(runtime.build_id).as_uuid()),
				args: Some(runtime.args.clone()),
				env: Some(
					runtime
						.env_vars
						.iter()
						.cloned()
						.map(|x| (x.key, x.value))
						.collect(),
				),
				network_mode: Some(
					internal_unwrap_owned!(
						backend::matchmaker::lobby_runtime::NetworkMode::from_i32(
							runtime.network_mode,
						)
					)
					.api_into(),
				),
				ports: Some(
					runtime
						.ports
						.iter()
						.map(|x| {
							GlobalResult::Ok((
								x.label.clone(),
								models::CloudVersionMatchmakerGameModeRuntimeDockerPort {
									port: x.target_port.map(|x| x as i32),
									port_range: x
										.port_range
										.clone()
										.map(ApiInto::api_into)
										.map(Box::new),
									protocol: Some(
										internal_unwrap_owned!(
										backend::matchmaker::lobby_runtime::ProxyProtocol::from_i32(
											x.proxy_protocol
										)
									)
										.api_into(),
									),
									proxy: Some(
										internal_unwrap_owned!(
											backend::matchmaker::lobby_runtime::ProxyKind::from_i32(
												x.proxy_kind
											)
										)
										.api_into(),
									),

									// Client-side configuration
									dev_port: None,
									dev_port_range: None,
									dev_protocol: None,
								},
							))
						})
						.collect::<Result<HashMap<_, _>, _>>()?,
				),

				// Client-side helper
				dockerfile: None,
				image: None,
			}
		}
	};

	Ok((
		value.name_id.clone(),
		models::CloudVersionMatchmakerGameMode {
			regions: Some(
				value
					.regions
					.into_iter()
					.map(|x| region_to_openapi(x, regions_data))
					.collect::<GlobalResult<HashMap<_, _>>>()?,
			),
			max_players: Some(value.max_players_normal as i32),
			max_players_direct: Some(value.max_players_direct as i32),
			max_players_party: Some(value.max_players_party as i32),

			docker: Some(Box::new(docker)),

			// Overrides
			idle_lobbies: None,
			tier: None,
		},
	))
}

fn region_to_proto(
	name_id: String,
	region: &models::CloudVersionMatchmakerGameModeRegion,
	game_mode: &models::CloudVersionMatchmakerGameMode,
	matchmaker: &models::CloudVersionMatchmakerConfig,
	regions_data: &[region::resolve::response::Region],
) -> GlobalResult<backend::matchmaker::lobby_group::Region> {
	let region_id = regions_data
		.iter()
		.find(|x| x.name_id == name_id)
		.and_then(|r| r.region_id);

	let tier_name_id = region
		.tier
		.clone()
		.or_else(|| game_mode.tier.clone())
		.or_else(|| matchmaker.tier.clone())
		.unwrap_or_else(|| util_mm::defaults::TIER_NAME_ID.to_string());

	let idle_lobbies = region
		.idle_lobbies
		.clone()
		.or_else(|| game_mode.idle_lobbies.clone())
		.or_else(|| matchmaker.idle_lobbies.clone())
		.unwrap_or_else(|| {
			Box::new(models::CloudVersionMatchmakerGameModeIdleLobbiesConfig {
				min: util_mm::defaults::IDLE_LOBBIES_MIN as i32,
				max: util_mm::defaults::IDLE_LOBBIES_MAX as i32,
			})
		});

	Ok(backend::matchmaker::lobby_group::Region {
		region_id,
		tier_name_id,
		idle_lobbies: Some(backend::matchmaker::lobby_group::IdleLobbies {
			min_idle_lobbies: idle_lobbies.min as u32,
			max_idle_lobbies: idle_lobbies.max as u32,
		}),
	})
}

fn region_to_openapi(
	region: backend::matchmaker::lobby_group::Region,
	regions_data: &[backend::region::Region],
) -> GlobalResult<(String, models::CloudVersionMatchmakerGameModeRegion)> {
	let region_data = internal_unwrap_owned!(
		regions_data
			.iter()
			.find(|x| x.region_id == region.region_id),
		"failed to find region matching name id"
	);

	Ok((
		region_data.name_id.clone(),
		models::CloudVersionMatchmakerGameModeRegion {
			tier: Some(region.tier_name_id.to_owned()),
			idle_lobbies: region
				.idle_lobbies
				.map(ApiTryInto::try_into)
				.transpose()?
				.map(Box::new),
		},
	))
}

impl ApiTryFrom<models::CloudVersionMatchmakerGameModeIdleLobbiesConfig>
	for backend::matchmaker::lobby_group::IdleLobbies
{
	type Error = GlobalError;

	fn try_from(
		value: models::CloudVersionMatchmakerGameModeIdleLobbiesConfig,
	) -> GlobalResult<Self> {
		internal_assert!(value.min >= 0);
		internal_assert!(value.max >= 0);

		Ok(backend::matchmaker::lobby_group::IdleLobbies {
			min_idle_lobbies: value.min as u32,
			max_idle_lobbies: value.max as u32,
		})
	}
}

impl ApiTryFrom<backend::matchmaker::lobby_group::IdleLobbies>
	for models::CloudVersionMatchmakerGameModeIdleLobbiesConfig
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::lobby_group::IdleLobbies) -> GlobalResult<Self> {
		Ok(models::CloudVersionMatchmakerGameModeIdleLobbiesConfig {
			min: value.min_idle_lobbies as i32,
			max: value.max_idle_lobbies as i32,
		})
	}
}

// TODO:
impl ApiTryFrom<models::CloudVersionMatchmakerLobbyGroupFindConfig>
	for backend::matchmaker::FindConfig
{
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionMatchmakerLobbyGroupFindConfig) -> GlobalResult<Self> {
		Ok(backend::matchmaker::FindConfig {
			
		})
	}
}

impl ApiTryFrom<backend::matchmaker::FindConfig>
	for models::CloudVersionMatchmakerLobbyGroupFindConfig
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::FindConfig) -> GlobalResult<Self> {
		Ok(models::CloudVersionMatchmakerLobbyGroupRuntime {
		
		})
	}
}