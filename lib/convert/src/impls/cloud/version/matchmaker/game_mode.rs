use std::collections::HashMap;

use proto::backend::{self, pkg::*};
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{ApiFrom, ApiInto, ApiTryFrom, ApiTryInto};

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
		.unwrap_or_else(|| util_mm::defaults::MAX_PLAYERS_NORMAL as i32);
	let max_players_direct = game_mode
		.max_players_direct
		.or(matchmaker.max_players_direct)
		.unwrap_or(max_players_normal);
	let max_players_party = game_mode
		.max_players_party
		.or(matchmaker.max_players_party)
		.unwrap_or(max_players_normal);

	// TODO: Make this return a 400 error instead
	ensure_with!(
		max_players_normal >= 0,
		MATCHMAKER_INVALID_VERSION_CONFIG,
		error = "`max_players` out of bounds"
	);
	ensure_with!(
		max_players_direct >= 0,
		MATCHMAKER_INVALID_VERSION_CONFIG,
		error = "`max_players_direct` out of bounds"
	);
	ensure_with!(
		max_players_party >= 0,
		MATCHMAKER_INVALID_VERSION_CONFIG,
		error = "`max_players_party` out of bounds"
	);

	// Derive runtime
	let runtime =
		if let Some(either_runtime) = game_mode.docker.as_ref().or(matchmaker.docker.as_ref()) {
			let args = either_runtime.args.clone().unwrap_or_default();

			let mut env_vars = HashMap::<String, String>::new();
			if let Some(env) = matchmaker.docker.as_ref().and_then(|x| x.env.clone()) {
				env_vars.extend(env);
			}
			if let Some(env) = game_mode.docker.as_ref().and_then(|x| x.env.clone()) {
				env_vars.extend(env);
			}

			let network_mode = either_runtime
				.network_mode
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
									target_port: value
										.port
										.map(|x| {
											ensure_with!(
												x >= 0,
												MATCHMAKER_INVALID_VERSION_CONFIG,
												error = "`port` out of bounds"
											);

											Ok(x.try_into()?)
										})
										.transpose()?,
									port_range: value
										.port_range
										.map(|x| (*x).try_into())
										.transpose()?,
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
		max_players_normal: max_players_normal.try_into()?,
		max_players_direct: max_players_direct.try_into()?,
		max_players_party: max_players_party.try_into()?,
		listable: game_mode.listable.unwrap_or(true),
		taggable: game_mode.taggable.unwrap_or(false),
		allow_dynamic_max_players: game_mode.allow_dynamic_max_players.unwrap_or(true),

		runtime,

		actions: game_mode
			.actions
			.clone()
			.map(|x| ApiTryInto::try_into(*x))
			.transpose()?,
	})
}

pub fn game_mode_to_openapi(
	value: backend::matchmaker::LobbyGroup,
	regions_data: &[backend::region::Region],
) -> GlobalResult<(String, models::CloudVersionMatchmakerGameMode)> {
	let docker = match unwrap!(
		value.runtime.as_ref().and_then(|x| x.runtime.as_ref()),
		"unknown runtime"
	) {
		backend::matchmaker::lobby_runtime::Runtime::Docker(runtime) => {
			models::CloudVersionMatchmakerGameModeRuntimeDocker {
				image_id: Some(unwrap_ref!(runtime.build_id).as_uuid()),
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
					unwrap!(backend::matchmaker::lobby_runtime::NetworkMode::from_i32(
						runtime.network_mode,
					))
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
									port: x.target_port.map(|x| x.try_into()).transpose()?,
									port_range: x
										.port_range
										.clone()
										.map(ApiTryInto::try_into)
										.transpose()?
										.map(Box::new),
									protocol: Some(
										unwrap!(
										backend::matchmaker::lobby_runtime::ProxyProtocol::from_i32(
											x.proxy_protocol
										)
									)
										.api_into(),
									),
									proxy: Some(
										unwrap!(
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
			max_players: Some(value.max_players_normal.try_into()?),
			max_players_direct: Some(value.max_players_direct.try_into()?),
			max_players_party: Some(value.max_players_party.try_into()?),
			listable: Some(value.listable),
			taggable: Some(value.taggable),
			allow_dynamic_max_players: Some(value.allow_dynamic_max_players),

			docker: Some(Box::new(docker)),

			actions: value
				.actions
				.map(ApiTryInto::try_into)
				.transpose()?
				.map(Box::new),

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
		.map(|x| *x)
		.map(ApiTryInto::try_into)
		.transpose()?;

	Ok(backend::matchmaker::lobby_group::Region {
		region_id,
		tier_name_id,
		idle_lobbies,
	})
}

fn region_to_openapi(
	region: backend::matchmaker::lobby_group::Region,
	regions_data: &[backend::region::Region],
) -> GlobalResult<(String, models::CloudVersionMatchmakerGameModeRegion)> {
	let name_id = if let Some(region_data) = regions_data
		.iter()
		.find(|x| x.region_id == region.region_id)
	{
		region_data.name_id.clone()
	} else {
		tracing::warn!(?region, "no region data for region");
		"unknown".to_string()
	};

	Ok((
		name_id,
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
		ensure_with!(
			value.min >= 0,
			MATCHMAKER_INVALID_VERSION_CONFIG,
			error = "`idle_lobbies.min` out of bounds"
		);
		ensure_with!(
			value.max >= 0,
			MATCHMAKER_INVALID_VERSION_CONFIG,
			error = "`idle_lobbies.max` out of bounds"
		);

		Ok(backend::matchmaker::lobby_group::IdleLobbies {
			min_idle_lobbies: value.min.try_into()?,
			max_idle_lobbies: value.max.try_into()?,
		})
	}
}

impl ApiTryFrom<backend::matchmaker::lobby_group::IdleLobbies>
	for models::CloudVersionMatchmakerGameModeIdleLobbiesConfig
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::lobby_group::IdleLobbies) -> GlobalResult<Self> {
		Ok(models::CloudVersionMatchmakerGameModeIdleLobbiesConfig {
			min: value.min_idle_lobbies.try_into()?,
			max: value.max_idle_lobbies.try_into()?,
		})
	}
}

impl ApiFrom<models::CloudVersionMatchmakerGameModeIdentityRequirement>
	for backend::matchmaker::IdentityRequirement
{
	fn api_from(
		value: models::CloudVersionMatchmakerGameModeIdentityRequirement,
	) -> backend::matchmaker::IdentityRequirement {
		match value {
			models::CloudVersionMatchmakerGameModeIdentityRequirement::None => {
				backend::matchmaker::IdentityRequirement::None
			}
			models::CloudVersionMatchmakerGameModeIdentityRequirement::Guest => {
				backend::matchmaker::IdentityRequirement::Guest
			}
			models::CloudVersionMatchmakerGameModeIdentityRequirement::Registered => {
				backend::matchmaker::IdentityRequirement::Registered
			}
		}
	}
}

impl ApiFrom<backend::matchmaker::IdentityRequirement>
	for models::CloudVersionMatchmakerGameModeIdentityRequirement
{
	fn api_from(value: backend::matchmaker::IdentityRequirement) -> Self {
		match value {
			backend::matchmaker::IdentityRequirement::None => {
				models::CloudVersionMatchmakerGameModeIdentityRequirement::None
			}
			backend::matchmaker::IdentityRequirement::Guest => {
				models::CloudVersionMatchmakerGameModeIdentityRequirement::Guest
			}
			backend::matchmaker::IdentityRequirement::Registered => {
				models::CloudVersionMatchmakerGameModeIdentityRequirement::Registered
			}
		}
	}
}

impl ApiTryFrom<models::CloudVersionMatchmakerGameModeActions>
	for backend::matchmaker::lobby_group::Actions
{
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionMatchmakerGameModeActions) -> GlobalResult<Self> {
		Ok(backend::matchmaker::lobby_group::Actions {
			find: value
				.find
				.clone()
				.map(|x| ApiTryInto::try_into(*x))
				.transpose()?,
			join: value
				.join
				.clone()
				.map(|x| ApiTryInto::try_into(*x))
				.transpose()?,
			create: value
				.create
				.clone()
				.map(|x| ApiTryInto::try_into(*x))
				.transpose()?,
		})
	}
}

impl ApiTryFrom<backend::matchmaker::lobby_group::Actions>
	for models::CloudVersionMatchmakerGameModeActions
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::lobby_group::Actions) -> GlobalResult<Self> {
		Ok(models::CloudVersionMatchmakerGameModeActions {
			find: value
				.find
				.map(ApiTryInto::try_into)
				.transpose()?
				.map(Box::new),
			join: value
				.join
				.map(ApiTryInto::try_into)
				.transpose()?
				.map(Box::new),
			create: value
				.create
				.map(ApiTryInto::try_into)
				.transpose()?
				.map(Box::new),
		})
	}
}

impl ApiTryFrom<models::CloudVersionMatchmakerGameModeVerificationConfig>
	for backend::matchmaker::VerificationConfig
{
	type Error = GlobalError;

	fn try_from(
		value: models::CloudVersionMatchmakerGameModeVerificationConfig,
	) -> GlobalResult<Self> {
		Ok(backend::matchmaker::VerificationConfig {
			url: value.url,
			headers: value.headers,
		})
	}
}

impl ApiTryFrom<backend::matchmaker::VerificationConfig>
	for models::CloudVersionMatchmakerGameModeVerificationConfig
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::VerificationConfig) -> GlobalResult<Self> {
		Ok(models::CloudVersionMatchmakerGameModeVerificationConfig {
			url: value.url,
			headers: value.headers,
		})
	}
}

impl ApiTryFrom<models::CloudVersionMatchmakerGameModeFindConfig>
	for backend::matchmaker::FindConfig
{
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionMatchmakerGameModeFindConfig) -> GlobalResult<Self> {
		Ok(backend::matchmaker::FindConfig {
			enabled: value.enabled,
			identity_requirement: value
				.identity_requirement
				.map(ApiInto::<backend::matchmaker::IdentityRequirement>::api_into)
				.unwrap_or(backend::matchmaker::IdentityRequirement::None) as i32,
			verification: value
				.verification
				.map(|x| ApiTryInto::try_into(*x))
				.transpose()?,
		})
	}
}

impl ApiTryFrom<backend::matchmaker::FindConfig>
	for models::CloudVersionMatchmakerGameModeFindConfig
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::FindConfig) -> GlobalResult<Self> {
		Ok(models::CloudVersionMatchmakerGameModeFindConfig {
			enabled: value.enabled,
			identity_requirement: Some(
				unwrap!(
					backend::matchmaker::IdentityRequirement::from_i32(value.identity_requirement),
					"invalid identity requirement variant"
				)
				.api_into(),
			),
			verification: value
				.verification
				.map(ApiTryInto::try_into)
				.transpose()?
				.map(Box::new),
		})
	}
}

impl ApiTryFrom<models::CloudVersionMatchmakerGameModeJoinConfig>
	for backend::matchmaker::JoinConfig
{
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionMatchmakerGameModeJoinConfig) -> GlobalResult<Self> {
		Ok(backend::matchmaker::JoinConfig {
			enabled: value.enabled,
			identity_requirement: value
				.identity_requirement
				.map(ApiInto::<backend::matchmaker::IdentityRequirement>::api_into)
				.unwrap_or(backend::matchmaker::IdentityRequirement::None) as i32,
			verification: value
				.verification
				.map(|x| ApiTryInto::try_into(*x))
				.transpose()?,
		})
	}
}

impl ApiTryFrom<backend::matchmaker::JoinConfig>
	for models::CloudVersionMatchmakerGameModeJoinConfig
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::JoinConfig) -> GlobalResult<Self> {
		Ok(models::CloudVersionMatchmakerGameModeJoinConfig {
			enabled: value.enabled,
			identity_requirement: Some(
				unwrap!(
					backend::matchmaker::IdentityRequirement::from_i32(value.identity_requirement),
					"invalid identity requirement variant"
				)
				.api_into(),
			),
			verification: value
				.verification
				.map(ApiTryInto::try_into)
				.transpose()?
				.map(Box::new),
		})
	}
}

impl ApiTryFrom<models::CloudVersionMatchmakerGameModeCreateConfig>
	for backend::matchmaker::CreateConfig
{
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionMatchmakerGameModeCreateConfig) -> GlobalResult<Self> {
		if let Some(max_lobbies_per_identity) = value.max_lobbies_per_identity {
			ensure_with!(
				max_lobbies_per_identity >= 0,
				MATCHMAKER_INVALID_VERSION_CONFIG,
				error = "`actions.create.max_lobbies_per_identity` out of bounds"
			);
		}

		Ok(backend::matchmaker::CreateConfig {
			enabled: value.enabled,
			identity_requirement: value
				.identity_requirement
				.map(ApiInto::<backend::matchmaker::IdentityRequirement>::api_into)
				.unwrap_or(backend::matchmaker::IdentityRequirement::None) as i32,
			verification: value
				.verification
				.map(|x| ApiTryInto::try_into(*x))
				.transpose()?,
			enable_public: value.enable_public.unwrap_or(false),
			enable_private: value.enable_private.unwrap_or(true),
			max_lobbies_per_identity: value
				.max_lobbies_per_identity
				.map(ApiTryInto::try_into)
				.transpose()?,
		})
	}
}

impl ApiTryFrom<backend::matchmaker::CreateConfig>
	for models::CloudVersionMatchmakerGameModeCreateConfig
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::CreateConfig) -> GlobalResult<Self> {
		Ok(models::CloudVersionMatchmakerGameModeCreateConfig {
			enabled: value.enabled,
			identity_requirement: Some(
				unwrap!(
					backend::matchmaker::IdentityRequirement::from_i32(value.identity_requirement),
					"invalid identity requirement variant"
				)
				.api_into(),
			),
			verification: value
				.verification
				.map(ApiTryInto::try_into)
				.transpose()?
				.map(Box::new),
			enable_public: Some(value.enable_public),
			enable_private: Some(value.enable_private),
			max_lobbies_per_identity: value
				.max_lobbies_per_identity
				.map(ApiTryInto::try_into)
				.transpose()?,
		})
	}
}

impl ApiFrom<models::MatchmakerCustomLobbyPublicity> for backend::matchmaker::lobby::Publicity {
	fn api_from(
		value: models::MatchmakerCustomLobbyPublicity,
	) -> backend::matchmaker::lobby::Publicity {
		match value {
			models::MatchmakerCustomLobbyPublicity::Public => {
				backend::matchmaker::lobby::Publicity::Public
			}
			models::MatchmakerCustomLobbyPublicity::Private => {
				backend::matchmaker::lobby::Publicity::Private
			}
		}
	}
}
