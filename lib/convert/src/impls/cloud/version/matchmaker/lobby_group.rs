use proto::backend;
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{ApiFrom, ApiInto, ApiTryFrom, ApiTryInto};

impl ApiTryFrom<models::CloudVersionMatchmakerLobbyGroup> for backend::matchmaker::LobbyGroup {
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionMatchmakerLobbyGroup) -> GlobalResult<Self> {
		assert_with!(
			value.max_players_normal >= 0,
			MATCHMAKER_INVALID_VERSION_CONFIG,
			error = "`max_players` out of bounds"
		);
		assert_with!(
			value.max_players_direct >= 0,
			MATCHMAKER_INVALID_VERSION_CONFIG,
			error = "`max_players_direct` out of bounds"
		);
		assert_with!(
			value.max_players_party >= 0,
			MATCHMAKER_INVALID_VERSION_CONFIG,
			error = "`max_players_party` out of bounds"
		);

		Ok(backend::matchmaker::LobbyGroup {
			name_id: value.name_id,

			regions: value
				.regions
				.into_iter()
				.map(|x| x.try_into())
				.collect::<GlobalResult<_>>()?,
			max_players_normal: value.max_players_normal as u32,
			max_players_direct: value.max_players_direct as u32,
			max_players_party: value.max_players_party as u32,
			listable: true,

			runtime: Some((*value.runtime).try_into()?),

			find_config: None,
			join_config: None,
			create_config: None,
		})
	}
}

impl ApiTryFrom<backend::matchmaker::LobbyGroup> for models::CloudVersionMatchmakerLobbyGroup {
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::LobbyGroup) -> GlobalResult<Self> {
		Ok(models::CloudVersionMatchmakerLobbyGroup {
			name_id: value.name_id.clone(),

			regions: value
				.regions
				.into_iter()
				.map(ApiTryFrom::try_from)
				.collect::<Result<Vec<_>, _>>()?,
			max_players_normal: value.max_players_normal.try_into()?,
			max_players_direct: value.max_players_direct.try_into()?,
			max_players_party: value.max_players_party.try_into()?,

			runtime: Box::new(internal_unwrap_owned!(value.runtime).try_into()?),
		})
	}
}

impl ApiTryFrom<models::CloudVersionMatchmakerLobbyGroupRegion>
	for backend::matchmaker::lobby_group::Region
{
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionMatchmakerLobbyGroupRegion) -> GlobalResult<Self> {
		Ok(backend::matchmaker::lobby_group::Region {
			region_id: Some(value.region_id.into()),
			tier_name_id: value.tier_name_id.to_owned(),
			idle_lobbies: value.idle_lobbies.map(|x| (*x).try_into()).transpose()?,
		})
	}
}

impl ApiTryFrom<backend::matchmaker::lobby_group::Region>
	for models::CloudVersionMatchmakerLobbyGroupRegion
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::lobby_group::Region) -> GlobalResult<Self> {
		Ok(models::CloudVersionMatchmakerLobbyGroupRegion {
			region_id: internal_unwrap_owned!(value.region_id).as_uuid(),
			tier_name_id: value.tier_name_id.to_owned(),
			idle_lobbies: value
				.idle_lobbies
				.map(ApiTryInto::try_into)
				.transpose()?
				.map(Box::new),
		})
	}
}

impl ApiTryFrom<models::CloudVersionMatchmakerLobbyGroupIdleLobbiesConfig>
	for backend::matchmaker::lobby_group::IdleLobbies
{
	type Error = GlobalError;

	fn try_from(
		value: models::CloudVersionMatchmakerLobbyGroupIdleLobbiesConfig,
	) -> GlobalResult<Self> {
		assert_with!(
			value.min_idle_lobbies >= 0,
			MATCHMAKER_INVALID_VERSION_CONFIG,
			error = "`min_idle_lobbies` out of bounds"
		);
		assert_with!(
			value.max_idle_lobbies >= 0,
			MATCHMAKER_INVALID_VERSION_CONFIG,
			error = "`max_idle_lobbies` out of bounds"
		);

		Ok(backend::matchmaker::lobby_group::IdleLobbies {
			min_idle_lobbies: value.min_idle_lobbies.try_into()?,
			max_idle_lobbies: value.max_idle_lobbies.try_into()?,
		})
	}
}

impl ApiTryFrom<backend::matchmaker::lobby_group::IdleLobbies>
	for models::CloudVersionMatchmakerLobbyGroupIdleLobbiesConfig
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::lobby_group::IdleLobbies) -> GlobalResult<Self> {
		Ok(models::CloudVersionMatchmakerLobbyGroupIdleLobbiesConfig {
			min_idle_lobbies: value.min_idle_lobbies.try_into()?,
			max_idle_lobbies: value.max_idle_lobbies.try_into()?,
		})
	}
}

impl ApiTryFrom<models::CloudVersionMatchmakerLobbyGroupRuntime>
	for backend::matchmaker::LobbyRuntime
{
	type Error = GlobalError;

	fn try_from(value: models::CloudVersionMatchmakerLobbyGroupRuntime) -> GlobalResult<Self> {
		Ok(backend::matchmaker::LobbyRuntime {
			runtime: value
				.docker
				.map(|runtime| {
					GlobalResult::Ok(backend::matchmaker::lobby_runtime::Runtime::Docker(
						backend::matchmaker::lobby_runtime::Docker {
							build_id: runtime.build_id.map(Into::into),
							args: runtime.args,
							env_vars: runtime
								.env_vars
								.into_iter()
								.map(ApiInto::api_into)
								.collect(),
							network_mode:
								ApiInto::<backend::matchmaker::lobby_runtime::NetworkMode>::api_into(
									runtime.network_mode.unwrap_or(
										models::CloudVersionMatchmakerNetworkMode::Bridge,
									),
								) as i32,
							ports: runtime
								.ports
								.into_iter()
								.map(ApiTryInto::try_into)
								.collect::<GlobalResult<_>>()?,
						},
					))
				})
				.transpose()?,
		})
	}
}

impl ApiTryFrom<backend::matchmaker::LobbyRuntime>
	for models::CloudVersionMatchmakerLobbyGroupRuntime
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::LobbyRuntime) -> GlobalResult<Self> {
		let runtime = internal_unwrap_owned!(value.runtime);

		Ok(match runtime {
			backend::matchmaker::lobby_runtime::Runtime::Docker(runtime) => {
				models::CloudVersionMatchmakerLobbyGroupRuntime {
					docker: Some(Box::new(
						models::CloudVersionMatchmakerLobbyGroupRuntimeDocker {
							build_id: Some(internal_unwrap!(runtime.build_id).as_uuid()),
							args: runtime.args,
							env_vars: runtime
								.env_vars
								.into_iter()
								.map(ApiTryFrom::try_from)
								.collect::<Result<Vec<_>, _>>()?,
							network_mode: Some(
								internal_unwrap_owned!(
									backend::matchmaker::lobby_runtime::NetworkMode::from_i32(
										runtime.network_mode,
									)
								)
								.api_into(),
							),
							ports: runtime
								.ports
								.into_iter()
								.map(ApiTryFrom::try_from)
								.collect::<Result<Vec<_>, _>>()?,
						},
					)),
				}
			}
		})
	}
}

impl ApiTryFrom<models::CloudVersionMatchmakerLobbyGroupRuntimeDockerPort>
	for backend::matchmaker::lobby_runtime::Port
{
	type Error = GlobalError;

	fn try_from(
		value: models::CloudVersionMatchmakerLobbyGroupRuntimeDockerPort,
	) -> GlobalResult<Self> {
		if let Some(target_port) = value.target_port {
			internal_assert!(target_port >= 0);
		}

		let proxy_kind = if value.port_range.is_some() {
			backend::matchmaker::lobby_runtime::ProxyKind::None as i32
		} else {
			backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32
		};

		Ok(backend::matchmaker::lobby_runtime::Port {
			label: value.label,
			target_port: value
				.target_port
				.map(|x| {
					assert_with!(
						x >= 0,
						MATCHMAKER_INVALID_VERSION_CONFIG,
						error = "`target_port` out of bounds"
					);

					Ok(x.try_into()?)
				})
				.transpose()?,
			port_range: value.port_range.map(|x| (*x).try_into()).transpose()?,
			proxy_protocol: (ApiInto::<backend::matchmaker::lobby_runtime::ProxyProtocol>::api_into(
				value.proxy_protocol,
			)) as i32,
			proxy_kind,
		})
	}
}

impl ApiTryFrom<backend::matchmaker::lobby_runtime::Port>
	for models::CloudVersionMatchmakerLobbyGroupRuntimeDockerPort
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::lobby_runtime::Port) -> GlobalResult<Self> {
		Ok(models::CloudVersionMatchmakerLobbyGroupRuntimeDockerPort {
			label: value.label,
			target_port: value.target_port.map(|x| x.try_into()).transpose()?,
			port_range: value
				.port_range
				.map(ApiTryInto::try_into)
				.transpose()?
				.map(Box::new),
			proxy_protocol: internal_unwrap_owned!(
				backend::matchmaker::lobby_runtime::ProxyProtocol::from_i32(value.proxy_protocol)
			)
			.api_into(),
		})
	}
}

impl ApiFrom<models::CloudVersionMatchmakerLobbyGroupRuntimeDockerEnvVar>
	for backend::matchmaker::lobby_runtime::EnvVar
{
	fn api_from(
		value: models::CloudVersionMatchmakerLobbyGroupRuntimeDockerEnvVar,
	) -> backend::matchmaker::lobby_runtime::EnvVar {
		backend::matchmaker::lobby_runtime::EnvVar {
			key: value.key,
			value: value.value,
		}
	}
}

impl ApiTryFrom<backend::matchmaker::lobby_runtime::EnvVar>
	for models::CloudVersionMatchmakerLobbyGroupRuntimeDockerEnvVar
{
	type Error = GlobalError;

	fn try_from(value: backend::matchmaker::lobby_runtime::EnvVar) -> GlobalResult<Self> {
		Ok(
			models::CloudVersionMatchmakerLobbyGroupRuntimeDockerEnvVar {
				key: value.key,
				value: value.value,
			},
		)
	}
}
