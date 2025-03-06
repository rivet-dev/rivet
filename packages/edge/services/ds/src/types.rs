use std::{collections::HashMap, fmt};

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
	pub tags: HashMap<String, String>,
	pub resources: ServerResources,
	pub lifecycle: ServerLifecycle,
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

impl ServerResources {
	pub fn default_isolate() -> Self {
		ServerResources {
			cpu_millicores: 125,
			memory_mib: 128,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ServerLifecycle {
	pub kill_timeout_ms: i64,
	pub durable: bool,
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
	pub public_path: Option<String>,
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

impl fmt::Display for GameGuardProtocol {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			GameGuardProtocol::Http => write!(f, "http"),
			GameGuardProtocol::Https => write!(f, "https"),
			GameGuardProtocol::Tcp => write!(f, "tcp"),
			GameGuardProtocol::TcpTls => write!(f, "tcps"),
			GameGuardProtocol::Udp => write!(f, "udp"),
		}
	}
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum HostProtocol {
	Tcp = 0,
	Udp = 1,
}

#[derive(Debug, Clone)]
pub struct GameConfig {
	pub game_id: Uuid,
	pub host_networking_enabled: bool,
	pub root_user_enabled: bool,
}

impl GameConfig {
	pub fn default(game_id: Uuid) -> Self {
		GameConfig {
			game_id,
			host_networking_enabled: false,
			root_user_enabled: false,
		}
	}
}

/// Determines how port endpoints are returned.
#[derive(Debug, Copy, Clone, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EndpointType {
	#[serde(rename = "hostname")]
	Hostname = 0,
	#[serde(rename = "path")]
	Path = 1,
}

impl EndpointType {
	pub fn default_for_guard_public_hostname(
		hostname: &cluster::types::GuardPublicHostname,
	) -> Self {
		match hostname {
			cluster::types::GuardPublicHostname::DnsParent(_) => Self::Hostname,
			cluster::types::GuardPublicHostname::Static(_) => Self::Path,
		}
	}
}

impl fmt::Display for EndpointType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			EndpointType::Hostname => write!(f, "hostname"),
			EndpointType::Path => write!(f, "path"),
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
		created_at: util::timestamp::to_string(value.create_ts)?,
		// `started_at` -> `connectable_ts` is intentional. We don't expose the internal
		// workings of DS to the API, so we need to return the timestamp at which the serer can
		// actually do anything useful.
		started_at: value
			.connectable_ts
			.map(util::timestamp::to_string)
			.transpose()?,
		destroyed_at: value
			.destroy_ts
			.map(util::timestamp::to_string)
			.transpose()?,
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
		lifecycle: Box::new(value.lifecycle.api_into()),
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

impl ApiFrom<models::ActorLifecycle> for ServerLifecycle {
	fn api_from(value: models::ActorLifecycle) -> ServerLifecycle {
		ServerLifecycle {
			kill_timeout_ms: value.kill_timeout.unwrap_or_default(),
			durable: value.durable.unwrap_or_default(),
		}
	}
}

impl ApiFrom<ServerLifecycle> for models::ActorLifecycle {
	fn api_from(value: ServerLifecycle) -> models::ActorLifecycle {
		models::ActorLifecycle {
			kill_timeout: Some(value.kill_timeout_ms),
			durable: Some(value.durable),
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
		let (protocol, routing, url) = match &value.routing {
			Routing::GameGuard { protocol } => {
				let url = match (
					protocol,
					value.public_hostname.as_ref(),
					value.public_port,
					value.public_path.as_ref(),
				) {
					(
						GameGuardProtocol::Http | GameGuardProtocol::Https,
						Some(hostname),
						Some(port),
						path,
					) => Some(format!(
						"{protocol}://{hostname}:{port}{}",
						util::format::OptDisplay(path)
					)),
					(
						GameGuardProtocol::Http | GameGuardProtocol::Https,
						Some(hostname),
						None,
						path,
					) => Some(format!(
						"{protocol}://{hostname}{}",
						util::format::OptDisplay(path)
					)),
					_ => None,
				};

				(
					(*protocol).api_into(),
					models::ActorPortRouting {
						guard: Some(json!({})),
						..Default::default()
					},
					url,
				)
			}
			Routing::Host { protocol } => (
				(*protocol).api_into(),
				models::ActorPortRouting {
					host: Some(json!({})),
					..Default::default()
				},
				None,
			),
		};

		models::ActorPort {
			protocol,
			internal_port: value.internal_port,
			hostname: value.public_hostname,
			port: value.public_port,
			path: value.public_path,
			url,
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
			_ => {
				bail_with!(
					ACTOR_FAILED_TO_CREATE,
					error = "Host port protocol must be either TCP or UDP."
				);
			}
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

impl ApiFrom<models::ActorEndpointType> for EndpointType {
	fn api_from(value: models::ActorEndpointType) -> EndpointType {
		match value {
			models::ActorEndpointType::Hostname => EndpointType::Hostname,
			models::ActorEndpointType::Path => EndpointType::Path,
		}
	}
}

impl ApiFrom<EndpointType> for models::ActorEndpointType {
	fn api_from(value: EndpointType) -> models::ActorEndpointType {
		match value {
			EndpointType::Hostname => models::ActorEndpointType::Hostname,
			EndpointType::Path => models::ActorEndpointType::Path,
		}
	}
}
