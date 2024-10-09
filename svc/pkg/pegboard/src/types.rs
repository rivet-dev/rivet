use chirp_workflow::prelude::*;

#[message("pegboard_command")]
#[signal("pegboard_command")]
pub struct Command {
	pub client_id: Uuid,
	pub inner: CommandInner,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CommandInner {
	StartContainer {},
}

impl CommandInner {
	pub fn serialize(&self) -> GlobalResult<Vec<u8>> {
		serde_json::to_vec(&self).map_err(Into::into)
	}
}

#[signal("pegboard_client_event")]
#[derive(Clone, Debug, Hash)]
pub enum ClientEvent {
	Init { },
	ContainerStateUpdate {
		container_id: Uuid,
		state: ContainerState,
	},
}

impl ClientEvent {
	pub fn serialize(&self) -> GlobalResult<Vec<u8>> {
		serde_json::to_vec(&self).map_err(Into::into)
	}

	pub fn deserialize(_protocol_version: u16, buf: &[u8]) -> GlobalResult<Self> {
		serde_json::from_slice(buf).map_err(Into::into)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum ContainerState {
	Starting,
	Running,
	Stopping,
	Exited { exit_code: Option<u16> },
}
