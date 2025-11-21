use anyhow::{Ok, Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::{v1, v2, v3};

pub enum ToClient {
	V1(v1::ToClient),
	V2(v2::ToClient),
	V3(v3::ToClient),
}

impl OwnedVersionedData for ToClient {
	type Latest = v3::ToClient;

	fn wrap_latest(latest: v3::ToClient) -> Self {
		ToClient::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		if let ToClient::V3(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToClient::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(ToClient::V2(serde_bare::from_slice(payload)?)),
			3 => Ok(ToClient::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToClient::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ToClient::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ToClient::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2, Self::v2_to_v3]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v3_to_v2, Self::v2_to_v1]
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
			_ => bail!("unexpected version"),
		}
	}

	fn v2_to_v3(self) -> Result<Self> {
		if let ToClient::V2(x) = self {
			let inner = match x {
				v2::ToClient::ToClientInit(init) => v3::ToClient::ToClientInit(v3::ToClientInit {
					runner_id: init.runner_id,
					last_event_idx: init.last_event_idx,
					metadata: v3::ProtocolMetadata {
						runner_lost_threshold: init.metadata.runner_lost_threshold,
					},
				}),
				v2::ToClient::ToClientClose => v3::ToClient::ToClientClose,
				v2::ToClient::ToClientCommands(commands) => v3::ToClient::ToClientCommands(
					commands
						.into_iter()
						.map(|cmd| v3::CommandWrapper {
							index: cmd.index,
							inner: match cmd.inner {
								v2::Command::CommandStartActor(start) => {
									v3::Command::CommandStartActor(v3::CommandStartActor {
										actor_id: start.actor_id,
										generation: start.generation,
										config: v3::ActorConfig {
											name: start.config.name,
											key: start.config.key,
											create_ts: start.config.create_ts,
											input: start.config.input,
										},
										hibernating_requests: Vec::new(),
									})
								}
								v2::Command::CommandStopActor(stop) => {
									v3::Command::CommandStopActor(v3::CommandStopActor {
										actor_id: stop.actor_id,
										generation: stop.generation,
									})
								}
							},
						})
						.collect(),
				),
				v2::ToClient::ToClientAckEvents(ack) => {
					v3::ToClient::ToClientAckEvents(v3::ToClientAckEvents {
						last_event_idx: ack.last_event_idx,
					})
				}
				v2::ToClient::ToClientKvResponse(resp) => {
					v3::ToClient::ToClientKvResponse(v3::ToClientKvResponse {
						request_id: resp.request_id,
						data: convert_kv_response_data_v2_to_v3(resp.data),
					})
				}
				v2::ToClient::ToClientTunnelMessage(msg) => {
					// Extract v3 message_id from v2's message_id
					// v3: gateway_id (4) + request_id (4) + message_index (2) = 10 bytes
					// v2.message_id contains: entire v3 message_id (10 bytes) + padding (6 bytes)
					let mut gateway_id = [0u8; 4];
					gateway_id.copy_from_slice(&msg.message_id[..4]);
					let mut request_id = [0u8; 4];
					request_id.copy_from_slice(&msg.request_id[..4]);

					v3::ToClient::ToClientTunnelMessage(v3::ToClientTunnelMessage {
						message_id: v3::MessageId {
							gateway_id,
							request_id,
							message_index: 0,
						},
						message_kind: convert_to_client_tunnel_message_kind_v2_to_v3(
							msg.message_kind,
						),
					})
				}
			};

			Ok(ToClient::V3(inner))
		} else {
			bail!("unexpected version");
		}
	}

	fn v3_to_v2(self) -> Result<Self> {
		if let ToClient::V3(x) = self {
			let inner = match x {
				v3::ToClient::ToClientInit(init) => v2::ToClient::ToClientInit(v2::ToClientInit {
					runner_id: init.runner_id,
					last_event_idx: init.last_event_idx,
					metadata: v2::ProtocolMetadata {
						runner_lost_threshold: init.metadata.runner_lost_threshold,
					},
				}),
				v3::ToClient::ToClientClose => v2::ToClient::ToClientClose,
				v3::ToClient::ToClientCommands(commands) => v2::ToClient::ToClientCommands(
					commands
						.into_iter()
						.map(|cmd| v2::CommandWrapper {
							index: cmd.index,
							inner: match cmd.inner {
								v3::Command::CommandStartActor(start) => {
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
								v3::Command::CommandStopActor(stop) => {
									v2::Command::CommandStopActor(v2::CommandStopActor {
										actor_id: stop.actor_id,
										generation: stop.generation,
									})
								}
							},
						})
						.collect(),
				),
				v3::ToClient::ToClientAckEvents(ack) => {
					v2::ToClient::ToClientAckEvents(v2::ToClientAckEvents {
						last_event_idx: ack.last_event_idx,
					})
				}
				v3::ToClient::ToClientKvResponse(resp) => {
					v2::ToClient::ToClientKvResponse(v2::ToClientKvResponse {
						request_id: resp.request_id,
						data: convert_kv_response_data_v3_to_v2(resp.data),
					})
				}
				v3::ToClient::ToClientTunnelMessage(msg) => {
					// Split v3 message_id into v2's request_id and message_id
					// v3: gateway_id (4) + request_id (4) + message_index (2) = 10 bytes
					// v2.request_id = gateway_id (4) + request_id (4) + padding (8 zeros)
					// v2.message_id = entire v3 message_id (10 bytes) + padding (4 zeros)
					let mut request_id = [0u8; 16];
					let mut message_id = [0u8; 16];
					request_id[..4].copy_from_slice(&msg.message_id.gateway_id);
					request_id[4..8].copy_from_slice(&msg.message_id.request_id);
					message_id[..8].copy_from_slice(&request_id[0..8]);
					request_id[8..10].copy_from_slice(&msg.message_id.message_index.to_le_bytes());

					v2::ToClient::ToClientTunnelMessage(v2::ToClientTunnelMessage {
						request_id,
						message_id,
						message_kind: convert_to_client_tunnel_message_kind_v3_to_v2(
							msg.message_kind,
							&msg.message_id,
						)?,
						gateway_reply_to: None,
					})
				}
			};

			Ok(ToClient::V2(inner))
		} else {
			bail!("unexpected version");
		}
	}

	fn v2_to_v1(self) -> Result<Self> {
		if let ToClient::V2(x) = self {
			let inner = match x {
				v2::ToClient::ToClientInit(init) => v1::ToClient::ToClientInit(v1::ToClientInit {
					runner_id: init.runner_id,
					last_event_idx: init.last_event_idx,
					metadata: v1::ProtocolMetadata {
						runner_lost_threshold: init.metadata.runner_lost_threshold,
					},
				}),
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
		} else {
			bail!("unexpected version");
		}
	}
}

pub enum ToServer {
	V1(v1::ToServer),
	V2(v2::ToServer),
	V3(v3::ToServer),
}

impl OwnedVersionedData for ToServer {
	type Latest = v3::ToServer;

	fn wrap_latest(latest: v3::ToServer) -> Self {
		ToServer::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		if let ToServer::V3(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToServer::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(ToServer::V2(serde_bare::from_slice(payload)?)),
			3 => Ok(ToServer::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToServer::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ToServer::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ToServer::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2, Self::v2_to_v3]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v3_to_v2, Self::v2_to_v1]
	}
}

impl ToServer {
	fn v1_to_v2(self) -> Result<Self> {
		if let ToServer::V1(x) = self {
			let inner = match x {
				v1::ToServer::ToServerInit(init) => v2::ToServer::ToServerInit(v2::ToServerInit {
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
				}),
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
		} else {
			bail!("unexpected version");
		}
	}

	fn v2_to_v3(self) -> Result<Self> {
		if let ToServer::V2(x) = self {
			let inner = match x {
				v2::ToServer::ToServerInit(init) => v3::ToServer::ToServerInit(v3::ToServerInit {
					name: init.name,
					version: init.version,
					total_slots: init.total_slots,
					last_command_idx: init.last_command_idx,
					prepopulate_actor_names: init.prepopulate_actor_names.map(|map| {
						map.into_iter()
							.map(|(k, v)| {
								(
									k,
									v3::ActorName {
										metadata: v.metadata,
									},
								)
							})
							.collect()
					}),
					metadata: init.metadata,
				}),
				v2::ToServer::ToServerEvents(events) => v3::ToServer::ToServerEvents(
					events
						.into_iter()
						.map(|event| v3::EventWrapper {
							index: event.index,
							inner: convert_event_v2_to_v3(event.inner),
						})
						.collect(),
				),
				v2::ToServer::ToServerAckCommands(ack) => {
					v3::ToServer::ToServerAckCommands(v3::ToServerAckCommands {
						last_command_idx: ack.last_command_idx,
					})
				}
				v2::ToServer::ToServerStopping => v3::ToServer::ToServerStopping,
				v2::ToServer::ToServerPing(ping) => {
					v3::ToServer::ToServerPing(v3::ToServerPing { ts: ping.ts })
				}
				v2::ToServer::ToServerKvRequest(req) => {
					v3::ToServer::ToServerKvRequest(v3::ToServerKvRequest {
						actor_id: req.actor_id,
						request_id: req.request_id,
						data: convert_kv_request_data_v2_to_v3(req.data),
					})
				}
				v2::ToServer::ToServerTunnelMessage(msg) => {
					// Extract v3 message_id from v2's message_id
					// v3: gateway_id (4) + request_id (4) + message_index (2) = 10 bytes
					// v2.message_id contains: entire v3 message_id (10 bytes) + padding (6 bytes)
					let mut gateway_id = [0u8; 4];
					gateway_id.copy_from_slice(&msg.message_id[..4]);
					let mut request_id = [0u8; 4];
					request_id.copy_from_slice(&msg.request_id[..4]);

					v3::ToServer::ToServerTunnelMessage(v3::ToServerTunnelMessage {
						message_id: v3::MessageId {
							gateway_id,
							request_id,
							message_index: 0,
						},
						message_kind: convert_to_server_tunnel_message_kind_v2_to_v3(
							msg.message_kind,
						),
					})
				}
			};

			Ok(ToServer::V3(inner))
		} else {
			bail!("unexpected version");
		}
	}

	fn v3_to_v2(self) -> Result<Self> {
		if let ToServer::V3(x) = self {
			let inner = match x {
				v3::ToServer::ToServerInit(init) => v2::ToServer::ToServerInit(v2::ToServerInit {
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
				}),
				v3::ToServer::ToServerEvents(events) => v2::ToServer::ToServerEvents(
					events
						.into_iter()
						.map(|event| v2::EventWrapper {
							index: event.index,
							inner: convert_event_v3_to_v2(event.inner),
						})
						.collect(),
				),
				v3::ToServer::ToServerAckCommands(ack) => {
					v2::ToServer::ToServerAckCommands(v2::ToServerAckCommands {
						last_command_idx: ack.last_command_idx,
					})
				}
				v3::ToServer::ToServerStopping => v2::ToServer::ToServerStopping,
				v3::ToServer::ToServerPing(ping) => {
					v2::ToServer::ToServerPing(v2::ToServerPing { ts: ping.ts })
				}
				v3::ToServer::ToServerKvRequest(req) => {
					v2::ToServer::ToServerKvRequest(v2::ToServerKvRequest {
						actor_id: req.actor_id,
						request_id: req.request_id,
						data: convert_kv_request_data_v3_to_v2(req.data),
					})
				}
				v3::ToServer::ToServerTunnelMessage(msg) => {
					// Split v3 message_id into v2's request_id and message_id
					// v3: gateway_id (4) + request_id (4) + message_index (2) = 10 bytes
					// v2.request_id = gateway_id (4) + request_id (4) + padding (8 zeros)
					// v2.message_id = entire v3 message_id (10 bytes) + padding (4 zeros)
					let mut request_id = [0u8; 16];
					let mut message_id = [0u8; 16];
					request_id[..4].copy_from_slice(&msg.message_id.gateway_id);
					request_id[4..8].copy_from_slice(&msg.message_id.request_id);
					message_id[..8].copy_from_slice(&request_id[0..8]);
					request_id[8..10].copy_from_slice(&msg.message_id.message_index.to_le_bytes());

					v2::ToServer::ToServerTunnelMessage(v2::ToServerTunnelMessage {
						request_id,
						message_id,
						message_kind: convert_to_server_tunnel_message_kind_v3_to_v2(
							msg.message_kind,
						)?,
					})
				}
			};

			Ok(ToServer::V2(inner))
		} else {
			bail!("unexpected version");
		}
	}

	fn v2_to_v1(self) -> Result<Self> {
		if let ToServer::V2(x) = self {
			let inner = match x {
				v2::ToServer::ToServerInit(init) => v1::ToServer::ToServerInit(v1::ToServerInit {
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
				}),
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
		} else {
			bail!("unexpected version");
		}
	}
}

pub enum ToRunner {
	// Only in v3
	V3(v3::ToRunner),
}

impl OwnedVersionedData for ToRunner {
	type Latest = v3::ToRunner;

	fn wrap_latest(latest: v3::ToRunner) -> Self {
		ToRunner::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToRunner::V3(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 | 2 | 3 => Ok(ToRunner::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToRunner::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		// No changes between v1 and v3
		vec![Ok, Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		// No changes between v1 and v3
		vec![Ok, Ok]
	}
}

pub enum ToGateway {
	// No change between v1 and v3
	V3(v3::ToGateway),
}

impl OwnedVersionedData for ToGateway {
	type Latest = v3::ToGateway;

	fn wrap_latest(latest: v3::ToGateway) -> Self {
		ToGateway::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToGateway::V3(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 | 2 | 3 => Ok(ToGateway::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToGateway::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		// No changes between v1 and v3
		vec![Ok, Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		// No changes between v1 and v3
		vec![Ok, Ok]
	}
}

pub enum ToServerlessServer {
	// No change between v1 and v3
	V3(v3::ToServerlessServer),
}

impl OwnedVersionedData for ToServerlessServer {
	type Latest = v3::ToServerlessServer;

	fn wrap_latest(latest: v3::ToServerlessServer) -> Self {
		ToServerlessServer::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToServerlessServer::V3(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 | 2 | 3 => Ok(ToServerlessServer::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToServerlessServer::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		// No changes between v1 and v3
		vec![Ok, Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		// No changes between v1 and v3
		vec![Ok, Ok]
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

fn convert_to_client_tunnel_message_kind_v2_to_v3(
	kind: v2::ToClientTunnelMessageKind,
) -> v3::ToClientTunnelMessageKind {
	match kind {
		v2::ToClientTunnelMessageKind::ToClientRequestStart(req) => {
			v3::ToClientTunnelMessageKind::ToClientRequestStart(v3::ToClientRequestStart {
				actor_id: req.actor_id,
				method: req.method,
				path: req.path,
				headers: req.headers,
				body: req.body,
				stream: req.stream,
			})
		}
		v2::ToClientTunnelMessageKind::ToClientRequestChunk(chunk) => {
			v3::ToClientTunnelMessageKind::ToClientRequestChunk(v3::ToClientRequestChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v2::ToClientTunnelMessageKind::ToClientRequestAbort => {
			v3::ToClientTunnelMessageKind::ToClientRequestAbort
		}
		v2::ToClientTunnelMessageKind::ToClientWebSocketOpen(ws) => {
			v3::ToClientTunnelMessageKind::ToClientWebSocketOpen(v3::ToClientWebSocketOpen {
				actor_id: ws.actor_id,
				path: ws.path,
				headers: ws.headers,
			})
		}
		v2::ToClientTunnelMessageKind::ToClientWebSocketMessage(msg) => {
			v3::ToClientTunnelMessageKind::ToClientWebSocketMessage(v3::ToClientWebSocketMessage {
				data: msg.data,
				binary: msg.binary,
			})
		}
		v2::ToClientTunnelMessageKind::ToClientWebSocketClose(close) => {
			v3::ToClientTunnelMessageKind::ToClientWebSocketClose(v3::ToClientWebSocketClose {
				code: close.code,
				reason: close.reason,
			})
		}
		// DeprecatedTunnelAck is kept for backwards compatibility
		v2::ToClientTunnelMessageKind::TunnelAck => {
			v3::ToClientTunnelMessageKind::DeprecatedTunnelAck
		}
	}
}

fn convert_to_client_tunnel_message_kind_v3_to_v2(
	kind: v3::ToClientTunnelMessageKind,
	message_id: &v3::MessageId,
) -> Result<v2::ToClientTunnelMessageKind> {
	Ok(match kind {
		v3::ToClientTunnelMessageKind::ToClientRequestStart(req) => {
			v2::ToClientTunnelMessageKind::ToClientRequestStart(v2::ToClientRequestStart {
				actor_id: req.actor_id,
				method: req.method,
				path: req.path,
				headers: req.headers,
				body: req.body,
				stream: req.stream,
			})
		}
		v3::ToClientTunnelMessageKind::ToClientRequestChunk(chunk) => {
			v2::ToClientTunnelMessageKind::ToClientRequestChunk(v2::ToClientRequestChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v3::ToClientTunnelMessageKind::ToClientRequestAbort => {
			v2::ToClientTunnelMessageKind::ToClientRequestAbort
		}
		v3::ToClientTunnelMessageKind::ToClientWebSocketOpen(ws) => {
			v2::ToClientTunnelMessageKind::ToClientWebSocketOpen(v2::ToClientWebSocketOpen {
				actor_id: ws.actor_id,
				path: ws.path,
				headers: ws.headers,
			})
		}
		v3::ToClientTunnelMessageKind::ToClientWebSocketMessage(msg) => {
			v2::ToClientTunnelMessageKind::ToClientWebSocketMessage(v2::ToClientWebSocketMessage {
				data: msg.data,
				binary: msg.binary,
				index: message_id.message_index,
			})
		}
		v3::ToClientTunnelMessageKind::ToClientWebSocketClose(close) => {
			v2::ToClientTunnelMessageKind::ToClientWebSocketClose(v2::ToClientWebSocketClose {
				code: close.code,
				reason: close.reason,
			})
		}
		v3::ToClientTunnelMessageKind::DeprecatedTunnelAck => {
			v2::ToClientTunnelMessageKind::TunnelAck
		}
	})
}

fn convert_to_server_tunnel_message_kind_v2_to_v3(
	kind: v2::ToServerTunnelMessageKind,
) -> v3::ToServerTunnelMessageKind {
	match kind {
		v2::ToServerTunnelMessageKind::ToServerResponseStart(resp) => {
			v3::ToServerTunnelMessageKind::ToServerResponseStart(v3::ToServerResponseStart {
				status: resp.status,
				headers: resp.headers,
				body: resp.body,
				stream: resp.stream,
			})
		}
		v2::ToServerTunnelMessageKind::ToServerResponseChunk(chunk) => {
			v3::ToServerTunnelMessageKind::ToServerResponseChunk(v3::ToServerResponseChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v2::ToServerTunnelMessageKind::ToServerResponseAbort => {
			v3::ToServerTunnelMessageKind::ToServerResponseAbort
		}
		v2::ToServerTunnelMessageKind::ToServerWebSocketOpen(open) => {
			v3::ToServerTunnelMessageKind::ToServerWebSocketOpen(v3::ToServerWebSocketOpen {
				can_hibernate: open.can_hibernate,
			})
		}
		v2::ToServerTunnelMessageKind::ToServerWebSocketMessage(msg) => {
			v3::ToServerTunnelMessageKind::ToServerWebSocketMessage(v3::ToServerWebSocketMessage {
				data: msg.data,
				binary: msg.binary,
			})
		}
		v2::ToServerTunnelMessageKind::ToServerWebSocketMessageAck(ack) => {
			v3::ToServerTunnelMessageKind::ToServerWebSocketMessageAck(
				v3::ToServerWebSocketMessageAck { index: ack.index },
			)
		}
		v2::ToServerTunnelMessageKind::ToServerWebSocketClose(close) => {
			v3::ToServerTunnelMessageKind::ToServerWebSocketClose(v3::ToServerWebSocketClose {
				code: close.code,
				reason: close.reason,
				hibernate: close.retry,
			})
		}
		// DeprecatedTunnelAck is kept for backwards compatibility
		v2::ToServerTunnelMessageKind::TunnelAck => {
			v3::ToServerTunnelMessageKind::DeprecatedTunnelAck
		}
	}
}

fn convert_to_server_tunnel_message_kind_v3_to_v2(
	kind: v3::ToServerTunnelMessageKind,
) -> Result<v2::ToServerTunnelMessageKind> {
	Ok(match kind {
		v3::ToServerTunnelMessageKind::ToServerResponseStart(resp) => {
			v2::ToServerTunnelMessageKind::ToServerResponseStart(v2::ToServerResponseStart {
				status: resp.status,
				headers: resp.headers,
				body: resp.body,
				stream: resp.stream,
			})
		}
		v3::ToServerTunnelMessageKind::ToServerResponseChunk(chunk) => {
			v2::ToServerTunnelMessageKind::ToServerResponseChunk(v2::ToServerResponseChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v3::ToServerTunnelMessageKind::ToServerResponseAbort => {
			v2::ToServerTunnelMessageKind::ToServerResponseAbort
		}
		v3::ToServerTunnelMessageKind::ToServerWebSocketOpen(open) => {
			v2::ToServerTunnelMessageKind::ToServerWebSocketOpen(v2::ToServerWebSocketOpen {
				can_hibernate: open.can_hibernate,
				last_msg_index: -1,
			})
		}
		v3::ToServerTunnelMessageKind::ToServerWebSocketMessage(msg) => {
			v2::ToServerTunnelMessageKind::ToServerWebSocketMessage(v2::ToServerWebSocketMessage {
				data: msg.data,
				binary: msg.binary,
			})
		}
		v3::ToServerTunnelMessageKind::ToServerWebSocketMessageAck(ack) => {
			v2::ToServerTunnelMessageKind::ToServerWebSocketMessageAck(
				v2::ToServerWebSocketMessageAck { index: ack.index },
			)
		}
		v3::ToServerTunnelMessageKind::ToServerWebSocketClose(close) => {
			v2::ToServerTunnelMessageKind::ToServerWebSocketClose(v2::ToServerWebSocketClose {
				code: close.code,
				reason: close.reason,
				retry: close.hibernate,
			})
		}
		v3::ToServerTunnelMessageKind::DeprecatedTunnelAck => {
			v2::ToServerTunnelMessageKind::TunnelAck
		}
	})
}

fn convert_event_v2_to_v3(event: v2::Event) -> v3::Event {
	match event {
		v2::Event::EventActorIntent(intent) => v3::Event::EventActorIntent(v3::EventActorIntent {
			actor_id: intent.actor_id,
			generation: intent.generation,
			intent: convert_actor_intent_v2_to_v3(intent.intent),
		}),
		v2::Event::EventActorStateUpdate(state) => {
			v3::Event::EventActorStateUpdate(v3::EventActorStateUpdate {
				actor_id: state.actor_id,
				generation: state.generation,
				state: convert_actor_state_v2_to_v3(state.state),
			})
		}
		v2::Event::EventActorSetAlarm(alarm) => {
			v3::Event::EventActorSetAlarm(v3::EventActorSetAlarm {
				actor_id: alarm.actor_id,
				generation: alarm.generation,
				alarm_ts: alarm.alarm_ts,
			})
		}
	}
}

fn convert_event_v3_to_v2(event: v3::Event) -> v2::Event {
	match event {
		v3::Event::EventActorIntent(intent) => v2::Event::EventActorIntent(v2::EventActorIntent {
			actor_id: intent.actor_id,
			generation: intent.generation,
			intent: convert_actor_intent_v3_to_v2(intent.intent),
		}),
		v3::Event::EventActorStateUpdate(state) => {
			v2::Event::EventActorStateUpdate(v2::EventActorStateUpdate {
				actor_id: state.actor_id,
				generation: state.generation,
				state: convert_actor_state_v3_to_v2(state.state),
			})
		}
		v3::Event::EventActorSetAlarm(alarm) => {
			v2::Event::EventActorSetAlarm(v2::EventActorSetAlarm {
				actor_id: alarm.actor_id,
				generation: alarm.generation,
				alarm_ts: alarm.alarm_ts,
			})
		}
	}
}

fn convert_actor_intent_v2_to_v3(intent: v2::ActorIntent) -> v3::ActorIntent {
	match intent {
		v2::ActorIntent::ActorIntentSleep => v3::ActorIntent::ActorIntentSleep,
		v2::ActorIntent::ActorIntentStop => v3::ActorIntent::ActorIntentStop,
	}
}

fn convert_actor_intent_v3_to_v2(intent: v3::ActorIntent) -> v2::ActorIntent {
	match intent {
		v3::ActorIntent::ActorIntentSleep => v2::ActorIntent::ActorIntentSleep,
		v3::ActorIntent::ActorIntentStop => v2::ActorIntent::ActorIntentStop,
	}
}

fn convert_actor_state_v2_to_v3(state: v2::ActorState) -> v3::ActorState {
	match state {
		v2::ActorState::ActorStateRunning => v3::ActorState::ActorStateRunning,
		v2::ActorState::ActorStateStopped(stopped) => {
			v3::ActorState::ActorStateStopped(v3::ActorStateStopped {
				code: convert_stop_code_v2_to_v3(stopped.code),
				message: stopped.message,
			})
		}
	}
}

fn convert_actor_state_v3_to_v2(state: v3::ActorState) -> v2::ActorState {
	match state {
		v3::ActorState::ActorStateRunning => v2::ActorState::ActorStateRunning,
		v3::ActorState::ActorStateStopped(stopped) => {
			v2::ActorState::ActorStateStopped(v2::ActorStateStopped {
				code: convert_stop_code_v3_to_v2(stopped.code),
				message: stopped.message,
			})
		}
	}
}

fn convert_stop_code_v2_to_v3(code: v2::StopCode) -> v3::StopCode {
	match code {
		v2::StopCode::Ok => v3::StopCode::Ok,
		v2::StopCode::Error => v3::StopCode::Error,
	}
}

fn convert_stop_code_v3_to_v2(code: v3::StopCode) -> v2::StopCode {
	match code {
		v3::StopCode::Ok => v2::StopCode::Ok,
		v3::StopCode::Error => v2::StopCode::Error,
	}
}

fn convert_kv_request_data_v2_to_v3(data: v2::KvRequestData) -> v3::KvRequestData {
	match data {
		v2::KvRequestData::KvGetRequest(req) => {
			v3::KvRequestData::KvGetRequest(v3::KvGetRequest { keys: req.keys })
		}
		v2::KvRequestData::KvListRequest(req) => {
			v3::KvRequestData::KvListRequest(v3::KvListRequest {
				query: convert_kv_list_query_v2_to_v3(req.query),
				reverse: req.reverse,
				limit: req.limit,
			})
		}
		v2::KvRequestData::KvPutRequest(req) => v3::KvRequestData::KvPutRequest(v3::KvPutRequest {
			keys: req.keys,
			values: req.values,
		}),
		v2::KvRequestData::KvDeleteRequest(req) => {
			v3::KvRequestData::KvDeleteRequest(v3::KvDeleteRequest { keys: req.keys })
		}
		v2::KvRequestData::KvDropRequest => v3::KvRequestData::KvDropRequest,
	}
}

fn convert_kv_request_data_v3_to_v2(data: v3::KvRequestData) -> v2::KvRequestData {
	match data {
		v3::KvRequestData::KvGetRequest(req) => {
			v2::KvRequestData::KvGetRequest(v2::KvGetRequest { keys: req.keys })
		}
		v3::KvRequestData::KvListRequest(req) => {
			v2::KvRequestData::KvListRequest(v2::KvListRequest {
				query: convert_kv_list_query_v3_to_v2(req.query),
				reverse: req.reverse,
				limit: req.limit,
			})
		}
		v3::KvRequestData::KvPutRequest(req) => v2::KvRequestData::KvPutRequest(v2::KvPutRequest {
			keys: req.keys,
			values: req.values,
		}),
		v3::KvRequestData::KvDeleteRequest(req) => {
			v2::KvRequestData::KvDeleteRequest(v2::KvDeleteRequest { keys: req.keys })
		}
		v3::KvRequestData::KvDropRequest => v2::KvRequestData::KvDropRequest,
	}
}

fn convert_kv_response_data_v2_to_v3(data: v2::KvResponseData) -> v3::KvResponseData {
	match data {
		v2::KvResponseData::KvErrorResponse(err) => {
			v3::KvResponseData::KvErrorResponse(v3::KvErrorResponse {
				message: err.message,
			})
		}
		v2::KvResponseData::KvGetResponse(resp) => {
			v3::KvResponseData::KvGetResponse(v3::KvGetResponse {
				keys: resp.keys,
				values: resp.values,
				metadata: resp
					.metadata
					.into_iter()
					.map(convert_kv_metadata_v2_to_v3)
					.collect(),
			})
		}
		v2::KvResponseData::KvListResponse(resp) => {
			v3::KvResponseData::KvListResponse(v3::KvListResponse {
				keys: resp.keys,
				values: resp.values,
				metadata: resp
					.metadata
					.into_iter()
					.map(convert_kv_metadata_v2_to_v3)
					.collect(),
			})
		}
		v2::KvResponseData::KvPutResponse => v3::KvResponseData::KvPutResponse,
		v2::KvResponseData::KvDeleteResponse => v3::KvResponseData::KvDeleteResponse,
		v2::KvResponseData::KvDropResponse => v3::KvResponseData::KvDropResponse,
	}
}

fn convert_kv_response_data_v3_to_v2(data: v3::KvResponseData) -> v2::KvResponseData {
	match data {
		v3::KvResponseData::KvErrorResponse(err) => {
			v2::KvResponseData::KvErrorResponse(v2::KvErrorResponse {
				message: err.message,
			})
		}
		v3::KvResponseData::KvGetResponse(resp) => {
			v2::KvResponseData::KvGetResponse(v2::KvGetResponse {
				keys: resp.keys,
				values: resp.values,
				metadata: resp
					.metadata
					.into_iter()
					.map(convert_kv_metadata_v3_to_v2)
					.collect(),
			})
		}
		v3::KvResponseData::KvListResponse(resp) => {
			v2::KvResponseData::KvListResponse(v2::KvListResponse {
				keys: resp.keys,
				values: resp.values,
				metadata: resp
					.metadata
					.into_iter()
					.map(convert_kv_metadata_v3_to_v2)
					.collect(),
			})
		}
		v3::KvResponseData::KvPutResponse => v2::KvResponseData::KvPutResponse,
		v3::KvResponseData::KvDeleteResponse => v2::KvResponseData::KvDeleteResponse,
		v3::KvResponseData::KvDropResponse => v2::KvResponseData::KvDropResponse,
	}
}

fn convert_kv_list_query_v2_to_v3(query: v2::KvListQuery) -> v3::KvListQuery {
	match query {
		v2::KvListQuery::KvListAllQuery => v3::KvListQuery::KvListAllQuery,
		v2::KvListQuery::KvListRangeQuery(range) => {
			v3::KvListQuery::KvListRangeQuery(v3::KvListRangeQuery {
				start: range.start,
				end: range.end,
				exclusive: range.exclusive,
			})
		}
		v2::KvListQuery::KvListPrefixQuery(prefix) => {
			v3::KvListQuery::KvListPrefixQuery(v3::KvListPrefixQuery { key: prefix.key })
		}
	}
}

fn convert_kv_list_query_v3_to_v2(query: v3::KvListQuery) -> v2::KvListQuery {
	match query {
		v3::KvListQuery::KvListAllQuery => v2::KvListQuery::KvListAllQuery,
		v3::KvListQuery::KvListRangeQuery(range) => {
			v2::KvListQuery::KvListRangeQuery(v2::KvListRangeQuery {
				start: range.start,
				end: range.end,
				exclusive: range.exclusive,
			})
		}
		v3::KvListQuery::KvListPrefixQuery(prefix) => {
			v2::KvListQuery::KvListPrefixQuery(v2::KvListPrefixQuery { key: prefix.key })
		}
	}
}

fn convert_kv_metadata_v2_to_v3(metadata: v2::KvMetadata) -> v3::KvMetadata {
	v3::KvMetadata {
		version: metadata.version,
		create_ts: metadata.create_ts,
	}
}

fn convert_kv_metadata_v3_to_v2(metadata: v3::KvMetadata) -> v2::KvMetadata {
	v2::KvMetadata {
		version: metadata.version,
		create_ts: metadata.create_ts,
	}
}
