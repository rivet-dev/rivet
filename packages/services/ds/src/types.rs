use std::collections::HashMap;

use chirp_workflow::prelude::*;
use rivet_api::models;
use rivet_convert::{ApiFrom, ApiInto, ApiTryFrom};
use serde::{Deserialize, Serialize};
use serde_json::json;
use strum::FromRepr;

#[derive(Debug, Clone)]
pub struct Server {
	pub server_id: Uuid,
	pub env_id: Uuid,
	pub datacenter_id: Uuid,
	pub cluster_id: Uuid,
	pub tags: HashMap<String, String>,
	pub resources: ServerResources,
	pub kill_timeout_ms: i64,
	pub create_ts: i64,
	pub start_ts: Option<i64>,
	pub connectable_ts: Option<i64>,
	pub destroy_ts: Option<i64>,
	pub image_id: Uuid,
	pub args: Vec<String>,
	pub network_mode: NetworkMode,
	pub environment: HashMap<String, String>,
	pub network_ports: HashMap<String, Port>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ServerResources {
	pub cpu_millicores: u32,
	pub memory_mib: u32,
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum NetworkMode {
	Bridge = 0,
	Host = 1,
}

#[derive(Debug, Clone)]
pub struct Port {
	// Null when using host networking since one is automatically assigned
	pub internal_port: Option<i32>,
	pub public_hostname: Option<String>,
	pub public_port: Option<i32>,
	pub routing: Routing,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum Routing {
	GameGuard {
		protocol: GameGuardProtocol,
		authorization: PortAuthorization,
	},
	Host {
		protocol: HostProtocol,
	},
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum GameGuardProtocol {
	Http = 0,
	Https = 1,
	Tcp = 2,
	TcpTls = 3,
	Udp = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum PortAuthorization {
	None,
	Bearer(String),
	Query(String, String),
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum PortAuthorizationType {
	None = 0,
	Bearer = 1,
	Query = 2,
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum HostProtocol {
	Tcp = 0,
	Udp = 1,
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr, Default)]
pub enum ServerRuntime {
	Nomad = 0,
	#[default]
	Pegboard = 1,
}

#[derive(Debug, Clone)]
pub struct GameConfig {
	pub game_id: Uuid,
	pub host_networking_enabled: bool,
	pub root_user_enabled: bool,
	pub runtime: ServerRuntime,
}

impl GameConfig {
	pub fn default(game_id: Uuid) -> Self {
		GameConfig {
			game_id,
			host_networking_enabled: false,
			root_user_enabled: false,
			runtime: ServerRuntime::default(),
		}
	}
}

pub fn convert_actor_to_api(
	value: Server,
	datacenter: &cluster::types::Datacenter,
) -> GlobalResult<models::ActorActor> {
	Ok(models::ActorActor {
		id: value.server_id,
		region: datacenter.name_id.clone(),
		created_at: value.create_ts,
		// `started_at` -> `connectable_ts` is intentional. We don't expose the internal
		// workings of DS to the API, so we need to return the timestamp at which the serer can
		// actually do anything useful.
		started_at: value.connectable_ts,
		destroyed_at: value.destroy_ts,
		tags: Some(serde_json::to_value(value.tags)?),
		runtime: Box::new(models::ActorRuntime {
			build: value.image_id,
			arguments: Some(value.args),
			environment: Some(value.environment),
		}),
		network: Box::new(models::ActorNetwork {
			mode: value.network_mode.api_into(),
			ports: value
				.network_ports
				.into_iter()
				.map(|(s, p)| (s, p.api_into()))
				.collect::<HashMap<_, _>>(),
		}),
		lifecycle: Box::new(models::ActorLifecycle {
			kill_timeout: Some(value.kill_timeout_ms),
		}),
		resources: Box::new(value.resources.api_into()),
	})
}

impl ApiFrom<models::ActorResources> for ServerResources {
	fn api_from(value: models::ActorResources) -> ServerResources {
		ServerResources {
			cpu_millicores: value.cpu as u32,
			memory_mib: value.memory as u32,
		}
	}
}

impl ApiFrom<ServerResources> for models::ActorResources {
	fn api_from(value: ServerResources) -> models::ActorResources {
		models::ActorResources {
			cpu: value.cpu_millicores as i32,
			memory: value.memory_mib as i32,
		}
	}
}

impl ApiFrom<models::ActorNetworkMode> for NetworkMode {
	fn api_from(value: models::ActorNetworkMode) -> NetworkMode {
		match value {
			models::ActorNetworkMode::Bridge => NetworkMode::Bridge,
			models::ActorNetworkMode::Host => NetworkMode::Host,
		}
	}
}

impl ApiFrom<NetworkMode> for models::ActorNetworkMode {
	fn api_from(value: NetworkMode) -> models::ActorNetworkMode {
		match value {
			NetworkMode::Bridge => models::ActorNetworkMode::Bridge,
			NetworkMode::Host => models::ActorNetworkMode::Host,
		}
	}
}

impl ApiFrom<Port> for models::ActorPort {
	fn api_from(value: Port) -> models::ActorPort {
		let (protocol, routing) = match &value.routing {
			Routing::GameGuard {
				protocol,
				authorization,
			} => (
				(*protocol).api_into(),
				models::ActorPortRouting {
					game_guard: Some(Box::new(models::ActorGameGuardRouting {
						authorization: match authorization {
							PortAuthorization::None => None,
							PortAuthorization::Bearer(token) => {
								Some(Box::new(models::ActorPortAuthorization {
									bearer: Some(token.clone()),
									..Default::default()
								}))
							}
							PortAuthorization::Query(key, value) => {
								Some(Box::new(models::ActorPortAuthorization {
									query: Some(Box::new(models::ActorPortQueryAuthorization {
										key: key.clone(),
										value: value.clone(),
									})),
									..Default::default()
								}))
							}
						},
					})),
					..Default::default()
				},
			),
			Routing::Host { protocol } => (
				(*protocol).api_into(),
				models::ActorPortRouting {
					host: Some(json!({})),
					..Default::default()
				},
			),
		};

		models::ActorPort {
			protocol,
			internal_port: value.internal_port,
			public_hostname: value.public_hostname,
			public_port: value.public_port,
			routing: Box::new(routing),
		}
	}
}

impl ApiFrom<models::ActorPortProtocol> for GameGuardProtocol {
	fn api_from(value: models::ActorPortProtocol) -> GameGuardProtocol {
		match value {
			models::ActorPortProtocol::Udp => GameGuardProtocol::Udp,
			models::ActorPortProtocol::Tcp => GameGuardProtocol::Tcp,
			models::ActorPortProtocol::Http => GameGuardProtocol::Http,
			models::ActorPortProtocol::Https => GameGuardProtocol::Https,
			models::ActorPortProtocol::TcpTls => GameGuardProtocol::TcpTls,
		}
	}
}

impl ApiFrom<GameGuardProtocol> for models::ActorPortProtocol {
	fn api_from(value: GameGuardProtocol) -> models::ActorPortProtocol {
		match value {
			GameGuardProtocol::Udp => models::ActorPortProtocol::Udp,
			GameGuardProtocol::Tcp => models::ActorPortProtocol::Tcp,
			GameGuardProtocol::Http => models::ActorPortProtocol::Http,
			GameGuardProtocol::Https => models::ActorPortProtocol::Https,
			GameGuardProtocol::TcpTls => models::ActorPortProtocol::TcpTls,
		}
	}
}

impl ApiTryFrom<models::ActorPortProtocol> for HostProtocol {
	type Error = GlobalError;
	fn api_try_from(value: models::ActorPortProtocol) -> GlobalResult<HostProtocol> {
		Ok(match value {
			models::ActorPortProtocol::Udp => HostProtocol::Udp,
			models::ActorPortProtocol::Tcp => HostProtocol::Tcp,
			_ => bail_with!(SERVERS_UNSUPPORTED_HOST_PROTOCOL),
		})
	}
}

impl ApiFrom<HostProtocol> for models::ActorPortProtocol {
	fn api_from(value: HostProtocol) -> models::ActorPortProtocol {
		match value {
			HostProtocol::Udp => models::ActorPortProtocol::Udp,
			HostProtocol::Tcp => models::ActorPortProtocol::Tcp,
		}
	}
}
