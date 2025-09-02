use anyhow::{Ok, Result, bail};
use base64::prelude::*;
use gas::prelude::*;
use versioned_data_util::OwnedVersionedData;

use crate::{PROTOCOL_VERSION, generated::v1, protocol};

pub enum ToClient {
	V1(v1::ToClient),
}

impl OwnedVersionedData for ToClient {
	type Latest = v1::ToClient;

	fn latest(latest: v1::ToClient) -> Self {
		ToClient::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToClient::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToClient::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToClient::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl ToClient {
	pub fn deserialize(buf: &[u8]) -> Result<v1::ToClient> {
		<Self as OwnedVersionedData>::deserialize(buf, PROTOCOL_VERSION)
	}
}

impl TryFrom<protocol::ToClient> for ToClient {
	type Error = anyhow::Error;

	fn try_from(value: protocol::ToClient) -> Result<Self> {
		Ok(ToClient::V1(match value {
			protocol::ToClient::Init {
				runner_id,
				last_event_idx,
				metadata,
			} => v1::ToClient::ToClientInit(v1::ToClientInit {
				runner_id: runner_id.to_string(),
				last_event_idx,
				metadata: metadata.try_into()?,
			}),
			protocol::ToClient::Commands(commands) => {
				let commands = commands
					.into_iter()
					.map(|c| c.try_into())
					.collect::<Result<_>>()?;

				v1::ToClient::ToClientCommands(commands)
			}
			protocol::ToClient::AckEvents { last_event_idx } => {
				v1::ToClient::ToClientAckEvents(v1::ToClientAckEvents { last_event_idx })
			}
		}))
	}
}

impl TryFrom<protocol::ProtocolMetadata> for v1::ProtocolMetadata {
	type Error = anyhow::Error;

	fn try_from(value: protocol::ProtocolMetadata) -> Result<Self> {
		Ok(v1::ProtocolMetadata {
			runner_lost_threshold: value.runner_lost_threshold,
		})
	}
}

impl TryFrom<protocol::CommandWrapper> for v1::CommandWrapper {
	type Error = anyhow::Error;

	fn try_from(value: protocol::CommandWrapper) -> Result<Self> {
		Ok(v1::CommandWrapper {
			index: value.index,
			inner: value.inner.try_into()?,
		})
	}
}

impl TryFrom<protocol::Command> for v1::Command {
	type Error = anyhow::Error;

	fn try_from(value: protocol::Command) -> Result<Self> {
		match value {
			protocol::Command::StartActor {
				actor_id,
				generation,
				config,
			} => Ok(v1::Command::CommandStartActor(v1::CommandStartActor {
				actor_id: actor_id.to_string(),
				generation,
				config: (*config).try_into()?,
			})),
			protocol::Command::StopActor {
				actor_id,
				generation,
			} => Ok(v1::Command::CommandStopActor(v1::CommandStopActor {
				actor_id: actor_id.to_string(),
				generation,
			})),
		}
	}
}

impl TryFrom<protocol::ActorConfig> for v1::ActorConfig {
	type Error = anyhow::Error;

	fn try_from(value: protocol::ActorConfig) -> Result<Self> {
		Ok(v1::ActorConfig {
			name: value.name,
			key: value.key,
			create_ts: value.create_ts,
			input: value.input.map(|x| BASE64_STANDARD.decode(x)).transpose()?,
		})
	}
}

pub enum ToServer {
	V1(v1::ToServer),
}

impl OwnedVersionedData for ToServer {
	type Latest = v1::ToServer;

	fn latest(latest: v1::ToServer) -> Self {
		ToServer::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToServer::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToServer::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToServer::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl ToServer {
	pub fn serialize(self) -> Result<Vec<u8>> {
		<Self as OwnedVersionedData>::serialize(self, PROTOCOL_VERSION)
	}
}

impl From<v1::ActorName> for protocol::ActorName {
	fn from(value: v1::ActorName) -> Self {
		protocol::ActorName {
			metadata: value.metadata,
		}
	}
}

impl TryFrom<v1::EventWrapper> for protocol::EventWrapper {
	type Error = anyhow::Error;

	fn try_from(value: v1::EventWrapper) -> Result<Self> {
		Ok(protocol::EventWrapper {
			index: value.index,
			inner: value.inner.try_into()?,
		})
	}
}

impl TryFrom<v1::Event> for protocol::Event {
	type Error = anyhow::Error;

	fn try_from(value: v1::Event) -> Result<Self> {
		match value {
			v1::Event::EventActorIntent(event) => Ok(protocol::Event::ActorIntent {
				actor_id: util::Id::parse(&event.actor_id)?,
				generation: event.generation,
				intent: event.intent.try_into()?,
			}),
			v1::Event::EventActorStateUpdate(event) => Ok(protocol::Event::ActorStateUpdate {
				actor_id: util::Id::parse(&event.actor_id)?,
				generation: event.generation,
				state: event.state.try_into()?,
			}),
			v1::Event::EventActorSetAlarm(event) => Ok(protocol::Event::ActorSetAlarm {
				actor_id: util::Id::parse(&event.actor_id)?,
				generation: event.generation,
				alarm_ts: event.alarm_ts,
			}),
		}
	}
}

impl TryFrom<v1::ActorIntent> for protocol::ActorIntent {
	type Error = anyhow::Error;

	fn try_from(value: v1::ActorIntent) -> Result<Self> {
		match value {
			v1::ActorIntent::ActorIntentSleep => Ok(protocol::ActorIntent::Sleep),
			v1::ActorIntent::ActorIntentStop => Ok(protocol::ActorIntent::Stop),
		}
	}
}

impl TryFrom<v1::ActorState> for protocol::ActorState {
	type Error = anyhow::Error;

	fn try_from(value: v1::ActorState) -> Result<Self> {
		match value {
			v1::ActorState::ActorStateRunning => Ok(protocol::ActorState::Running),
			v1::ActorState::ActorStateStopped(stopped) => Ok(protocol::ActorState::Stopped {
				code: stopped.code.try_into()?,
				message: stopped.message,
			}),
		}
	}
}

impl TryFrom<v1::StopCode> for protocol::StopCode {
	type Error = anyhow::Error;

	fn try_from(value: v1::StopCode) -> Result<Self> {
		match value {
			v1::StopCode::Ok => Ok(protocol::StopCode::Ok),
			v1::StopCode::Error => Ok(protocol::StopCode::Error),
		}
	}
}

impl From<v1::RunnerAddressHttp> for protocol::RunnerAddressHttp {
	fn from(value: v1::RunnerAddressHttp) -> Self {
		protocol::RunnerAddressHttp {
			hostname: value.hostname,
			port: value.port,
		}
	}
}

impl From<v1::RunnerAddressTcp> for protocol::RunnerAddressTcp {
	fn from(value: v1::RunnerAddressTcp) -> Self {
		protocol::RunnerAddressTcp {
			hostname: value.hostname,
			port: value.port,
		}
	}
}

impl From<v1::RunnerAddressUdp> for protocol::RunnerAddressUdp {
	fn from(value: v1::RunnerAddressUdp) -> Self {
		protocol::RunnerAddressUdp {
			hostname: value.hostname,
			port: value.port,
		}
	}
}

impl TryFrom<v1::ToServer> for protocol::ToServer {
	type Error = anyhow::Error;

	fn try_from(value: v1::ToServer) -> Result<Self> {
		match value {
			v1::ToServer::ToServerInit(init) => Ok(protocol::ToServer::Init {
				runner_id: init.runner_id.map(|id| util::Id::parse(&id)).transpose()?,
				name: init.name,
				key: init.key,
				version: init.version,
				total_slots: init.total_slots,
				addresses_http: init
					.addresses_http
					.map(|addrs| Ok(addrs.into_iter().map(|(k, v)| (k, v.into())).collect()))
					.transpose()?,
				addresses_tcp: init
					.addresses_tcp
					.map(|addrs| Ok(addrs.into_iter().map(|(k, v)| (k, v.into())).collect()))
					.transpose()?,
				addresses_udp: init
					.addresses_udp
					.map(|addrs| Ok(addrs.into_iter().map(|(k, v)| (k, v.into())).collect()))
					.transpose()?,
				last_command_idx: init.last_command_idx,
				prepopulate_actor_names: init
					.prepopulate_actor_names
					.map(|x| x.into_iter().map(|(k, v)| (k, v.into())).collect()),
				metadata: init.metadata,
			}),
			v1::ToServer::ToServerEvents(events) => Ok(protocol::ToServer::Events(
				events
					.into_iter()
					.map(|e| e.try_into())
					.collect::<Result<_>>()?,
			)),
			v1::ToServer::ToServerAckCommands(ack) => Ok(protocol::ToServer::AckCommands {
				last_command_idx: ack.last_command_idx,
			}),
			v1::ToServer::ToServerStopping => Ok(protocol::ToServer::Stopping),
			v1::ToServer::ToServerPing(_) => {
				// NOTE: Ping is handled at the websocket level and never reaches the workflow.
				bail!("Ping variant should not be converted")
			}
			v1::ToServer::ToServerKvRequest(_) => {
				// NOTE: KV is handled at the websocket level and never reaches the workflow.
				bail!("KV variant should not be converted")
			}
		}
	}
}
