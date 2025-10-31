use anyhow::{Ok, Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::{v1, v2};

pub enum ToClient {
	V1(v1::ToClient),
	V2(v2::ToClient),
}

impl OwnedVersionedData for ToClient {
	type Latest = v2::ToClient;

	fn wrap_latest(latest: v2::ToClient) -> Self {
		ToClient::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		if let ToClient::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToClient::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(ToClient::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToClient::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ToClient::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}

impl ToClient {
	fn v1_to_v2(self) -> Result<Self> {
		match self {
			ToClient::V1(x) => {
				let inner = match x {
					v1::ToClient::ToClientInit(init) => {
						v2::ToClient::ToClientInit(v2::ToClientInit {
							runner_id: init.runner_id,
							last_event_idx: init.last_event_idx,
							metadata: v2::ProtocolMetadata {
								runner_lost_threshold: init.metadata.runner_lost_threshold,
							},
						})
					}
					v1::ToClient::ToClientClose => v2::ToClient::ToClientClose,
					v1::ToClient::ToClientCommands(commands) => v2::ToClient::ToClientCommands(
						commands
							.into_iter()
							.map(|cmd| v2::CommandWrapper {
								index: cmd.index,
								inner: match cmd.inner {
									v1::Command::CommandStartActor(start) => {
										v2::Command::CommandStartActor(v2::CommandStartActor {
											actor_id: start.actor_id,
											generation: start.generation,
											config: v2::ActorConfig {
												name: start.config.name,
												key: start.config.key,
												create_ts: start.config.create_ts,
												input: start.config.input,
											},
										})
									}
									v1::Command::CommandStopActor(stop) => {
										v2::Command::CommandStopActor(v2::CommandStopActor {
											actor_id: stop.actor_id,
											generation: stop.generation,
										})
									}
								},
							})
							.collect(),
					),
					v1::ToClient::ToClientAckEvents(ack) => {
						v2::ToClient::ToClientAckEvents(v2::ToClientAckEvents {
							last_event_idx: ack.last_event_idx,
						})
					}
					v1::ToClient::ToClientKvResponse(resp) => {
						v2::ToClient::ToClientKvResponse(v2::ToClientKvResponse {
							request_id: resp.request_id,
							data: convert_kv_response_data_v1_to_v2(resp.data),
						})
					}
					v1::ToClient::ToClientTunnelMessage(msg) => {
						v2::ToClient::ToClientTunnelMessage(v2::ToClientTunnelMessage {
							request_id: msg.request_id,
							message_id: msg.message_id,
							message_kind: convert_to_client_tunnel_message_kind_v1_to_v2(
								msg.message_kind,
							),
							gateway_reply_to: msg.gateway_reply_to,
						})
					}
				};

				Ok(ToClient::V2(inner))
			}
			value @ ToClient::V2(_) => Ok(value),
		}
	}

	fn v2_to_v1(self) -> Result<Self> {
		match self {
			ToClient::V1(_) => Ok(self),
			ToClient::V2(x) => {
				let inner = match x {
					v2::ToClient::ToClientInit(init) => {
						v1::ToClient::ToClientInit(v1::ToClientInit {
							runner_id: init.runner_id,
							last_event_idx: init.last_event_idx,
							metadata: v1::ProtocolMetadata {
								runner_lost_threshold: init.metadata.runner_lost_threshold,
							},
						})
					}
					v2::ToClient::ToClientClose => v1::ToClient::ToClientClose,
					v2::ToClient::ToClientCommands(commands) => v1::ToClient::ToClientCommands(
						commands
							.into_iter()
							.map(|cmd| v1::CommandWrapper {
								index: cmd.index,
								inner: match cmd.inner {
									v2::Command::CommandStartActor(start) => {
										v1::Command::CommandStartActor(v1::CommandStartActor {
											actor_id: start.actor_id,
											generation: start.generation,
											config: v1::ActorConfig {
												name: start.config.name,
												key: start.config.key,
												create_ts: start.config.create_ts,
												input: start.config.input,
											},
										})
									}
									v2::Command::CommandStopActor(stop) => {
										v1::Command::CommandStopActor(v1::CommandStopActor {
											actor_id: stop.actor_id,
											generation: stop.generation,
										})
									}
								},
							})
							.collect(),
					),
					v2::ToClient::ToClientAckEvents(ack) => {
						v1::ToClient::ToClientAckEvents(v1::ToClientAckEvents {
							last_event_idx: ack.last_event_idx,
						})
					}
					v2::ToClient::ToClientKvResponse(resp) => {
						v1::ToClient::ToClientKvResponse(v1::ToClientKvResponse {
							request_id: resp.request_id,
							data: convert_kv_response_data_v2_to_v1(resp.data),
						})
					}
					v2::ToClient::ToClientTunnelMessage(msg) => {
						v1::ToClient::ToClientTunnelMessage(v1::ToClientTunnelMessage {
							request_id: msg.request_id,
							message_id: msg.message_id,
							message_kind: convert_to_client_tunnel_message_kind_v2_to_v1(
								msg.message_kind,
							)?,
							gateway_reply_to: msg.gateway_reply_to,
						})
					}
				};

				Ok(ToClient::V1(inner))
			}
		}
	}
}

pub enum ToServer {
	V1(v1::ToServer),
	V2(v2::ToServer),
}

impl OwnedVersionedData for ToServer {
	type Latest = v2::ToServer;

	fn wrap_latest(latest: v2::ToServer) -> Self {
		ToServer::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		if let ToServer::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToServer::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(ToServer::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToServer::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ToServer::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}

impl ToServer {
	fn v1_to_v2(self) -> Result<Self> {
		match self {
			ToServer::V1(x) => {
				let inner = match x {
					v1::ToServer::ToServerInit(init) => {
						v2::ToServer::ToServerInit(v2::ToServerInit {
							name: init.name,
							version: init.version,
							total_slots: init.total_slots,
							last_command_idx: init.last_command_idx,
							prepopulate_actor_names: init.prepopulate_actor_names.map(|map| {
								map.into_iter()
									.map(|(k, v)| {
										(
											k,
											v2::ActorName {
												metadata: v.metadata,
											},
										)
									})
									.collect()
							}),
							metadata: init.metadata,
						})
					}
					v1::ToServer::ToServerEvents(events) => v2::ToServer::ToServerEvents(
						events
							.into_iter()
							.map(|event| v2::EventWrapper {
								index: event.index,
								inner: convert_event_v1_to_v2(event.inner),
							})
							.collect(),
					),
					v1::ToServer::ToServerAckCommands(ack) => {
						v2::ToServer::ToServerAckCommands(v2::ToServerAckCommands {
							last_command_idx: ack.last_command_idx,
						})
					}
					v1::ToServer::ToServerStopping => v2::ToServer::ToServerStopping,
					v1::ToServer::ToServerPing(ping) => {
						v2::ToServer::ToServerPing(v2::ToServerPing { ts: ping.ts })
					}
					v1::ToServer::ToServerKvRequest(req) => {
						v2::ToServer::ToServerKvRequest(v2::ToServerKvRequest {
							actor_id: req.actor_id,
							request_id: req.request_id,
							data: convert_kv_request_data_v1_to_v2(req.data),
						})
					}
					v1::ToServer::ToServerTunnelMessage(msg) => {
						v2::ToServer::ToServerTunnelMessage(v2::ToServerTunnelMessage {
							request_id: msg.request_id,
							message_id: msg.message_id,
							message_kind: convert_to_server_tunnel_message_kind_v1_to_v2(
								msg.message_kind,
							),
						})
					}
				};

				Ok(ToServer::V2(inner))
			}
			value @ ToServer::V2(_) => Ok(value),
		}
	}

	fn v2_to_v1(self) -> Result<Self> {
		match self {
			ToServer::V1(_) => Ok(self),
			ToServer::V2(x) => {
				let inner = match x {
					v2::ToServer::ToServerInit(init) => {
						v1::ToServer::ToServerInit(v1::ToServerInit {
							name: init.name,
							version: init.version,
							total_slots: init.total_slots,
							last_command_idx: init.last_command_idx,
							prepopulate_actor_names: init.prepopulate_actor_names.map(|map| {
								map.into_iter()
									.map(|(k, v)| {
										(
											k,
											v1::ActorName {
												metadata: v.metadata,
											},
										)
									})
									.collect()
							}),
							metadata: init.metadata,
						})
					}
					v2::ToServer::ToServerEvents(events) => v1::ToServer::ToServerEvents(
						events
							.into_iter()
							.map(|event| v1::EventWrapper {
								index: event.index,
								inner: convert_event_v2_to_v1(event.inner),
							})
							.collect(),
					),
					v2::ToServer::ToServerAckCommands(ack) => {
						v1::ToServer::ToServerAckCommands(v1::ToServerAckCommands {
							last_command_idx: ack.last_command_idx,
						})
					}
					v2::ToServer::ToServerStopping => v1::ToServer::ToServerStopping,
					v2::ToServer::ToServerPing(ping) => {
						v1::ToServer::ToServerPing(v1::ToServerPing { ts: ping.ts })
					}
					v2::ToServer::ToServerKvRequest(req) => {
						v1::ToServer::ToServerKvRequest(v1::ToServerKvRequest {
							actor_id: req.actor_id,
							request_id: req.request_id,
							data: convert_kv_request_data_v2_to_v1(req.data),
						})
					}
					v2::ToServer::ToServerTunnelMessage(msg) => {
						v1::ToServer::ToServerTunnelMessage(v1::ToServerTunnelMessage {
							request_id: msg.request_id,
							message_id: msg.message_id,
							message_kind: convert_to_server_tunnel_message_kind_v2_to_v1(
								msg.message_kind,
							)?,
						})
					}
				};

				Ok(ToServer::V1(inner))
			}
		}
	}
}

pub enum ToGateway {
	// No change between v1 and v2
	V2(v2::ToGateway),
}

impl OwnedVersionedData for ToGateway {
	type Latest = v2::ToGateway;

	fn wrap_latest(latest: v2::ToGateway) -> Self {
		ToGateway::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToGateway::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 | 2 => Ok(ToGateway::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToGateway::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub enum ToServerlessServer {
	// No change between v1 and v2
	V2(v2::ToServerlessServer),
}

impl OwnedVersionedData for ToServerlessServer {
	type Latest = v2::ToServerlessServer;

	fn wrap_latest(latest: v2::ToServerlessServer) -> Self {
		ToServerlessServer::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToServerlessServer::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 | 2 => Ok(ToServerlessServer::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToServerlessServer::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

// Helper conversion functions
fn convert_to_client_tunnel_message_kind_v1_to_v2(
	kind: v1::ToClientTunnelMessageKind,
) -> v2::ToClientTunnelMessageKind {
	match kind {
		v1::ToClientTunnelMessageKind::TunnelAck => v2::ToClientTunnelMessageKind::TunnelAck,
		v1::ToClientTunnelMessageKind::ToClientRequestStart(req) => {
			v2::ToClientTunnelMessageKind::ToClientRequestStart(v2::ToClientRequestStart {
				actor_id: req.actor_id,
				method: req.method,
				path: req.path,
				headers: req.headers,
				body: req.body,
				stream: req.stream,
			})
		}
		v1::ToClientTunnelMessageKind::ToClientRequestChunk(chunk) => {
			v2::ToClientTunnelMessageKind::ToClientRequestChunk(v2::ToClientRequestChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v1::ToClientTunnelMessageKind::ToClientRequestAbort => {
			v2::ToClientTunnelMessageKind::ToClientRequestAbort
		}
		v1::ToClientTunnelMessageKind::ToClientWebSocketOpen(ws) => {
			v2::ToClientTunnelMessageKind::ToClientWebSocketOpen(v2::ToClientWebSocketOpen {
				actor_id: ws.actor_id,
				path: ws.path,
				headers: ws.headers,
			})
		}
		v1::ToClientTunnelMessageKind::ToClientWebSocketMessage(msg) => {
			v2::ToClientTunnelMessageKind::ToClientWebSocketMessage(v2::ToClientWebSocketMessage {
				// Default to 0 for v1 messages (hibernation disabled by default)
				index: 0,
				data: msg.data,
				binary: msg.binary,
			})
		}
		v1::ToClientTunnelMessageKind::ToClientWebSocketClose(close) => {
			v2::ToClientTunnelMessageKind::ToClientWebSocketClose(v2::ToClientWebSocketClose {
				code: close.code,
				reason: close.reason,
			})
		}
	}
}

fn convert_to_client_tunnel_message_kind_v2_to_v1(
	kind: v2::ToClientTunnelMessageKind,
) -> Result<v1::ToClientTunnelMessageKind> {
	Ok(match kind {
		v2::ToClientTunnelMessageKind::TunnelAck => v1::ToClientTunnelMessageKind::TunnelAck,
		v2::ToClientTunnelMessageKind::ToClientRequestStart(req) => {
			v1::ToClientTunnelMessageKind::ToClientRequestStart(v1::ToClientRequestStart {
				actor_id: req.actor_id,
				method: req.method,
				path: req.path,
				headers: req.headers,
				body: req.body,
				stream: req.stream,
			})
		}
		v2::ToClientTunnelMessageKind::ToClientRequestChunk(chunk) => {
			v1::ToClientTunnelMessageKind::ToClientRequestChunk(v1::ToClientRequestChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v2::ToClientTunnelMessageKind::ToClientRequestAbort => {
			v1::ToClientTunnelMessageKind::ToClientRequestAbort
		}
		v2::ToClientTunnelMessageKind::ToClientWebSocketOpen(ws) => {
			v1::ToClientTunnelMessageKind::ToClientWebSocketOpen(v1::ToClientWebSocketOpen {
				actor_id: ws.actor_id,
				path: ws.path,
				headers: ws.headers,
			})
		}
		v2::ToClientTunnelMessageKind::ToClientWebSocketMessage(msg) => {
			v1::ToClientTunnelMessageKind::ToClientWebSocketMessage(v1::ToClientWebSocketMessage {
				data: msg.data,
				binary: msg.binary,
			})
		}
		v2::ToClientTunnelMessageKind::ToClientWebSocketClose(close) => {
			v1::ToClientTunnelMessageKind::ToClientWebSocketClose(v1::ToClientWebSocketClose {
				code: close.code,
				reason: close.reason,
			})
		}
	})
}

fn convert_to_server_tunnel_message_kind_v1_to_v2(
	kind: v1::ToServerTunnelMessageKind,
) -> v2::ToServerTunnelMessageKind {
	match kind {
		v1::ToServerTunnelMessageKind::TunnelAck => v2::ToServerTunnelMessageKind::TunnelAck,
		v1::ToServerTunnelMessageKind::ToServerResponseStart(resp) => {
			v2::ToServerTunnelMessageKind::ToServerResponseStart(v2::ToServerResponseStart {
				status: resp.status,
				headers: resp.headers,
				body: resp.body,
				stream: resp.stream,
			})
		}
		v1::ToServerTunnelMessageKind::ToServerResponseChunk(chunk) => {
			v2::ToServerTunnelMessageKind::ToServerResponseChunk(v2::ToServerResponseChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v1::ToServerTunnelMessageKind::ToServerResponseAbort => {
			v2::ToServerTunnelMessageKind::ToServerResponseAbort
		}
		v1::ToServerTunnelMessageKind::ToServerWebSocketOpen => {
			v2::ToServerTunnelMessageKind::ToServerWebSocketOpen(v2::ToServerWebSocketOpen {
				can_hibernate: false,
				last_msg_index: -1,
			})
		}
		v1::ToServerTunnelMessageKind::ToServerWebSocketMessage(msg) => {
			v2::ToServerTunnelMessageKind::ToServerWebSocketMessage(v2::ToServerWebSocketMessage {
				data: msg.data,
				binary: msg.binary,
			})
		}
		v1::ToServerTunnelMessageKind::ToServerWebSocketClose(close) => {
			v2::ToServerTunnelMessageKind::ToServerWebSocketClose(v2::ToServerWebSocketClose {
				code: close.code,
				reason: close.reason,
				retry: false,
			})
		}
	}
}

fn convert_to_server_tunnel_message_kind_v2_to_v1(
	kind: v2::ToServerTunnelMessageKind,
) -> Result<v1::ToServerTunnelMessageKind> {
	Ok(match kind {
		v2::ToServerTunnelMessageKind::TunnelAck => v1::ToServerTunnelMessageKind::TunnelAck,
		v2::ToServerTunnelMessageKind::ToServerResponseStart(resp) => {
			v1::ToServerTunnelMessageKind::ToServerResponseStart(v1::ToServerResponseStart {
				status: resp.status,
				headers: resp.headers,
				body: resp.body,
				stream: resp.stream,
			})
		}
		v2::ToServerTunnelMessageKind::ToServerResponseChunk(chunk) => {
			v1::ToServerTunnelMessageKind::ToServerResponseChunk(v1::ToServerResponseChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v2::ToServerTunnelMessageKind::ToServerResponseAbort => {
			v1::ToServerTunnelMessageKind::ToServerResponseAbort
		}
		v2::ToServerTunnelMessageKind::ToServerWebSocketOpen(_) => {
			v1::ToServerTunnelMessageKind::ToServerWebSocketOpen
		}
		v2::ToServerTunnelMessageKind::ToServerWebSocketMessage(msg) => {
			v1::ToServerTunnelMessageKind::ToServerWebSocketMessage(v1::ToServerWebSocketMessage {
				data: msg.data,
				binary: msg.binary,
			})
		}
		v2::ToServerTunnelMessageKind::ToServerWebSocketMessageAck(_) => {
			// v1 doesn't have MessageAck, this is a v2-only feature
			bail!("ToServerWebSocketMessageAck is not supported in v1");
		}
		v2::ToServerTunnelMessageKind::ToServerWebSocketClose(close) => {
			v1::ToServerTunnelMessageKind::ToServerWebSocketClose(v1::ToServerWebSocketClose {
				code: close.code,
				reason: close.reason,
			})
		}
	})
}

fn convert_event_v1_to_v2(event: v1::Event) -> v2::Event {
	match event {
		v1::Event::EventActorIntent(intent) => v2::Event::EventActorIntent(v2::EventActorIntent {
			actor_id: intent.actor_id,
			generation: intent.generation,
			intent: convert_actor_intent_v1_to_v2(intent.intent),
		}),
		v1::Event::EventActorStateUpdate(state) => {
			v2::Event::EventActorStateUpdate(v2::EventActorStateUpdate {
				actor_id: state.actor_id,
				generation: state.generation,
				state: convert_actor_state_v1_to_v2(state.state),
			})
		}
		v1::Event::EventActorSetAlarm(alarm) => {
			v2::Event::EventActorSetAlarm(v2::EventActorSetAlarm {
				actor_id: alarm.actor_id,
				generation: alarm.generation,
				alarm_ts: alarm.alarm_ts,
			})
		}
	}
}

fn convert_event_v2_to_v1(event: v2::Event) -> v1::Event {
	match event {
		v2::Event::EventActorIntent(intent) => v1::Event::EventActorIntent(v1::EventActorIntent {
			actor_id: intent.actor_id,
			generation: intent.generation,
			intent: convert_actor_intent_v2_to_v1(intent.intent),
		}),
		v2::Event::EventActorStateUpdate(state) => {
			v1::Event::EventActorStateUpdate(v1::EventActorStateUpdate {
				actor_id: state.actor_id,
				generation: state.generation,
				state: convert_actor_state_v2_to_v1(state.state),
			})
		}
		v2::Event::EventActorSetAlarm(alarm) => {
			v1::Event::EventActorSetAlarm(v1::EventActorSetAlarm {
				actor_id: alarm.actor_id,
				generation: alarm.generation,
				alarm_ts: alarm.alarm_ts,
			})
		}
	}
}

fn convert_actor_intent_v1_to_v2(intent: v1::ActorIntent) -> v2::ActorIntent {
	match intent {
		v1::ActorIntent::ActorIntentSleep => v2::ActorIntent::ActorIntentSleep,
		v1::ActorIntent::ActorIntentStop => v2::ActorIntent::ActorIntentStop,
	}
}

fn convert_actor_intent_v2_to_v1(intent: v2::ActorIntent) -> v1::ActorIntent {
	match intent {
		v2::ActorIntent::ActorIntentSleep => v1::ActorIntent::ActorIntentSleep,
		v2::ActorIntent::ActorIntentStop => v1::ActorIntent::ActorIntentStop,
	}
}

fn convert_actor_state_v1_to_v2(state: v1::ActorState) -> v2::ActorState {
	match state {
		v1::ActorState::ActorStateRunning => v2::ActorState::ActorStateRunning,
		v1::ActorState::ActorStateStopped(stopped) => {
			v2::ActorState::ActorStateStopped(v2::ActorStateStopped {
				code: convert_stop_code_v1_to_v2(stopped.code),
				message: stopped.message,
			})
		}
	}
}

fn convert_actor_state_v2_to_v1(state: v2::ActorState) -> v1::ActorState {
	match state {
		v2::ActorState::ActorStateRunning => v1::ActorState::ActorStateRunning,
		v2::ActorState::ActorStateStopped(stopped) => {
			v1::ActorState::ActorStateStopped(v1::ActorStateStopped {
				code: convert_stop_code_v2_to_v1(stopped.code),
				message: stopped.message,
			})
		}
	}
}

fn convert_stop_code_v1_to_v2(code: v1::StopCode) -> v2::StopCode {
	match code {
		v1::StopCode::Ok => v2::StopCode::Ok,
		v1::StopCode::Error => v2::StopCode::Error,
	}
}

fn convert_stop_code_v2_to_v1(code: v2::StopCode) -> v1::StopCode {
	match code {
		v2::StopCode::Ok => v1::StopCode::Ok,
		v2::StopCode::Error => v1::StopCode::Error,
	}
}

fn convert_kv_request_data_v1_to_v2(data: v1::KvRequestData) -> v2::KvRequestData {
	match data {
		v1::KvRequestData::KvGetRequest(req) => {
			v2::KvRequestData::KvGetRequest(v2::KvGetRequest { keys: req.keys })
		}
		v1::KvRequestData::KvListRequest(req) => {
			v2::KvRequestData::KvListRequest(v2::KvListRequest {
				query: convert_kv_list_query_v1_to_v2(req.query),
				reverse: req.reverse,
				limit: req.limit,
			})
		}
		v1::KvRequestData::KvPutRequest(req) => v2::KvRequestData::KvPutRequest(v2::KvPutRequest {
			keys: req.keys,
			values: req.values,
		}),
		v1::KvRequestData::KvDeleteRequest(req) => {
			v2::KvRequestData::KvDeleteRequest(v2::KvDeleteRequest { keys: req.keys })
		}
		v1::KvRequestData::KvDropRequest => v2::KvRequestData::KvDropRequest,
	}
}

fn convert_kv_request_data_v2_to_v1(data: v2::KvRequestData) -> v1::KvRequestData {
	match data {
		v2::KvRequestData::KvGetRequest(req) => {
			v1::KvRequestData::KvGetRequest(v1::KvGetRequest { keys: req.keys })
		}
		v2::KvRequestData::KvListRequest(req) => {
			v1::KvRequestData::KvListRequest(v1::KvListRequest {
				query: convert_kv_list_query_v2_to_v1(req.query),
				reverse: req.reverse,
				limit: req.limit,
			})
		}
		v2::KvRequestData::KvPutRequest(req) => v1::KvRequestData::KvPutRequest(v1::KvPutRequest {
			keys: req.keys,
			values: req.values,
		}),
		v2::KvRequestData::KvDeleteRequest(req) => {
			v1::KvRequestData::KvDeleteRequest(v1::KvDeleteRequest { keys: req.keys })
		}
		v2::KvRequestData::KvDropRequest => v1::KvRequestData::KvDropRequest,
	}
}

fn convert_kv_response_data_v1_to_v2(data: v1::KvResponseData) -> v2::KvResponseData {
	match data {
		v1::KvResponseData::KvErrorResponse(err) => {
			v2::KvResponseData::KvErrorResponse(v2::KvErrorResponse {
				message: err.message,
			})
		}
		v1::KvResponseData::KvGetResponse(resp) => {
			v2::KvResponseData::KvGetResponse(v2::KvGetResponse {
				keys: resp.keys,
				values: resp.values,
				metadata: resp
					.metadata
					.into_iter()
					.map(convert_kv_metadata_v1_to_v2)
					.collect(),
			})
		}
		v1::KvResponseData::KvListResponse(resp) => {
			v2::KvResponseData::KvListResponse(v2::KvListResponse {
				keys: resp.keys,
				values: resp.values,
				metadata: resp
					.metadata
					.into_iter()
					.map(convert_kv_metadata_v1_to_v2)
					.collect(),
			})
		}
		v1::KvResponseData::KvPutResponse => v2::KvResponseData::KvPutResponse,
		v1::KvResponseData::KvDeleteResponse => v2::KvResponseData::KvDeleteResponse,
		v1::KvResponseData::KvDropResponse => v2::KvResponseData::KvDropResponse,
	}
}

fn convert_kv_response_data_v2_to_v1(data: v2::KvResponseData) -> v1::KvResponseData {
	match data {
		v2::KvResponseData::KvErrorResponse(err) => {
			v1::KvResponseData::KvErrorResponse(v1::KvErrorResponse {
				message: err.message,
			})
		}
		v2::KvResponseData::KvGetResponse(resp) => {
			v1::KvResponseData::KvGetResponse(v1::KvGetResponse {
				keys: resp.keys,
				values: resp.values,
				metadata: resp
					.metadata
					.into_iter()
					.map(convert_kv_metadata_v2_to_v1)
					.collect(),
			})
		}
		v2::KvResponseData::KvListResponse(resp) => {
			v1::KvResponseData::KvListResponse(v1::KvListResponse {
				keys: resp.keys,
				values: resp.values,
				metadata: resp
					.metadata
					.into_iter()
					.map(convert_kv_metadata_v2_to_v1)
					.collect(),
			})
		}
		v2::KvResponseData::KvPutResponse => v1::KvResponseData::KvPutResponse,
		v2::KvResponseData::KvDeleteResponse => v1::KvResponseData::KvDeleteResponse,
		v2::KvResponseData::KvDropResponse => v1::KvResponseData::KvDropResponse,
	}
}

fn convert_kv_list_query_v1_to_v2(query: v1::KvListQuery) -> v2::KvListQuery {
	match query {
		v1::KvListQuery::KvListAllQuery => v2::KvListQuery::KvListAllQuery,
		v1::KvListQuery::KvListRangeQuery(range) => {
			v2::KvListQuery::KvListRangeQuery(v2::KvListRangeQuery {
				start: range.start,
				end: range.end,
				exclusive: range.exclusive,
			})
		}
		v1::KvListQuery::KvListPrefixQuery(prefix) => {
			v2::KvListQuery::KvListPrefixQuery(v2::KvListPrefixQuery { key: prefix.key })
		}
	}
}

fn convert_kv_list_query_v2_to_v1(query: v2::KvListQuery) -> v1::KvListQuery {
	match query {
		v2::KvListQuery::KvListAllQuery => v1::KvListQuery::KvListAllQuery,
		v2::KvListQuery::KvListRangeQuery(range) => {
			v1::KvListQuery::KvListRangeQuery(v1::KvListRangeQuery {
				start: range.start,
				end: range.end,
				exclusive: range.exclusive,
			})
		}
		v2::KvListQuery::KvListPrefixQuery(prefix) => {
			v1::KvListQuery::KvListPrefixQuery(v1::KvListPrefixQuery { key: prefix.key })
		}
	}
}

fn convert_kv_metadata_v1_to_v2(metadata: v1::KvMetadata) -> v2::KvMetadata {
	v2::KvMetadata {
		version: metadata.version,
		create_ts: metadata.create_ts,
	}
}

fn convert_kv_metadata_v2_to_v1(metadata: v2::KvMetadata) -> v1::KvMetadata {
	v1::KvMetadata {
		version: metadata.version,
		create_ts: metadata.create_ts,
	}
}
