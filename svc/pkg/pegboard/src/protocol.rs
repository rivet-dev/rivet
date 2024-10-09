use chirp_workflow::prelude::*;

#[derive(thiserror::Error, Debug)]
pub enum PegboardProtocolError {
	#[error("ser/de error: {0}")]
	Serde(#[from] serde_json::Error),
}

#[signal("pegboard_forward_to_client")]
#[derive(Debug)]
pub enum ToClient {
	Init { vector_socket_addr: String },
	Commands(Vec<Command>),
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
	Init {},
	Events(Vec<Event>),
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

#[signal("pegboard_client_command")]
#[derive(Debug)]
pub enum Command {
	StartContainer {
		container_id: Uuid,
		image_artifact_url: String,
		container_runner_binary_url: String,
		root_user_enabled: bool,
		stakeholder: Stakeholder,
	},
	StopContainer {
		container_id: Uuid,
	}
}

#[derive(Debug, Serialize, Deserialize)]
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
pub enum Event {
	ContainerStateUpdate {
		container_id: Uuid,
		state: ContainerState,
	},
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum ContainerState {
	Starting,
	Running { pid: usize },
	Stopping,
	Exited { exit_code: Option<i32> },
}
