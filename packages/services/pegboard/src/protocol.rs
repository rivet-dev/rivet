use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use strum::FromRepr;
use uuid::Uuid;

#[cfg(feature = "chirp")]
use chirp_workflow::prelude::*;

// Reexport for ease of use in pegboard manager
pub use ::util::serde::{HashableMap, Raw};

#[derive(thiserror::Error, Debug)]
pub enum PegboardProtocolError {
	#[error("ser/de error: {0}")]
	Serde(#[from] serde_json::Error),
	#[error("invalid client flavor: {0}")]
	InvalidClientFlavor(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToClient {
	Init { last_event_idx: i64 },
	Commands(Vec<CommandWrapper>),
	FetchStateRequest {},
}

impl ToClient {
	pub fn serialize(&self, _protocol_version: u16) -> Result<Vec<u8>, PegboardProtocolError> {
		serde_json::to_vec(&self).map_err(PegboardProtocolError::Serde)
	}

	pub fn deserialize(buf: &[u8]) -> Result<Self, PegboardProtocolError> {
		serde_json::from_slice(buf).map_err(PegboardProtocolError::Serde)
	}
}

#[cfg_attr(feature = "chirp", signal("pegboard_forward_to_server"))]
#[cfg_attr(not(feature = "chirp"), derive(Serialize, Deserialize))]
#[derive(Debug)]
#[serde(rename_all = "snake_case")]
pub enum ToServer {
	Init {
		last_command_idx: i64,
		system: SystemInfo,
	},
	Events(Vec<EventWrapper>),
	FetchStateResponse {},
}

impl ToServer {
	pub fn serialize(&self) -> Result<Vec<u8>, PegboardProtocolError> {
		serde_json::to_vec(&self).map_err(PegboardProtocolError::Serde)
	}

	pub fn deserialize(_protocol_version: u16, buf: &[u8]) -> Result<Self, PegboardProtocolError> {
		serde_json::from_slice(buf).map_err(PegboardProtocolError::Serde)
	}
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct SystemInfo {
	// MHz
	pub cpu: u64,
	// MiB
	pub memory: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandWrapper {
	pub index: i64,
	pub inner: Raw<Command>,
}

#[cfg_attr(feature = "chirp", signal("pegboard_client_command"))]
#[cfg_attr(not(feature = "chirp"), derive(Serialize, Deserialize))]
#[derive(Debug, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Command {
	StartActor {
		actor_id: Uuid,
		config: Box<ActorConfig>,
	},
	SignalActor {
		actor_id: Uuid,
		// See nix::sys::signal::Signal
		signal: i32,
	},
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub struct ActorConfig {
	// pub driver: Driver,
	pub image: Image,
	pub root_user_enabled: bool,
	pub resources: Resources,
	pub env: ::util::serde::HashableMap<String, String>,
	pub ports: ::util::serde::HashableMap<String, Port>,
	pub network_mode: NetworkMode,
	pub stakeholder: Stakeholder,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub struct Image {
	pub artifact_url: String,
	pub kind: ImageKind,
	pub compression: ImageCompression,
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageKind {
	DockerImage,
	OciBundle,
	JavaScript,
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageCompression {
	None,
	Lz4,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
#[serde(rename_all = "snake_case")]
pub struct Port {
	/// Must be set with bridge networking, unset with host networking.
	pub target: Option<u16>,
	pub protocol: TransportProtocol,
	pub routing: PortRouting,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PortRouting {
	GameGuard,
	Host,
}

#[derive(
	Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, FromRepr,
)]
#[serde(rename_all = "snake_case")]
pub enum TransportProtocol {
	Tcp = 0,
	Udp = 1,
}

impl std::fmt::Display for TransportProtocol {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			TransportProtocol::Tcp => write!(f, "tcp"),
			TransportProtocol::Udp => write!(f, "udp"),
		}
	}
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
	Bridge,
	Host,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub struct Resources {
	/// Millicore (1/1000 of a core).
	pub cpu: u64,
	// Bytes.
	pub memory: u64,
	// Bytes.
	pub memory_max: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Stakeholder {
	DynamicServer { server_id: Uuid },
}

impl Stakeholder {
	pub fn env(&self) -> Vec<(&str, String)> {
		match self {
			Stakeholder::DynamicServer { server_id } => {
				vec![
					("STAKEHOLDER", "dynamic_server".to_string()),
					("SERVER_ID", server_id.to_string()),
				]
			}
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct EventWrapper {
	pub index: i64,
	pub inner: Raw<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Event {
	ActorStateUpdate { actor_id: Uuid, state: ActorState },
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ActorState {
	/// Actor planned, not yet started.
	/// Sent by pegboard dc.
	Allocated { client_id: Uuid },
	/// Actor starting on client.
	/// Sent by pegboard client.
	Starting,
	/// Actor has a running process.
	/// Sent by pegboard client.
	Running {
		pid: usize,
		ports: ::util::serde::HashableMap<String, ProxiedPort>,
	},
	/// Actor planned to stop.
	/// Sent by pegboard dc.
	Stopping,
	/// Actor stopped, process exit not yet received.
	/// Sent by pegboard client and pegboard gc.
	Stopped,
	/// Actor process exited.
	/// Sent by pegboard client.
	Exited { exit_code: Option<i32> },
	/// Actor failed to allocate to a client.
	/// Sent by pegboard dc.
	FailedToAllocate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ProxiedPort {
	/// Port on the host.
	pub source: u16,
	/// Port in the container.
	pub target: u16,
	/// Vlan IP of the node running the container.
	pub ip: Ipv4Addr,
	pub protocol: TransportProtocol,
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[serde(rename_all = "snake_case")]
pub enum ClientFlavor {
	Container = 0,
	Isolate = 1,
}

impl std::fmt::Display for ClientFlavor {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ClientFlavor::Container => write!(f, "container"),
			ClientFlavor::Isolate => write!(f, "isolate"),
		}
	}
}

impl std::str::FromStr for ClientFlavor {
	type Err = PegboardProtocolError;

	fn from_str(s: &str) -> Result<Self, PegboardProtocolError> {
		match s {
			"container" => Ok(ClientFlavor::Container),
			"isolate" => Ok(ClientFlavor::Isolate),
			x => Err(PegboardProtocolError::InvalidClientFlavor(x.to_string())),
		}
	}
}
