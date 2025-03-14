use std::{collections::HashMap, fmt};

use chirp_workflow::prelude::*;
use rivet_api::models;
use rivet_convert::{ApiFrom, ApiInto, ApiTryFrom};
use serde::{Deserialize, Serialize};
use serde_json::json;
use strum::FromRepr;

#[derive(Debug, Clone)]
pub struct Actor {
	pub actor_id: Uuid,
	pub env_id: Uuid,
	pub tags: HashMap<String, String>,
	pub resources: ActorResources,
	pub lifecycle: ActorLifecycle,
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
pub struct ActorResources {
	pub cpu_millicores: u32,
	pub memory_mib: u32,
}

impl ActorResources {
	pub fn default_isolate() -> Self {
		ActorResources {
			// cpu_millicores: 125,
			// memory_mib: 128,
			cpu_millicores: 250,
			memory_mib: 256,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ActorLifecycle {
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

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
pub enum LogsStreamType {
	StdOut = 0,
	StdErr = 1,
}

pub fn convert_actor_to_api(
	value: Actor,
	datacenter: &cluster::types::Datacenter,
) -> GlobalResult<models::ActorsActor> {
	Ok(models::ActorsActor {
		id: value.actor_id,
		region: datacenter.name_id.clone(),
		created_at: util::timestamp::to_string(value.create_ts)?,
		// `started_at` -> `connectable_ts` is intentional. We don't expose the internal
		// workings of actors to the API, so we need to return the timestamp at which the server can
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
		runtime: Box::new(models::ActorsRuntime {
			build: value.image_id,
			arguments: Some(value.args),
			environment: Some(value.environment),
		}),
		network: Box::new(models::ActorsNetwork {
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

impl ApiFrom<models::ActorsResources> for ActorResources {
	fn api_from(value: models::ActorsResources) -> ActorResources {
		ActorResources {
			cpu_millicores: value.cpu as u32,
			memory_mib: value.memory as u32,
		}
	}
}

impl ApiFrom<ActorResources> for models::ActorsResources {
	fn api_from(value: ActorResources) -> models::ActorsResources {
		models::ActorsResources {
			cpu: value.cpu_millicores as i32,
			memory: value.memory_mib as i32,
		}
	}
}

impl ApiFrom<models::ActorsLifecycle> for ActorLifecycle {
	fn api_from(value: models::ActorsLifecycle) -> ActorLifecycle {
		ActorLifecycle {
			kill_timeout_ms: value.kill_timeout.unwrap_or_default(),
			durable: value.durable.unwrap_or_default(),
		}
	}
}

impl ApiFrom<ActorLifecycle> for models::ActorsLifecycle {
	fn api_from(value: ActorLifecycle) -> models::ActorsLifecycle {
		models::ActorsLifecycle {
			kill_timeout: Some(value.kill_timeout_ms),
			durable: Some(value.durable),
		}
	}
}

impl ApiFrom<models::ActorsNetworkMode> for NetworkMode {
	fn api_from(value: models::ActorsNetworkMode) -> NetworkMode {
		match value {
			models::ActorsNetworkMode::Bridge => NetworkMode::Bridge,
			models::ActorsNetworkMode::Host => NetworkMode::Host,
		}
	}
}

impl ApiFrom<NetworkMode> for models::ActorsNetworkMode {
	fn api_from(value: NetworkMode) -> models::ActorsNetworkMode {
		match value {
			NetworkMode::Bridge => models::ActorsNetworkMode::Bridge,
			NetworkMode::Host => models::ActorsNetworkMode::Host,
		}
	}
}

impl ApiFrom<Port> for models::ActorsPort {
	fn api_from(value: Port) -> models::ActorsPort {
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
					models::ActorsPortRouting {
						guard: Some(json!({})),
						..Default::default()
					},
					url,
				)
			}
			Routing::Host { protocol } => (
				(*protocol).api_into(),
				models::ActorsPortRouting {
					host: Some(json!({})),
					..Default::default()
				},
				None,
			),
		};

		models::ActorsPort {
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

impl ApiFrom<models::ActorsPortProtocol> for GameGuardProtocol {
	fn api_from(value: models::ActorsPortProtocol) -> GameGuardProtocol {
		match value {
			models::ActorsPortProtocol::Udp => GameGuardProtocol::Udp,
			models::ActorsPortProtocol::Tcp => GameGuardProtocol::Tcp,
			models::ActorsPortProtocol::Http => GameGuardProtocol::Http,
			models::ActorsPortProtocol::Https => GameGuardProtocol::Https,
			models::ActorsPortProtocol::TcpTls => GameGuardProtocol::TcpTls,
		}
	}
}

impl ApiFrom<GameGuardProtocol> for models::ActorsPortProtocol {
	fn api_from(value: GameGuardProtocol) -> models::ActorsPortProtocol {
		match value {
			GameGuardProtocol::Udp => models::ActorsPortProtocol::Udp,
			GameGuardProtocol::Tcp => models::ActorsPortProtocol::Tcp,
			GameGuardProtocol::Http => models::ActorsPortProtocol::Http,
			GameGuardProtocol::Https => models::ActorsPortProtocol::Https,
			GameGuardProtocol::TcpTls => models::ActorsPortProtocol::TcpTls,
		}
	}
}

impl ApiTryFrom<models::ActorsPortProtocol> for HostProtocol {
	type Error = GlobalError;
	fn api_try_from(value: models::ActorsPortProtocol) -> GlobalResult<HostProtocol> {
		Ok(match value {
			models::ActorsPortProtocol::Udp => HostProtocol::Udp,
			models::ActorsPortProtocol::Tcp => HostProtocol::Tcp,
			_ => {
				bail_with!(
					ACTOR_FAILED_TO_CREATE,
					error = "Host port protocol must be either TCP or UDP."
				);
			}
		})
	}
}

impl ApiFrom<HostProtocol> for models::ActorsPortProtocol {
	fn api_from(value: HostProtocol) -> models::ActorsPortProtocol {
		match value {
			HostProtocol::Udp => models::ActorsPortProtocol::Udp,
			HostProtocol::Tcp => models::ActorsPortProtocol::Tcp,
		}
	}
}

impl ApiFrom<models::ActorsEndpointType> for EndpointType {
	fn api_from(value: models::ActorsEndpointType) -> EndpointType {
		match value {
			models::ActorsEndpointType::Hostname => EndpointType::Hostname,
			models::ActorsEndpointType::Path => EndpointType::Path,
		}
	}
}

impl ApiFrom<EndpointType> for models::ActorsEndpointType {
	fn api_from(value: EndpointType) -> models::ActorsEndpointType {
		match value {
			EndpointType::Hostname => models::ActorsEndpointType::Hostname,
			EndpointType::Path => models::ActorsEndpointType::Path,
		}
	}
}
