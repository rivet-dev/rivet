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
	pub cpu_millicores: i32,
	pub memory_mib: i32,
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
	GameGuard { protocol: GameGuardProtocol },
	Host { protocol: HostProtocol },
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum GameGuardProtocol {
	Http = 0,
	Https = 1,
	Tcp = 2,
	TcpTls = 3,
	Udp = 4,
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum HostProtocol {
	Tcp = 0,
	Udp = 1,
}

// Move to build pkg when migrated to workflows
pub mod build {
	use serde::{Deserialize, Serialize};
	use strum::FromRepr;

	#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
	pub enum BuildKind {
		DockerImage = 0,
		OciBundle = 1,
	}

	#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
	pub enum BuildCompression {
		None = 0,
		Lz4 = 1,
	}
}

impl ApiTryFrom<Server> for models::ServersServer {
	type Error = GlobalError;

	fn api_try_from(value: Server) -> GlobalResult<models::ServersServer> {
		Ok(models::ServersServer {
			id: value.server_id,
			environment: value.env_id,
			datacenter: value.datacenter_id,
			cluster: value.cluster_id,
			created_at: value.create_ts,
			started_at: value.start_ts,
			connectable_at: value.connectable_ts,
			destroyed_at: value.destroy_ts,
			tags: Some(serde_json::to_value(value.tags)?),
			runtime: Box::new(models::ServersRuntime {
				build: value.image_id,
				arguments: Some(value.args),
				environment: Some(value.environment),
			}),
			network: Box::new(models::ServersNetwork {
				mode: Some(value.network_mode.api_into()),
				ports: value
					.network_ports
					.into_iter()
					.map(|(s, p)| (s, p.api_into()))
					.collect::<HashMap<_, _>>(),
			}),
			lifecycle: Box::new(models::ServersLifecycle {
				kill_timeout: Some(value.kill_timeout_ms),
			}),
			resources: Box::new(value.resources.api_into()),
		})
	}
}

impl ApiFrom<models::ServersResources> for ServerResources {
	fn api_from(value: models::ServersResources) -> ServerResources {
		ServerResources {
			cpu_millicores: value.cpu,
			memory_mib: value.memory,
		}
	}
}

impl ApiFrom<ServerResources> for models::ServersResources {
	fn api_from(value: ServerResources) -> models::ServersResources {
		models::ServersResources {
			cpu: value.cpu_millicores,
			memory: value.memory_mib,
		}
	}
}

impl ApiFrom<models::ServersNetworkMode> for NetworkMode {
	fn api_from(value: models::ServersNetworkMode) -> NetworkMode {
		match value {
			models::ServersNetworkMode::Bridge => NetworkMode::Bridge,
			models::ServersNetworkMode::Host => NetworkMode::Host,
		}
	}
}

impl ApiFrom<NetworkMode> for models::ServersNetworkMode {
	fn api_from(value: NetworkMode) -> models::ServersNetworkMode {
		match value {
			NetworkMode::Bridge => models::ServersNetworkMode::Bridge,
			NetworkMode::Host => models::ServersNetworkMode::Host,
		}
	}
}

impl ApiFrom<Port> for models::ServersPort {
	fn api_from(value: Port) -> models::ServersPort {
		let (protocol, routing) = match &value.routing {
			Routing::GameGuard { protocol } => (
				(*protocol).api_into(),
				models::ServersPortRouting {
					game_guard: Some(json!({})),
					..Default::default()
				},
			),
			Routing::Host { protocol } => (
				(*protocol).api_into(),
				models::ServersPortRouting {
					host: Some(json!({})),
					..Default::default()
				},
			),
		};

		models::ServersPort {
			protocol,
			internal_port: value.internal_port,
			public_hostname: value.public_hostname,
			public_port: value.public_port,
			routing: Box::new(routing),
		}
	}
}

impl ApiFrom<models::ServersPortProtocol> for GameGuardProtocol {
	fn api_from(value: models::ServersPortProtocol) -> GameGuardProtocol {
		match value {
			models::ServersPortProtocol::Udp => GameGuardProtocol::Udp,
			models::ServersPortProtocol::Tcp => GameGuardProtocol::Tcp,
			models::ServersPortProtocol::Http => GameGuardProtocol::Http,
			models::ServersPortProtocol::Https => GameGuardProtocol::Https,
			models::ServersPortProtocol::TcpTls => GameGuardProtocol::TcpTls,
		}
	}
}

impl ApiFrom<GameGuardProtocol> for models::ServersPortProtocol {
	fn api_from(value: GameGuardProtocol) -> models::ServersPortProtocol {
		match value {
			GameGuardProtocol::Udp => models::ServersPortProtocol::Udp,
			GameGuardProtocol::Tcp => models::ServersPortProtocol::Tcp,
			GameGuardProtocol::Http => models::ServersPortProtocol::Http,
			GameGuardProtocol::Https => models::ServersPortProtocol::Https,
			GameGuardProtocol::TcpTls => models::ServersPortProtocol::TcpTls,
		}
	}
}

impl ApiTryFrom<models::ServersPortProtocol> for HostProtocol {
	type Error = GlobalError;
	fn api_try_from(value: models::ServersPortProtocol) -> GlobalResult<HostProtocol> {
		Ok(match value {
			models::ServersPortProtocol::Udp => HostProtocol::Udp,
			models::ServersPortProtocol::Tcp => HostProtocol::Tcp,
			_ => bail_with!(SERVERS_UNSUPPORTED_HOST_PROTOCOL),
		})
	}
}

impl ApiFrom<HostProtocol> for models::ServersPortProtocol {
	fn api_from(value: HostProtocol) -> models::ServersPortProtocol {
		match value {
			HostProtocol::Udp => models::ServersPortProtocol::Udp,
			HostProtocol::Tcp => models::ServersPortProtocol::Tcp,
		}
	}
}
