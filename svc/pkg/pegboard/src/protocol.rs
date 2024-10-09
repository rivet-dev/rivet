use std::net::Ipv4Addr;

use chirp_workflow::prelude::*;
use strum::FromRepr;

// Reexport for ease of use in pegboard manager
pub use util::serde::{HashableMap, Raw};

#[derive(thiserror::Error, Debug)]
pub enum PegboardProtocolError {
	#[error("ser/de error: {0}")]
	Serde(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ToClient {
	Init {
		last_event_idx: i64,
		api_endpoint: String,
	},
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

#[signal("pegboard_forward_to_server")]
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

#[signal("pegboard_client_command")]
#[derive(Debug, Hash)]
pub enum Command {
	StartContainer {
		container_id: Uuid,
		config: Box<ContainerConfig>,
	},
	SignalContainer {
		container_id: Uuid,
		// See nix::sys::signal::Signal
		signal: i32,
	},
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub struct ContainerConfig {
	pub image: Image,
	pub container_runner_binary_url: String,
	pub root_user_enabled: bool,
	pub env: util::serde::HashableMap<String, String>,
	pub ports: util::serde::HashableMap<String, Port>,
	pub network_mode: NetworkMode,
	pub resources: Resources,
	pub stakeholder: Stakeholder,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub struct Image {
	pub artifact_url: String,
	pub kind: ImageKind,
	pub compression: ImageCompression,
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
	DockerImage,
	OciBundle,
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageCompression {
	None,
	Lz4,
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub enum Port {
	GameGuard {
		target: u16,
		protocol: TransportProtocol,
	},
	Host {
		protocol: TransportProtocol,
	},
}

impl Port {
	pub fn protocol(&self) -> &TransportProtocol {
		match self {
			Port::GameGuard { protocol, .. } => protocol,
			Port::Host { protocol } => protocol,
		}
	}
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
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
pub enum NetworkMode {
	Bridge,
	Host,
}

/// runc-compatible resources.
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
pub enum Stakeholder {
	DynamicServer { server_id: Uuid },
}

impl Stakeholder {
	pub fn env(&self) -> Vec<(&str, String)> {
		match self {
			Stakeholder::DynamicServer { server_id } => {
				vec![
					("PEGBOARD_META_stakeholder", "dynamic_server".to_string()),
					("PEGBOARD_META_server_id", server_id.to_string()),
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
pub enum Event {
	ContainerStateUpdate {
		container_id: Uuid,
		state: ContainerState,
	},
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum ContainerState {
	/// Container planned, not yet started.
	/// Sent by pegboard dc.
	Allocated { client_id: Uuid },
	/// Container starting on client.
	/// Sent by pegboard client.
	Starting,
	/// Container has a running process.
	/// Sent by pegboard client.
	Running {
		pid: usize,
		proxied_ports: util::serde::HashableMap<String, ProxiedPort>,
	},
	/// Container planned to stop.
	/// Sent by pegboard dc.
	Stopping,
	/// Container stopped, process exit not yet received.
	/// Sent by pegboard client and pegboard gc.
	Stopped,
	/// Container process exited.
	/// Sent by pegboard client.
	Exited { exit_code: Option<i32> },
	/// Container failed to allocate to a client.
	/// Sent by pegboard dc.
	FailedToAllocate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ProxiedPort {
	pub source: u16,
	pub target: u16,
	pub ip: Ipv4Addr,
	pub protocol: TransportProtocol,
}
