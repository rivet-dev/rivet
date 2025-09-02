use gas::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToClient {
	Init {
		runner_id: Id,
		last_event_idx: i64,
		metadata: ProtocolMetadata,
	},
	Commands(Vec<CommandWrapper>),
	AckEvents {
		last_event_idx: i64,
	},
}

#[signal("pegboard_to_server")]
#[derive(Debug)]
#[serde(rename_all = "snake_case")]
pub enum ToServer {
	Init {
		runner_id: Option<Id>,
		name: String,
		key: String,
		version: u32,
		total_slots: u32,

		addresses_http: Option<util::serde::HashableMap<String, RunnerAddressHttp>>,
		addresses_tcp: Option<util::serde::HashableMap<String, RunnerAddressTcp>>,
		addresses_udp: Option<util::serde::HashableMap<String, RunnerAddressUdp>>,

		last_command_idx: Option<i64>,
		prepopulate_actor_names: Option<util::serde::HashableMap<String, ActorName>>,
		metadata: Option<String>,
	},
	Events(Vec<EventWrapper>),
	AckCommands {
		last_command_idx: i64,
	},
	Stopping,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct ActorName {
	/// JSON.
	pub metadata: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolMetadata {
	pub runner_lost_threshold: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandWrapper {
	pub index: i64,
	pub inner: Command,
}

#[signal("pegboard_command")]
#[derive(Debug, Clone, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Command {
	StartActor {
		actor_id: Id,
		generation: u32,
		config: Box<ActorConfig>,
	},
	StopActor {
		actor_id: Id,
		generation: u32,
	},
}

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub struct ActorConfig {
	pub name: String,
	pub key: Option<String>,
	pub create_ts: i64,
	/// Arbitrary user-defined binary data, base64 encoded.
	pub input: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct EventWrapper {
	pub index: i64,
	pub inner: Event,
}

#[signal("pegboard_event")]
#[derive(Debug, Clone, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Event {
	ActorIntent {
		actor_id: Id,
		generation: u32,
		intent: ActorIntent,
	},
	ActorStateUpdate {
		actor_id: Id,
		generation: u32,
		state: ActorState,
	},
	ActorSetAlarm {
		actor_id: Id,
		generation: u32,
		alarm_ts: Option<i64>,
	},
}

impl Event {
	// For now, all events are actor related so they doesn't need to return an `Option`
	pub fn actor_id(&self) -> Id {
		match self {
			Event::ActorIntent { actor_id, .. } => *actor_id,
			Event::ActorStateUpdate { actor_id, .. } => *actor_id,
			Event::ActorSetAlarm { actor_id, .. } => *actor_id,
		}
	}

	// For now, all events are actor related so they doesn't need to return an `Option`
	pub fn generation(&self) -> u32 {
		match self {
			Event::ActorIntent { generation, .. } => *generation,
			Event::ActorStateUpdate { generation, .. } => *generation,
			Event::ActorSetAlarm { generation, .. } => *generation,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ActorIntent {
	/// Actor intends to sleep. This informs rivet that the actor should be stopped and can be woken up later
	// either by an alarm or guard.
	Sleep,
	/// Actor intends to stop.
	Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ActorState {
	/// Actor is running.
	Running,
	/// Actor stopped on runner.
	Stopped {
		code: StopCode,
		message: Option<String>,
	},
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StopCode {
	Ok,
	Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, utoipa::ToSchema)]
pub struct RunnerAddressHttp {
	pub hostname: String,
	pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, utoipa::ToSchema)]
pub struct RunnerAddressTcp {
	pub hostname: String,
	pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, utoipa::ToSchema)]
pub struct RunnerAddressUdp {
	pub hostname: String,
	pub port: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WebsocketCloseReason {
	Ok,
	Error,
}
