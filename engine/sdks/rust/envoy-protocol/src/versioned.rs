use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::{v1, v2, v3};

pub enum ToEnvoy {
	V3(v3::ToEnvoy),
}

impl OwnedVersionedData for ToEnvoy {
	type Latest = v3::ToEnvoy;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V3(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => match serde_bare::from_slice::<v2::ToEnvoy>(payload) {
				Ok(data) => Ok(Self::V3(convert_to_envoy_v2_to_v3(data))),
				Err(_) => Ok(Self::V3(convert_to_envoy_v2_to_v3(
					convert_to_envoy_v1_to_v2(serde_bare::from_slice(payload)?)?,
				))),
			},
			2 => Ok(Self::V3(convert_to_envoy_v2_to_v3(serde_bare::from_slice(
				payload,
			)?))),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V3(data) => {
					serde_bare::to_vec(&convert_to_envoy_v2_to_v1(convert_to_envoy_v3_to_v2(data))?)
						.map_err(Into::into)
				}
			},
			2 => match self {
				Self::V3(data) => {
					serde_bare::to_vec(&convert_to_envoy_v3_to_v2(data)).map_err(Into::into)
				}
			},
			3 => match self {
				Self::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
			},
			_ => bail!("invalid version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok]
	}
}

pub enum ToRivet {
	V3(v3::ToRivet),
}

impl OwnedVersionedData for ToRivet {
	type Latest = v3::ToRivet;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V3(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V3(convert_to_rivet_v2_to_v3(
				convert_to_rivet_v1_to_v2(serde_bare::from_slice(payload)?),
			))),
			2 => Ok(Self::V3(convert_to_rivet_v2_to_v3(serde_bare::from_slice(
				payload,
			)?))),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V3(data) => {
					serde_bare::to_vec(&convert_to_rivet_v2_to_v1(convert_to_rivet_v3_to_v2(data))?)
						.map_err(Into::into)
				}
			},
			2 => match self {
				Self::V3(data) => {
					serde_bare::to_vec(&convert_to_rivet_v3_to_v2(data)).map_err(Into::into)
				}
			},
			3 => match self {
				Self::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
			},
			_ => bail!("invalid version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok]
	}
}

pub enum ToGateway {
	V3(v3::ToGateway),
}

impl OwnedVersionedData for ToGateway {
	type Latest = v3::ToGateway;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V3(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V3(convert_to_gateway_v2_to_v3(
				convert_to_gateway_v1_to_v2(serde_bare::from_slice(payload)?),
			))),
			2 => Ok(Self::V3(convert_to_gateway_v2_to_v3(
				serde_bare::from_slice(payload)?,
			))),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V3(data) => serde_bare::to_vec(&convert_to_gateway_v2_to_v1(
					convert_to_gateway_v3_to_v2(data),
				))
				.map_err(Into::into),
			},
			2 => match self {
				Self::V3(data) => {
					serde_bare::to_vec(&convert_to_gateway_v3_to_v2(data)).map_err(Into::into)
				}
			},
			3 => match self {
				Self::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
			},
			_ => bail!("invalid version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok]
	}
}

pub enum ToOutbound {
	V3(v3::ToOutbound),
}

impl OwnedVersionedData for ToOutbound {
	type Latest = v3::ToOutbound;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V3(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V3(convert_to_outbound_v2_to_v3(
				convert_to_outbound_v1_to_v2(serde_bare::from_slice(payload)?),
			))),
			2 => Ok(Self::V3(convert_to_outbound_v2_to_v3(
				serde_bare::from_slice(payload)?,
			))),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V3(data) => serde_bare::to_vec(&convert_to_outbound_v2_to_v1(
					convert_to_outbound_v3_to_v2(data),
				))
				.map_err(Into::into),
			},
			2 => match self {
				Self::V3(data) => {
					serde_bare::to_vec(&convert_to_outbound_v3_to_v2(data)).map_err(Into::into)
				}
			},
			3 => match self {
				Self::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
			},
			_ => bail!("invalid version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok]
	}
}

pub enum ToEnvoyConn {
	V3(v3::ToEnvoyConn),
}

impl OwnedVersionedData for ToEnvoyConn {
	type Latest = v3::ToEnvoyConn;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V3(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => match serde_bare::from_slice::<v2::ToEnvoyConn>(payload) {
				Ok(data) => Ok(Self::V3(convert_to_envoy_conn_v2_to_v3(data)?)),
				Err(_) => Ok(Self::V3(convert_to_envoy_conn_v2_to_v3(
					convert_to_envoy_conn_v1_to_v2(serde_bare::from_slice(payload)?)?,
				)?)),
			},
			2 => Ok(Self::V3(convert_to_envoy_conn_v2_to_v3(
				serde_bare::from_slice(payload)?,
			)?)),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V3(data) => serde_bare::to_vec(&convert_to_envoy_conn_v2_to_v1(
					convert_to_envoy_conn_v3_to_v2(data),
				)?)
				.map_err(Into::into),
			},
			2 => match self {
				Self::V3(data) => {
					serde_bare::to_vec(&convert_to_envoy_conn_v3_to_v2(data)).map_err(Into::into)
				}
			},
			3 => match self {
				Self::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
			},
			_ => bail!("invalid version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok]
	}
}

pub enum ActorCommandKeyData {
	V3(v3::ActorCommandKeyData),
}

impl OwnedVersionedData for ActorCommandKeyData {
	type Latest = v3::ActorCommandKeyData;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V3(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V3(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V3(convert_actor_command_key_data_v2_to_v3(
				convert_actor_command_key_data_v1_to_v2(serde_bare::from_slice(payload)?),
			))),
			2 => Ok(Self::V3(convert_actor_command_key_data_v2_to_v3(
				serde_bare::from_slice(payload)?,
			))),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V3(data) => serde_bare::to_vec(&convert_actor_command_key_data_v2_to_v1(
					convert_actor_command_key_data_v3_to_v2(data),
				))
				.map_err(Into::into),
			},
			2 => match self {
				Self::V3(data) => {
					serde_bare::to_vec(&convert_actor_command_key_data_v3_to_v2(data))
						.map_err(Into::into)
				}
			},
			3 => match self {
				Self::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
			},
			_ => bail!("invalid version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok]
	}
}

// MARK: ToEnvoy

fn convert_to_envoy_v1_to_v2(message: v1::ToEnvoy) -> Result<v2::ToEnvoy> {
	Ok(match message {
		v1::ToEnvoy::ToEnvoyInit(init) => {
			v2::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v1_to_v2(init))
		}
		v1::ToEnvoy::ToEnvoyCommands(commands) => v2::ToEnvoy::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v1_to_v2)
				.collect(),
		),
		v1::ToEnvoy::ToEnvoyAckEvents(ack) => {
			v2::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v1_to_v2(ack))
		}
		v1::ToEnvoy::ToEnvoyKvResponse(response) => {
			v2::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v1_to_v2(response))
		}
		v1::ToEnvoy::ToEnvoyTunnelMessage(message) => {
			v2::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v1_to_v2(message))
		}
		v1::ToEnvoy::ToEnvoyPing(ping) => {
			v2::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v1_to_v2(ping))
		}
	})
}

fn convert_to_envoy_v2_to_v1(message: v2::ToEnvoy) -> Result<v1::ToEnvoy> {
	Ok(match message {
		v2::ToEnvoy::ToEnvoyInit(init) => {
			v1::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v2_to_v1(init))
		}
		v2::ToEnvoy::ToEnvoyCommands(commands) => v1::ToEnvoy::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v2_to_v1)
				.collect::<Result<Vec<_>>>()?,
		),
		v2::ToEnvoy::ToEnvoyAckEvents(ack) => {
			v1::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v2_to_v1(ack))
		}
		v2::ToEnvoy::ToEnvoyKvResponse(response) => {
			v1::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v2_to_v1(response))
		}
		v2::ToEnvoy::ToEnvoyTunnelMessage(message) => {
			v1::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v2_to_v1(message))
		}
		v2::ToEnvoy::ToEnvoyPing(ping) => {
			v1::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v2_to_v1(ping))
		}
		v2::ToEnvoy::ToEnvoySqliteGetPagesResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitStageResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(_) => {
			bail!("sqlite responses require envoy-protocol v2 or later")
		}
	})
}

fn convert_to_envoy_v2_to_v3(message: v2::ToEnvoy) -> v3::ToEnvoy {
	match message {
		v2::ToEnvoy::ToEnvoyInit(init) => {
			v3::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v2_to_v3(init))
		}
		v2::ToEnvoy::ToEnvoyCommands(commands) => v3::ToEnvoy::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v2_to_v3)
				.collect(),
		),
		v2::ToEnvoy::ToEnvoyAckEvents(ack) => {
			v3::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v2_to_v3(ack))
		}
		v2::ToEnvoy::ToEnvoyKvResponse(response) => {
			v3::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v2_to_v3(response))
		}
		v2::ToEnvoy::ToEnvoyTunnelMessage(message) => {
			v3::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v2_to_v3(message))
		}
		v2::ToEnvoy::ToEnvoyPing(ping) => {
			v3::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v2_to_v3(ping))
		}
		v2::ToEnvoy::ToEnvoySqliteGetPagesResponse(response) => {
			v3::ToEnvoy::ToEnvoySqliteGetPagesResponse(v3::ToEnvoySqliteGetPagesResponse {
				request_id: response.request_id,
				data: convert_sqlite_get_pages_response_v2_to_v3(response.data),
			})
		}
		v2::ToEnvoy::ToEnvoySqliteCommitResponse(response) => {
			v3::ToEnvoy::ToEnvoySqliteCommitResponse(v3::ToEnvoySqliteCommitResponse {
				request_id: response.request_id,
				data: convert_sqlite_commit_response_v2_to_v3(response.data),
			})
		}
		v2::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(response) => {
			v3::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(
				v3::ToEnvoySqliteCommitStageBeginResponse {
					request_id: response.request_id,
					data: convert_sqlite_commit_stage_begin_response_v2_to_v3(response.data),
				},
			)
		}
		v2::ToEnvoy::ToEnvoySqliteCommitStageResponse(response) => {
			v3::ToEnvoy::ToEnvoySqliteCommitStageResponse(v3::ToEnvoySqliteCommitStageResponse {
				request_id: response.request_id,
				data: convert_sqlite_commit_stage_response_v2_to_v3(response.data),
			})
		}
		v2::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(response) => {
			v3::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(
				v3::ToEnvoySqliteCommitFinalizeResponse {
					request_id: response.request_id,
					data: convert_sqlite_commit_finalize_response_v2_to_v3(response.data),
				},
			)
		}
	}
}

fn convert_to_envoy_v3_to_v2(message: v3::ToEnvoy) -> v2::ToEnvoy {
	match message {
		v3::ToEnvoy::ToEnvoyInit(init) => {
			v2::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v3_to_v2(init))
		}
		v3::ToEnvoy::ToEnvoyCommands(commands) => v2::ToEnvoy::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v3_to_v2)
				.collect(),
		),
		v3::ToEnvoy::ToEnvoyAckEvents(ack) => {
			v2::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v3_to_v2(ack))
		}
		v3::ToEnvoy::ToEnvoyKvResponse(response) => {
			v2::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v3_to_v2(response))
		}
		v3::ToEnvoy::ToEnvoyTunnelMessage(message) => {
			v2::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v3_to_v2(message))
		}
		v3::ToEnvoy::ToEnvoyPing(ping) => {
			v2::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v3_to_v2(ping))
		}
		v3::ToEnvoy::ToEnvoySqliteGetPagesResponse(response) => {
			v2::ToEnvoy::ToEnvoySqliteGetPagesResponse(v2::ToEnvoySqliteGetPagesResponse {
				request_id: response.request_id,
				data: convert_sqlite_get_pages_response_v3_to_v2(response.data),
			})
		}
		v3::ToEnvoy::ToEnvoySqliteCommitResponse(response) => {
			v2::ToEnvoy::ToEnvoySqliteCommitResponse(v2::ToEnvoySqliteCommitResponse {
				request_id: response.request_id,
				data: convert_sqlite_commit_response_v3_to_v2(response.data),
			})
		}
		v3::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(response) => {
			v2::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(
				v2::ToEnvoySqliteCommitStageBeginResponse {
					request_id: response.request_id,
					data: convert_sqlite_commit_stage_begin_response_v3_to_v2(response.data),
				},
			)
		}
		v3::ToEnvoy::ToEnvoySqliteCommitStageResponse(response) => {
			v2::ToEnvoy::ToEnvoySqliteCommitStageResponse(v2::ToEnvoySqliteCommitStageResponse {
				request_id: response.request_id,
				data: convert_sqlite_commit_stage_response_v3_to_v2(response.data),
			})
		}
		v3::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(response) => {
			v2::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(
				v2::ToEnvoySqliteCommitFinalizeResponse {
					request_id: response.request_id,
					data: convert_sqlite_commit_finalize_response_v3_to_v2(response.data),
				},
			)
		}
	}
}

// MARK: ToRivet

fn convert_to_rivet_v1_to_v2(message: v1::ToRivet) -> v2::ToRivet {
	match message {
		v1::ToRivet::ToRivetMetadata(metadata) => {
			v2::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v1_to_v2(metadata))
		}
		v1::ToRivet::ToRivetEvents(events) => v2::ToRivet::ToRivetEvents(
			events
				.into_iter()
				.map(convert_event_wrapper_v1_to_v2)
				.collect(),
		),
		v1::ToRivet::ToRivetAckCommands(ack) => {
			v2::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v1_to_v2(ack))
		}
		v1::ToRivet::ToRivetStopping => v2::ToRivet::ToRivetStopping,
		v1::ToRivet::ToRivetPong(pong) => {
			v2::ToRivet::ToRivetPong(convert_to_rivet_pong_v1_to_v2(pong))
		}
		v1::ToRivet::ToRivetKvRequest(request) => {
			v2::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v1_to_v2(request))
		}
		v1::ToRivet::ToRivetTunnelMessage(message) => {
			v2::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v1_to_v2(message))
		}
	}
}

fn convert_to_rivet_v2_to_v1(message: v2::ToRivet) -> Result<v1::ToRivet> {
	Ok(match message {
		v2::ToRivet::ToRivetMetadata(metadata) => {
			v1::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v2_to_v1(metadata))
		}
		v2::ToRivet::ToRivetEvents(events) => v1::ToRivet::ToRivetEvents(
			events
				.into_iter()
				.map(convert_event_wrapper_v2_to_v1)
				.collect(),
		),
		v2::ToRivet::ToRivetAckCommands(ack) => {
			v1::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v2_to_v1(ack))
		}
		v2::ToRivet::ToRivetStopping => v1::ToRivet::ToRivetStopping,
		v2::ToRivet::ToRivetPong(pong) => {
			v1::ToRivet::ToRivetPong(convert_to_rivet_pong_v2_to_v1(pong))
		}
		v2::ToRivet::ToRivetKvRequest(request) => {
			v1::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v2_to_v1(request))
		}
		v2::ToRivet::ToRivetTunnelMessage(message) => {
			v1::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v2_to_v1(message))
		}
		v2::ToRivet::ToRivetSqliteGetPagesRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitStageBeginRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitStageRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitFinalizeRequest(_) => {
			bail!("sqlite requests require envoy-protocol v2 or later")
		}
	})
}

fn convert_to_rivet_v2_to_v3(message: v2::ToRivet) -> v3::ToRivet {
	match message {
		v2::ToRivet::ToRivetMetadata(metadata) => {
			v3::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v2_to_v3(metadata))
		}
		v2::ToRivet::ToRivetEvents(events) => v3::ToRivet::ToRivetEvents(
			events
				.into_iter()
				.map(convert_event_wrapper_v2_to_v3)
				.collect(),
		),
		v2::ToRivet::ToRivetAckCommands(ack) => {
			v3::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v2_to_v3(ack))
		}
		v2::ToRivet::ToRivetStopping => v3::ToRivet::ToRivetStopping,
		v2::ToRivet::ToRivetPong(pong) => {
			v3::ToRivet::ToRivetPong(convert_to_rivet_pong_v2_to_v3(pong))
		}
		v2::ToRivet::ToRivetKvRequest(request) => {
			v3::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v2_to_v3(request))
		}
		v2::ToRivet::ToRivetTunnelMessage(message) => {
			v3::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v2_to_v3(message))
		}
		v2::ToRivet::ToRivetSqliteGetPagesRequest(request) => {
			v3::ToRivet::ToRivetSqliteGetPagesRequest(v3::ToRivetSqliteGetPagesRequest {
				request_id: request.request_id,
				data: convert_sqlite_get_pages_request_v2_to_v3(request.data),
			})
		}
		v2::ToRivet::ToRivetSqliteCommitRequest(request) => {
			v3::ToRivet::ToRivetSqliteCommitRequest(v3::ToRivetSqliteCommitRequest {
				request_id: request.request_id,
				data: convert_sqlite_commit_request_v2_to_v3(request.data),
			})
		}
		v2::ToRivet::ToRivetSqliteCommitStageBeginRequest(request) => {
			v3::ToRivet::ToRivetSqliteCommitStageBeginRequest(
				v3::ToRivetSqliteCommitStageBeginRequest {
					request_id: request.request_id,
					data: convert_sqlite_commit_stage_begin_request_v2_to_v3(request.data),
				},
			)
		}
		v2::ToRivet::ToRivetSqliteCommitStageRequest(request) => {
			v3::ToRivet::ToRivetSqliteCommitStageRequest(v3::ToRivetSqliteCommitStageRequest {
				request_id: request.request_id,
				data: convert_sqlite_commit_stage_request_v2_to_v3(request.data),
			})
		}
		v2::ToRivet::ToRivetSqliteCommitFinalizeRequest(request) => {
			v3::ToRivet::ToRivetSqliteCommitFinalizeRequest(
				v3::ToRivetSqliteCommitFinalizeRequest {
					request_id: request.request_id,
					data: convert_sqlite_commit_finalize_request_v2_to_v3(request.data),
				},
			)
		}
	}
}

fn convert_to_rivet_v3_to_v2(message: v3::ToRivet) -> v2::ToRivet {
	match message {
		v3::ToRivet::ToRivetMetadata(metadata) => {
			v2::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v3_to_v2(metadata))
		}
		v3::ToRivet::ToRivetEvents(events) => v2::ToRivet::ToRivetEvents(
			events
				.into_iter()
				.map(convert_event_wrapper_v3_to_v2)
				.collect(),
		),
		v3::ToRivet::ToRivetAckCommands(ack) => {
			v2::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v3_to_v2(ack))
		}
		v3::ToRivet::ToRivetStopping => v2::ToRivet::ToRivetStopping,
		v3::ToRivet::ToRivetPong(pong) => {
			v2::ToRivet::ToRivetPong(convert_to_rivet_pong_v3_to_v2(pong))
		}
		v3::ToRivet::ToRivetKvRequest(request) => {
			v2::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v3_to_v2(request))
		}
		v3::ToRivet::ToRivetTunnelMessage(message) => {
			v2::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v3_to_v2(message))
		}
		v3::ToRivet::ToRivetSqliteGetPagesRequest(request) => {
			v2::ToRivet::ToRivetSqliteGetPagesRequest(v2::ToRivetSqliteGetPagesRequest {
				request_id: request.request_id,
				data: convert_sqlite_get_pages_request_v3_to_v2(request.data),
			})
		}
		v3::ToRivet::ToRivetSqliteCommitRequest(request) => {
			v2::ToRivet::ToRivetSqliteCommitRequest(v2::ToRivetSqliteCommitRequest {
				request_id: request.request_id,
				data: convert_sqlite_commit_request_v3_to_v2(request.data),
			})
		}
		v3::ToRivet::ToRivetSqliteCommitStageBeginRequest(request) => {
			v2::ToRivet::ToRivetSqliteCommitStageBeginRequest(
				v2::ToRivetSqliteCommitStageBeginRequest {
					request_id: request.request_id,
					data: convert_sqlite_commit_stage_begin_request_v3_to_v2(request.data),
				},
			)
		}
		v3::ToRivet::ToRivetSqliteCommitStageRequest(request) => {
			v2::ToRivet::ToRivetSqliteCommitStageRequest(v2::ToRivetSqliteCommitStageRequest {
				request_id: request.request_id,
				data: convert_sqlite_commit_stage_request_v3_to_v2(request.data),
			})
		}
		v3::ToRivet::ToRivetSqliteCommitFinalizeRequest(request) => {
			v2::ToRivet::ToRivetSqliteCommitFinalizeRequest(
				v2::ToRivetSqliteCommitFinalizeRequest {
					request_id: request.request_id,
					data: convert_sqlite_commit_finalize_request_v3_to_v2(request.data),
				},
			)
		}
	}
}

// MARK: ToEnvoyConn

fn convert_to_envoy_conn_v1_to_v2(message: v1::ToEnvoyConn) -> Result<v2::ToEnvoyConn> {
	Ok(match message {
		v1::ToEnvoyConn::ToEnvoyConnPing(ping) => {
			v2::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v1_to_v2(ping))
		}
		v1::ToEnvoyConn::ToEnvoyConnClose => v2::ToEnvoyConn::ToEnvoyConnClose,
		v1::ToEnvoyConn::ToEnvoyCommands(commands) => v2::ToEnvoyConn::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v1_to_v2)
				.collect(),
		),
		v1::ToEnvoyConn::ToEnvoyAckEvents(ack) => {
			v2::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v1_to_v2(ack))
		}
		v1::ToEnvoyConn::ToEnvoyTunnelMessage(message) => {
			v2::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v1_to_v2(message))
		}
	})
}

fn convert_to_envoy_conn_v2_to_v1(message: v2::ToEnvoyConn) -> Result<v1::ToEnvoyConn> {
	Ok(match message {
		v2::ToEnvoyConn::ToEnvoyConnPing(ping) => {
			v1::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v2_to_v1(ping))
		}
		v2::ToEnvoyConn::ToEnvoyConnClose => v1::ToEnvoyConn::ToEnvoyConnClose,
		v2::ToEnvoyConn::ToEnvoyCommands(commands) => v1::ToEnvoyConn::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v2_to_v1)
				.collect::<Result<Vec<_>>>()?,
		),
		v2::ToEnvoyConn::ToEnvoyAckEvents(ack) => {
			v1::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v2_to_v1(ack))
		}
		v2::ToEnvoyConn::ToEnvoyTunnelMessage(message) => {
			v1::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v2_to_v1(message))
		}
	})
}

fn convert_to_envoy_conn_v2_to_v3(message: v2::ToEnvoyConn) -> Result<v3::ToEnvoyConn> {
	Ok(match message {
		v2::ToEnvoyConn::ToEnvoyConnPing(ping) => {
			v3::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v2_to_v3(ping))
		}
		v2::ToEnvoyConn::ToEnvoyConnClose => v3::ToEnvoyConn::ToEnvoyConnClose,
		v2::ToEnvoyConn::ToEnvoyCommands(commands) => v3::ToEnvoyConn::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v2_to_v3)
				.collect(),
		),
		v2::ToEnvoyConn::ToEnvoyAckEvents(ack) => {
			v3::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v2_to_v3(ack))
		}
		v2::ToEnvoyConn::ToEnvoyTunnelMessage(message) => {
			v3::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v2_to_v3(message))
		}
	})
}

fn convert_to_envoy_conn_v3_to_v2(message: v3::ToEnvoyConn) -> v2::ToEnvoyConn {
	match message {
		v3::ToEnvoyConn::ToEnvoyConnPing(ping) => {
			v2::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v3_to_v2(ping))
		}
		v3::ToEnvoyConn::ToEnvoyConnClose => v2::ToEnvoyConn::ToEnvoyConnClose,
		v3::ToEnvoyConn::ToEnvoyCommands(commands) => v2::ToEnvoyConn::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v3_to_v2)
				.collect(),
		),
		v3::ToEnvoyConn::ToEnvoyAckEvents(ack) => {
			v2::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v3_to_v2(ack))
		}
		v3::ToEnvoyConn::ToEnvoyTunnelMessage(message) => {
			v2::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v3_to_v2(message))
		}
	}
}

// MARK: ToGateway

fn convert_to_gateway_v1_to_v2(message: v1::ToGateway) -> v2::ToGateway {
	match message {
		v1::ToGateway::ToGatewayPong(pong) => {
			v2::ToGateway::ToGatewayPong(convert_to_gateway_pong_v1_to_v2(pong))
		}
		v1::ToGateway::ToRivetTunnelMessage(message) => {
			v2::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v1_to_v2(message))
		}
	}
}

fn convert_to_gateway_v2_to_v1(message: v2::ToGateway) -> v1::ToGateway {
	match message {
		v2::ToGateway::ToGatewayPong(pong) => {
			v1::ToGateway::ToGatewayPong(convert_to_gateway_pong_v2_to_v1(pong))
		}
		v2::ToGateway::ToRivetTunnelMessage(message) => {
			v1::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v2_to_v1(message))
		}
	}
}

fn convert_to_gateway_v2_to_v3(message: v2::ToGateway) -> v3::ToGateway {
	match message {
		v2::ToGateway::ToGatewayPong(pong) => {
			v3::ToGateway::ToGatewayPong(convert_to_gateway_pong_v2_to_v3(pong))
		}
		v2::ToGateway::ToRivetTunnelMessage(message) => {
			v3::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v2_to_v3(message))
		}
	}
}

fn convert_to_gateway_v3_to_v2(message: v3::ToGateway) -> v2::ToGateway {
	match message {
		v3::ToGateway::ToGatewayPong(pong) => {
			v2::ToGateway::ToGatewayPong(convert_to_gateway_pong_v3_to_v2(pong))
		}
		v3::ToGateway::ToRivetTunnelMessage(message) => {
			v2::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v3_to_v2(message))
		}
	}
}

// MARK: ToOutbound

fn convert_to_outbound_v1_to_v2(message: v1::ToOutbound) -> v2::ToOutbound {
	match message {
		v1::ToOutbound::ToOutboundActorStart(start) => {
			v2::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v1_to_v2(start))
		}
	}
}

fn convert_to_outbound_v2_to_v1(message: v2::ToOutbound) -> v1::ToOutbound {
	match message {
		v2::ToOutbound::ToOutboundActorStart(start) => {
			v1::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v2_to_v1(start))
		}
	}
}

fn convert_to_outbound_v2_to_v3(message: v2::ToOutbound) -> v3::ToOutbound {
	match message {
		v2::ToOutbound::ToOutboundActorStart(start) => {
			v3::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v2_to_v3(start))
		}
	}
}

fn convert_to_outbound_v3_to_v2(message: v3::ToOutbound) -> v2::ToOutbound {
	match message {
		v3::ToOutbound::ToOutboundActorStart(start) => {
			v2::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v3_to_v2(start))
		}
	}
}

fn convert_to_outbound_actor_start_v1_to_v2(
	start: v1::ToOutboundActorStart,
) -> v2::ToOutboundActorStart {
	v2::ToOutboundActorStart {
		namespace_id: start.namespace_id,
		pool_name: start.pool_name,
		checkpoint: convert_actor_checkpoint_v1_to_v2(start.checkpoint),
		actor_config: convert_actor_config_v1_to_v2(start.actor_config),
	}
}

fn convert_to_outbound_actor_start_v2_to_v1(
	start: v2::ToOutboundActorStart,
) -> v1::ToOutboundActorStart {
	v1::ToOutboundActorStart {
		namespace_id: start.namespace_id,
		pool_name: start.pool_name,
		checkpoint: convert_actor_checkpoint_v2_to_v1(start.checkpoint),
		actor_config: convert_actor_config_v2_to_v1(start.actor_config),
	}
}

fn convert_to_outbound_actor_start_v2_to_v3(
	start: v2::ToOutboundActorStart,
) -> v3::ToOutboundActorStart {
	v3::ToOutboundActorStart {
		namespace_id: start.namespace_id,
		pool_name: start.pool_name,
		checkpoint: convert_actor_checkpoint_v2_to_v3(start.checkpoint),
		actor_config: convert_actor_config_v2_to_v3(start.actor_config),
	}
}

fn convert_to_outbound_actor_start_v3_to_v2(
	start: v3::ToOutboundActorStart,
) -> v2::ToOutboundActorStart {
	v2::ToOutboundActorStart {
		namespace_id: start.namespace_id,
		pool_name: start.pool_name,
		checkpoint: convert_actor_checkpoint_v3_to_v2(start.checkpoint),
		actor_config: convert_actor_config_v3_to_v2(start.actor_config),
	}
}

// MARK: Command wrappers

fn convert_command_wrapper_v1_to_v2(wrapper: v1::CommandWrapper) -> v2::CommandWrapper {
	v2::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v1_to_v2(wrapper.checkpoint),
		inner: convert_command_v1_to_v2(wrapper.inner),
	}
}

fn convert_command_wrapper_v2_to_v1(wrapper: v2::CommandWrapper) -> Result<v1::CommandWrapper> {
	Ok(v1::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v2_to_v1(wrapper.checkpoint),
		inner: convert_command_v2_to_v1(wrapper.inner)?,
	})
}

fn convert_command_wrapper_v2_to_v3(wrapper: v2::CommandWrapper) -> v3::CommandWrapper {
	v3::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v2_to_v3(wrapper.checkpoint),
		inner: convert_command_v2_to_v3(wrapper.inner),
	}
}

fn convert_command_wrapper_v3_to_v2(wrapper: v3::CommandWrapper) -> v2::CommandWrapper {
	v2::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v3_to_v2(wrapper.checkpoint),
		inner: convert_command_v3_to_v2(wrapper.inner),
	}
}

fn convert_command_v1_to_v2(command: v1::Command) -> v2::Command {
	match command {
		v1::Command::CommandStartActor(start) => {
			v2::Command::CommandStartActor(convert_command_start_actor_v1_to_v2(start))
		}
		v1::Command::CommandStopActor(stop) => {
			v2::Command::CommandStopActor(v2::CommandStopActor {
				reason: convert_stop_actor_reason_v1_to_v2(stop.reason),
			})
		}
	}
}

fn convert_command_v2_to_v1(command: v2::Command) -> Result<v1::Command> {
	Ok(match command {
		v2::Command::CommandStartActor(start) => {
			v1::Command::CommandStartActor(convert_command_start_actor_v2_to_v1(start))
		}
		v2::Command::CommandStopActor(stop) => {
			v1::Command::CommandStopActor(v1::CommandStopActor {
				reason: convert_stop_actor_reason_v2_to_v1(stop.reason),
			})
		}
	})
}

fn convert_command_v2_to_v3(command: v2::Command) -> v3::Command {
	match command {
		v2::Command::CommandStartActor(start) => {
			v3::Command::CommandStartActor(convert_command_start_actor_v2_to_v3(start))
		}
		v2::Command::CommandStopActor(stop) => {
			v3::Command::CommandStopActor(v3::CommandStopActor {
				reason: convert_stop_actor_reason_v2_to_v3(stop.reason),
			})
		}
	}
}

fn convert_command_v3_to_v2(command: v3::Command) -> v2::Command {
	match command {
		v3::Command::CommandStartActor(start) => {
			v2::Command::CommandStartActor(convert_command_start_actor_v3_to_v2(start))
		}
		v3::Command::CommandStopActor(stop) => {
			v2::Command::CommandStopActor(v2::CommandStopActor {
				reason: convert_stop_actor_reason_v3_to_v2(stop.reason),
			})
		}
	}
}

fn convert_command_start_actor_v1_to_v2(start: v1::CommandStartActor) -> v2::CommandStartActor {
	v2::CommandStartActor {
		config: convert_actor_config_v1_to_v2(start.config),
		hibernating_requests: start
			.hibernating_requests
			.into_iter()
			.map(convert_hibernating_request_v1_to_v2)
			.collect(),
		preloaded_kv: start.preloaded_kv.map(convert_preloaded_kv_v1_to_v2),
		sqlite_startup_data: None,
	}
}

fn convert_command_start_actor_v2_to_v1(start: v2::CommandStartActor) -> v1::CommandStartActor {
	v1::CommandStartActor {
		config: convert_actor_config_v2_to_v1(start.config),
		hibernating_requests: start
			.hibernating_requests
			.into_iter()
			.map(convert_hibernating_request_v2_to_v1)
			.collect(),
		preloaded_kv: start.preloaded_kv.map(convert_preloaded_kv_v2_to_v1),
	}
}

fn convert_command_start_actor_v2_to_v3(start: v2::CommandStartActor) -> v3::CommandStartActor {
	v3::CommandStartActor {
		config: convert_actor_config_v2_to_v3(start.config),
		hibernating_requests: start
			.hibernating_requests
			.into_iter()
			.map(convert_hibernating_request_v2_to_v3)
			.collect(),
		preloaded_kv: start.preloaded_kv.map(convert_preloaded_kv_v2_to_v3),
		sqlite_startup_data: start
			.sqlite_startup_data
			.map(convert_sqlite_startup_data_v2_to_v3),
	}
}

fn convert_command_start_actor_v3_to_v2(start: v3::CommandStartActor) -> v2::CommandStartActor {
	v2::CommandStartActor {
		config: convert_actor_config_v3_to_v2(start.config),
		hibernating_requests: start
			.hibernating_requests
			.into_iter()
			.map(convert_hibernating_request_v3_to_v2)
			.collect(),
		preloaded_kv: start.preloaded_kv.map(convert_preloaded_kv_v3_to_v2),
		sqlite_startup_data: start
			.sqlite_startup_data
			.map(convert_sqlite_startup_data_v3_to_v2),
	}
}

fn convert_actor_command_key_data_v1_to_v2(
	data: v1::ActorCommandKeyData,
) -> v2::ActorCommandKeyData {
	match data {
		v1::ActorCommandKeyData::CommandStartActor(start) => {
			v2::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v1_to_v2(start))
		}
		v1::ActorCommandKeyData::CommandStopActor(stop) => {
			v2::ActorCommandKeyData::CommandStopActor(v2::CommandStopActor {
				reason: convert_stop_actor_reason_v1_to_v2(stop.reason),
			})
		}
	}
}

fn convert_actor_command_key_data_v2_to_v1(
	data: v2::ActorCommandKeyData,
) -> v1::ActorCommandKeyData {
	match data {
		v2::ActorCommandKeyData::CommandStartActor(start) => {
			v1::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v2_to_v1(start))
		}
		v2::ActorCommandKeyData::CommandStopActor(stop) => {
			v1::ActorCommandKeyData::CommandStopActor(v1::CommandStopActor {
				reason: convert_stop_actor_reason_v2_to_v1(stop.reason),
			})
		}
	}
}

fn convert_actor_command_key_data_v2_to_v3(
	data: v2::ActorCommandKeyData,
) -> v3::ActorCommandKeyData {
	match data {
		v2::ActorCommandKeyData::CommandStartActor(start) => {
			v3::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v2_to_v3(start))
		}
		v2::ActorCommandKeyData::CommandStopActor(stop) => {
			v3::ActorCommandKeyData::CommandStopActor(v3::CommandStopActor {
				reason: convert_stop_actor_reason_v2_to_v3(stop.reason),
			})
		}
	}
}

fn convert_actor_command_key_data_v3_to_v2(
	data: v3::ActorCommandKeyData,
) -> v2::ActorCommandKeyData {
	match data {
		v3::ActorCommandKeyData::CommandStartActor(start) => {
			v2::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v3_to_v2(start))
		}
		v3::ActorCommandKeyData::CommandStopActor(stop) => {
			v2::ActorCommandKeyData::CommandStopActor(v2::CommandStopActor {
				reason: convert_stop_actor_reason_v3_to_v2(stop.reason),
			})
		}
	}
}

// MARK: Actor primitives

fn convert_actor_checkpoint_v1_to_v2(checkpoint: v1::ActorCheckpoint) -> v2::ActorCheckpoint {
	v2::ActorCheckpoint {
		actor_id: checkpoint.actor_id,
		generation: checkpoint.generation,
		index: checkpoint.index,
	}
}

fn convert_actor_checkpoint_v2_to_v1(checkpoint: v2::ActorCheckpoint) -> v1::ActorCheckpoint {
	v1::ActorCheckpoint {
		actor_id: checkpoint.actor_id,
		generation: checkpoint.generation,
		index: checkpoint.index,
	}
}

fn convert_actor_checkpoint_v2_to_v3(checkpoint: v2::ActorCheckpoint) -> v3::ActorCheckpoint {
	v3::ActorCheckpoint {
		actor_id: checkpoint.actor_id,
		generation: checkpoint.generation,
		index: checkpoint.index,
	}
}

fn convert_actor_checkpoint_v3_to_v2(checkpoint: v3::ActorCheckpoint) -> v2::ActorCheckpoint {
	v2::ActorCheckpoint {
		actor_id: checkpoint.actor_id,
		generation: checkpoint.generation,
		index: checkpoint.index,
	}
}

fn convert_actor_config_v1_to_v2(config: v1::ActorConfig) -> v2::ActorConfig {
	v2::ActorConfig {
		name: config.name,
		key: config.key,
		create_ts: config.create_ts,
		input: config.input,
	}
}

fn convert_actor_config_v2_to_v1(config: v2::ActorConfig) -> v1::ActorConfig {
	v1::ActorConfig {
		name: config.name,
		key: config.key,
		create_ts: config.create_ts,
		input: config.input,
	}
}

fn convert_actor_config_v2_to_v3(config: v2::ActorConfig) -> v3::ActorConfig {
	v3::ActorConfig {
		name: config.name,
		key: config.key,
		create_ts: config.create_ts,
		input: config.input,
	}
}

fn convert_actor_config_v3_to_v2(config: v3::ActorConfig) -> v2::ActorConfig {
	v2::ActorConfig {
		name: config.name,
		key: config.key,
		create_ts: config.create_ts,
		input: config.input,
	}
}

fn convert_hibernating_request_v1_to_v2(request: v1::HibernatingRequest) -> v2::HibernatingRequest {
	v2::HibernatingRequest {
		gateway_id: request.gateway_id,
		request_id: request.request_id,
	}
}

fn convert_hibernating_request_v2_to_v1(request: v2::HibernatingRequest) -> v1::HibernatingRequest {
	v1::HibernatingRequest {
		gateway_id: request.gateway_id,
		request_id: request.request_id,
	}
}

fn convert_hibernating_request_v2_to_v3(request: v2::HibernatingRequest) -> v3::HibernatingRequest {
	v3::HibernatingRequest {
		gateway_id: request.gateway_id,
		request_id: request.request_id,
	}
}

fn convert_hibernating_request_v3_to_v2(request: v3::HibernatingRequest) -> v2::HibernatingRequest {
	v2::HibernatingRequest {
		gateway_id: request.gateway_id,
		request_id: request.request_id,
	}
}

fn convert_stop_actor_reason_v1_to_v2(reason: v1::StopActorReason) -> v2::StopActorReason {
	match reason {
		v1::StopActorReason::SleepIntent => v2::StopActorReason::SleepIntent,
		v1::StopActorReason::StopIntent => v2::StopActorReason::StopIntent,
		v1::StopActorReason::Destroy => v2::StopActorReason::Destroy,
		v1::StopActorReason::GoingAway => v2::StopActorReason::GoingAway,
		v1::StopActorReason::Lost => v2::StopActorReason::Lost,
	}
}

fn convert_stop_actor_reason_v2_to_v1(reason: v2::StopActorReason) -> v1::StopActorReason {
	match reason {
		v2::StopActorReason::SleepIntent => v1::StopActorReason::SleepIntent,
		v2::StopActorReason::StopIntent => v1::StopActorReason::StopIntent,
		v2::StopActorReason::Destroy => v1::StopActorReason::Destroy,
		v2::StopActorReason::GoingAway => v1::StopActorReason::GoingAway,
		v2::StopActorReason::Lost => v1::StopActorReason::Lost,
	}
}

fn convert_stop_actor_reason_v2_to_v3(reason: v2::StopActorReason) -> v3::StopActorReason {
	match reason {
		v2::StopActorReason::SleepIntent => v3::StopActorReason::SleepIntent,
		v2::StopActorReason::StopIntent => v3::StopActorReason::StopIntent,
		v2::StopActorReason::Destroy => v3::StopActorReason::Destroy,
		v2::StopActorReason::GoingAway => v3::StopActorReason::GoingAway,
		v2::StopActorReason::Lost => v3::StopActorReason::Lost,
	}
}

fn convert_stop_actor_reason_v3_to_v2(reason: v3::StopActorReason) -> v2::StopActorReason {
	match reason {
		v3::StopActorReason::SleepIntent => v2::StopActorReason::SleepIntent,
		v3::StopActorReason::StopIntent => v2::StopActorReason::StopIntent,
		v3::StopActorReason::Destroy => v2::StopActorReason::Destroy,
		v3::StopActorReason::GoingAway => v2::StopActorReason::GoingAway,
		v3::StopActorReason::Lost => v2::StopActorReason::Lost,
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

// MARK: Events

fn convert_event_wrapper_v1_to_v2(wrapper: v1::EventWrapper) -> v2::EventWrapper {
	v2::EventWrapper {
		checkpoint: convert_actor_checkpoint_v1_to_v2(wrapper.checkpoint),
		inner: convert_event_v1_to_v2(wrapper.inner),
	}
}

fn convert_event_wrapper_v2_to_v1(wrapper: v2::EventWrapper) -> v1::EventWrapper {
	v1::EventWrapper {
		checkpoint: convert_actor_checkpoint_v2_to_v1(wrapper.checkpoint),
		inner: convert_event_v2_to_v1(wrapper.inner),
	}
}

fn convert_event_wrapper_v2_to_v3(wrapper: v2::EventWrapper) -> v3::EventWrapper {
	v3::EventWrapper {
		checkpoint: convert_actor_checkpoint_v2_to_v3(wrapper.checkpoint),
		inner: convert_event_v2_to_v3(wrapper.inner),
	}
}

fn convert_event_wrapper_v3_to_v2(wrapper: v3::EventWrapper) -> v2::EventWrapper {
	v2::EventWrapper {
		checkpoint: convert_actor_checkpoint_v3_to_v2(wrapper.checkpoint),
		inner: convert_event_v3_to_v2(wrapper.inner),
	}
}

fn convert_event_v1_to_v2(event: v1::Event) -> v2::Event {
	match event {
		v1::Event::EventActorIntent(intent) => v2::Event::EventActorIntent(v2::EventActorIntent {
			intent: convert_actor_intent_v1_to_v2(intent.intent),
		}),
		v1::Event::EventActorStateUpdate(update) => {
			v2::Event::EventActorStateUpdate(v2::EventActorStateUpdate {
				state: convert_actor_state_v1_to_v2(update.state),
			})
		}
		v1::Event::EventActorSetAlarm(alarm) => {
			v2::Event::EventActorSetAlarm(v2::EventActorSetAlarm {
				alarm_ts: alarm.alarm_ts,
			})
		}
	}
}

fn convert_event_v2_to_v1(event: v2::Event) -> v1::Event {
	match event {
		v2::Event::EventActorIntent(intent) => v1::Event::EventActorIntent(v1::EventActorIntent {
			intent: convert_actor_intent_v2_to_v1(intent.intent),
		}),
		v2::Event::EventActorStateUpdate(update) => {
			v1::Event::EventActorStateUpdate(v1::EventActorStateUpdate {
				state: convert_actor_state_v2_to_v1(update.state),
			})
		}
		v2::Event::EventActorSetAlarm(alarm) => {
			v1::Event::EventActorSetAlarm(v1::EventActorSetAlarm {
				alarm_ts: alarm.alarm_ts,
			})
		}
	}
}

fn convert_event_v2_to_v3(event: v2::Event) -> v3::Event {
	match event {
		v2::Event::EventActorIntent(intent) => v3::Event::EventActorIntent(v3::EventActorIntent {
			intent: convert_actor_intent_v2_to_v3(intent.intent),
		}),
		v2::Event::EventActorStateUpdate(update) => {
			v3::Event::EventActorStateUpdate(v3::EventActorStateUpdate {
				state: convert_actor_state_v2_to_v3(update.state),
			})
		}
		v2::Event::EventActorSetAlarm(alarm) => {
			v3::Event::EventActorSetAlarm(v3::EventActorSetAlarm {
				alarm_ts: alarm.alarm_ts,
			})
		}
	}
}

fn convert_event_v3_to_v2(event: v3::Event) -> v2::Event {
	match event {
		v3::Event::EventActorIntent(intent) => v2::Event::EventActorIntent(v2::EventActorIntent {
			intent: convert_actor_intent_v3_to_v2(intent.intent),
		}),
		v3::Event::EventActorStateUpdate(update) => {
			v2::Event::EventActorStateUpdate(v2::EventActorStateUpdate {
				state: convert_actor_state_v3_to_v2(update.state),
			})
		}
		v3::Event::EventActorSetAlarm(alarm) => {
			v2::Event::EventActorSetAlarm(v2::EventActorSetAlarm {
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

// MARK: KV

fn convert_preloaded_kv_v1_to_v2(preloaded: v1::PreloadedKv) -> v2::PreloadedKv {
	v2::PreloadedKv {
		entries: preloaded
			.entries
			.into_iter()
			.map(convert_preloaded_kv_entry_v1_to_v2)
			.collect(),
		requested_get_keys: preloaded.requested_get_keys,
		requested_prefixes: preloaded.requested_prefixes,
	}
}

fn convert_preloaded_kv_v2_to_v1(preloaded: v2::PreloadedKv) -> v1::PreloadedKv {
	v1::PreloadedKv {
		entries: preloaded
			.entries
			.into_iter()
			.map(convert_preloaded_kv_entry_v2_to_v1)
			.collect(),
		requested_get_keys: preloaded.requested_get_keys,
		requested_prefixes: preloaded.requested_prefixes,
	}
}

fn convert_preloaded_kv_v2_to_v3(preloaded: v2::PreloadedKv) -> v3::PreloadedKv {
	v3::PreloadedKv {
		entries: preloaded
			.entries
			.into_iter()
			.map(convert_preloaded_kv_entry_v2_to_v3)
			.collect(),
		requested_get_keys: preloaded.requested_get_keys,
		requested_prefixes: preloaded.requested_prefixes,
	}
}

fn convert_preloaded_kv_v3_to_v2(preloaded: v3::PreloadedKv) -> v2::PreloadedKv {
	v2::PreloadedKv {
		entries: preloaded
			.entries
			.into_iter()
			.map(convert_preloaded_kv_entry_v3_to_v2)
			.collect(),
		requested_get_keys: preloaded.requested_get_keys,
		requested_prefixes: preloaded.requested_prefixes,
	}
}

fn convert_preloaded_kv_entry_v1_to_v2(entry: v1::PreloadedKvEntry) -> v2::PreloadedKvEntry {
	v2::PreloadedKvEntry {
		key: entry.key,
		value: entry.value,
		metadata: convert_kv_metadata_v1_to_v2(entry.metadata),
	}
}

fn convert_preloaded_kv_entry_v2_to_v1(entry: v2::PreloadedKvEntry) -> v1::PreloadedKvEntry {
	v1::PreloadedKvEntry {
		key: entry.key,
		value: entry.value,
		metadata: convert_kv_metadata_v2_to_v1(entry.metadata),
	}
}

fn convert_preloaded_kv_entry_v2_to_v3(entry: v2::PreloadedKvEntry) -> v3::PreloadedKvEntry {
	v3::PreloadedKvEntry {
		key: entry.key,
		value: entry.value,
		metadata: convert_kv_metadata_v2_to_v3(entry.metadata),
	}
}

fn convert_preloaded_kv_entry_v3_to_v2(entry: v3::PreloadedKvEntry) -> v2::PreloadedKvEntry {
	v2::PreloadedKvEntry {
		key: entry.key,
		value: entry.value,
		metadata: convert_kv_metadata_v3_to_v2(entry.metadata),
	}
}

fn convert_kv_metadata_v1_to_v2(metadata: v1::KvMetadata) -> v2::KvMetadata {
	v2::KvMetadata {
		version: metadata.version,
		update_ts: metadata.update_ts,
	}
}

fn convert_kv_metadata_v2_to_v1(metadata: v2::KvMetadata) -> v1::KvMetadata {
	v1::KvMetadata {
		version: metadata.version,
		update_ts: metadata.update_ts,
	}
}

fn convert_kv_metadata_v2_to_v3(metadata: v2::KvMetadata) -> v3::KvMetadata {
	v3::KvMetadata {
		version: metadata.version,
		update_ts: metadata.update_ts,
	}
}

fn convert_kv_metadata_v3_to_v2(metadata: v3::KvMetadata) -> v2::KvMetadata {
	v2::KvMetadata {
		version: metadata.version,
		update_ts: metadata.update_ts,
	}
}

fn convert_to_envoy_kv_response_v1_to_v2(response: v1::ToEnvoyKvResponse) -> v2::ToEnvoyKvResponse {
	v2::ToEnvoyKvResponse {
		request_id: response.request_id,
		data: convert_kv_response_data_v1_to_v2(response.data),
	}
}

fn convert_to_envoy_kv_response_v2_to_v1(response: v2::ToEnvoyKvResponse) -> v1::ToEnvoyKvResponse {
	v1::ToEnvoyKvResponse {
		request_id: response.request_id,
		data: convert_kv_response_data_v2_to_v1(response.data),
	}
}

fn convert_to_envoy_kv_response_v2_to_v3(response: v2::ToEnvoyKvResponse) -> v3::ToEnvoyKvResponse {
	v3::ToEnvoyKvResponse {
		request_id: response.request_id,
		data: convert_kv_response_data_v2_to_v3(response.data),
	}
}

fn convert_to_envoy_kv_response_v3_to_v2(response: v3::ToEnvoyKvResponse) -> v2::ToEnvoyKvResponse {
	v2::ToEnvoyKvResponse {
		request_id: response.request_id,
		data: convert_kv_response_data_v3_to_v2(response.data),
	}
}

fn convert_kv_response_data_v1_to_v2(data: v1::KvResponseData) -> v2::KvResponseData {
	match data {
		v1::KvResponseData::KvErrorResponse(err) => {
			v2::KvResponseData::KvErrorResponse(v2::KvErrorResponse {
				message: err.message,
			})
		}
		v1::KvResponseData::KvGetResponse(response) => {
			v2::KvResponseData::KvGetResponse(v2::KvGetResponse {
				keys: response.keys,
				values: response.values,
				metadata: response
					.metadata
					.into_iter()
					.map(convert_kv_metadata_v1_to_v2)
					.collect(),
			})
		}
		v1::KvResponseData::KvListResponse(response) => {
			v2::KvResponseData::KvListResponse(v2::KvListResponse {
				keys: response.keys,
				values: response.values,
				metadata: response
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
		v2::KvResponseData::KvGetResponse(response) => {
			v1::KvResponseData::KvGetResponse(v1::KvGetResponse {
				keys: response.keys,
				values: response.values,
				metadata: response
					.metadata
					.into_iter()
					.map(convert_kv_metadata_v2_to_v1)
					.collect(),
			})
		}
		v2::KvResponseData::KvListResponse(response) => {
			v1::KvResponseData::KvListResponse(v1::KvListResponse {
				keys: response.keys,
				values: response.values,
				metadata: response
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

fn convert_kv_response_data_v2_to_v3(data: v2::KvResponseData) -> v3::KvResponseData {
	match data {
		v2::KvResponseData::KvErrorResponse(err) => {
			v3::KvResponseData::KvErrorResponse(v3::KvErrorResponse {
				message: err.message,
			})
		}
		v2::KvResponseData::KvGetResponse(response) => {
			v3::KvResponseData::KvGetResponse(v3::KvGetResponse {
				keys: response.keys,
				values: response.values,
				metadata: response
					.metadata
					.into_iter()
					.map(convert_kv_metadata_v2_to_v3)
					.collect(),
			})
		}
		v2::KvResponseData::KvListResponse(response) => {
			v3::KvResponseData::KvListResponse(v3::KvListResponse {
				keys: response.keys,
				values: response.values,
				metadata: response
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
		v3::KvResponseData::KvGetResponse(response) => {
			v2::KvResponseData::KvGetResponse(v2::KvGetResponse {
				keys: response.keys,
				values: response.values,
				metadata: response
					.metadata
					.into_iter()
					.map(convert_kv_metadata_v3_to_v2)
					.collect(),
			})
		}
		v3::KvResponseData::KvListResponse(response) => {
			v2::KvResponseData::KvListResponse(v2::KvListResponse {
				keys: response.keys,
				values: response.values,
				metadata: response
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

fn convert_to_rivet_kv_request_v1_to_v2(request: v1::ToRivetKvRequest) -> v2::ToRivetKvRequest {
	v2::ToRivetKvRequest {
		actor_id: request.actor_id,
		request_id: request.request_id,
		data: convert_kv_request_data_v1_to_v2(request.data),
	}
}

fn convert_to_rivet_kv_request_v2_to_v1(request: v2::ToRivetKvRequest) -> v1::ToRivetKvRequest {
	v1::ToRivetKvRequest {
		actor_id: request.actor_id,
		request_id: request.request_id,
		data: convert_kv_request_data_v2_to_v1(request.data),
	}
}

fn convert_to_rivet_kv_request_v2_to_v3(request: v2::ToRivetKvRequest) -> v3::ToRivetKvRequest {
	v3::ToRivetKvRequest {
		actor_id: request.actor_id,
		request_id: request.request_id,
		data: convert_kv_request_data_v2_to_v3(request.data),
	}
}

fn convert_to_rivet_kv_request_v3_to_v2(request: v3::ToRivetKvRequest) -> v2::ToRivetKvRequest {
	v2::ToRivetKvRequest {
		actor_id: request.actor_id,
		request_id: request.request_id,
		data: convert_kv_request_data_v3_to_v2(request.data),
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
		v1::KvRequestData::KvDeleteRangeRequest(req) => {
			v2::KvRequestData::KvDeleteRangeRequest(v2::KvDeleteRangeRequest {
				start: req.start,
				end: req.end,
			})
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
		v2::KvRequestData::KvDeleteRangeRequest(req) => {
			v1::KvRequestData::KvDeleteRangeRequest(v1::KvDeleteRangeRequest {
				start: req.start,
				end: req.end,
			})
		}
		v2::KvRequestData::KvDropRequest => v1::KvRequestData::KvDropRequest,
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
		v2::KvRequestData::KvDeleteRangeRequest(req) => {
			v3::KvRequestData::KvDeleteRangeRequest(v3::KvDeleteRangeRequest {
				start: req.start,
				end: req.end,
			})
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
		v3::KvRequestData::KvDeleteRangeRequest(req) => {
			v2::KvRequestData::KvDeleteRangeRequest(v2::KvDeleteRangeRequest {
				start: req.start,
				end: req.end,
			})
		}
		v3::KvRequestData::KvDropRequest => v2::KvRequestData::KvDropRequest,
	}
}

fn convert_kv_list_query_v1_to_v2(query: v1::KvListQuery) -> v2::KvListQuery {
	match query {
		v1::KvListQuery::KvListAllQuery => v2::KvListQuery::KvListAllQuery,
		v1::KvListQuery::KvListRangeQuery(q) => {
			v2::KvListQuery::KvListRangeQuery(v2::KvListRangeQuery {
				start: q.start,
				end: q.end,
				exclusive: q.exclusive,
			})
		}
		v1::KvListQuery::KvListPrefixQuery(q) => {
			v2::KvListQuery::KvListPrefixQuery(v2::KvListPrefixQuery { key: q.key })
		}
	}
}

fn convert_kv_list_query_v2_to_v1(query: v2::KvListQuery) -> v1::KvListQuery {
	match query {
		v2::KvListQuery::KvListAllQuery => v1::KvListQuery::KvListAllQuery,
		v2::KvListQuery::KvListRangeQuery(q) => {
			v1::KvListQuery::KvListRangeQuery(v1::KvListRangeQuery {
				start: q.start,
				end: q.end,
				exclusive: q.exclusive,
			})
		}
		v2::KvListQuery::KvListPrefixQuery(q) => {
			v1::KvListQuery::KvListPrefixQuery(v1::KvListPrefixQuery { key: q.key })
		}
	}
}

fn convert_kv_list_query_v2_to_v3(query: v2::KvListQuery) -> v3::KvListQuery {
	match query {
		v2::KvListQuery::KvListAllQuery => v3::KvListQuery::KvListAllQuery,
		v2::KvListQuery::KvListRangeQuery(q) => {
			v3::KvListQuery::KvListRangeQuery(v3::KvListRangeQuery {
				start: q.start,
				end: q.end,
				exclusive: q.exclusive,
			})
		}
		v2::KvListQuery::KvListPrefixQuery(q) => {
			v3::KvListQuery::KvListPrefixQuery(v3::KvListPrefixQuery { key: q.key })
		}
	}
}

fn convert_kv_list_query_v3_to_v2(query: v3::KvListQuery) -> v2::KvListQuery {
	match query {
		v3::KvListQuery::KvListAllQuery => v2::KvListQuery::KvListAllQuery,
		v3::KvListQuery::KvListRangeQuery(q) => {
			v2::KvListQuery::KvListRangeQuery(v2::KvListRangeQuery {
				start: q.start,
				end: q.end,
				exclusive: q.exclusive,
			})
		}
		v3::KvListQuery::KvListPrefixQuery(q) => {
			v2::KvListQuery::KvListPrefixQuery(v2::KvListPrefixQuery { key: q.key })
		}
	}
}

// MARK: ToEnvoy small types

fn convert_to_envoy_init_v1_to_v2(init: v1::ToEnvoyInit) -> v2::ToEnvoyInit {
	v2::ToEnvoyInit {
		metadata: convert_protocol_metadata_v1_to_v2(init.metadata),
	}
}

fn convert_to_envoy_init_v2_to_v1(init: v2::ToEnvoyInit) -> v1::ToEnvoyInit {
	v1::ToEnvoyInit {
		metadata: convert_protocol_metadata_v2_to_v1(init.metadata),
	}
}

fn convert_to_envoy_init_v2_to_v3(init: v2::ToEnvoyInit) -> v3::ToEnvoyInit {
	v3::ToEnvoyInit {
		metadata: convert_protocol_metadata_v2_to_v3(init.metadata),
	}
}

fn convert_to_envoy_init_v3_to_v2(init: v3::ToEnvoyInit) -> v2::ToEnvoyInit {
	v2::ToEnvoyInit {
		metadata: convert_protocol_metadata_v3_to_v2(init.metadata),
	}
}

fn convert_protocol_metadata_v1_to_v2(metadata: v1::ProtocolMetadata) -> v2::ProtocolMetadata {
	v2::ProtocolMetadata {
		envoy_lost_threshold: metadata.envoy_lost_threshold,
		actor_stop_threshold: metadata.actor_stop_threshold,
		max_response_payload_size: metadata.max_response_payload_size,
	}
}

fn convert_protocol_metadata_v2_to_v1(metadata: v2::ProtocolMetadata) -> v1::ProtocolMetadata {
	v1::ProtocolMetadata {
		envoy_lost_threshold: metadata.envoy_lost_threshold,
		actor_stop_threshold: metadata.actor_stop_threshold,
		max_response_payload_size: metadata.max_response_payload_size,
	}
}

fn convert_protocol_metadata_v2_to_v3(metadata: v2::ProtocolMetadata) -> v3::ProtocolMetadata {
	v3::ProtocolMetadata {
		envoy_lost_threshold: metadata.envoy_lost_threshold,
		actor_stop_threshold: metadata.actor_stop_threshold,
		max_response_payload_size: metadata.max_response_payload_size,
	}
}

fn convert_protocol_metadata_v3_to_v2(metadata: v3::ProtocolMetadata) -> v2::ProtocolMetadata {
	v2::ProtocolMetadata {
		envoy_lost_threshold: metadata.envoy_lost_threshold,
		actor_stop_threshold: metadata.actor_stop_threshold,
		max_response_payload_size: metadata.max_response_payload_size,
	}
}

fn convert_to_envoy_ack_events_v1_to_v2(ack: v1::ToEnvoyAckEvents) -> v2::ToEnvoyAckEvents {
	v2::ToEnvoyAckEvents {
		last_event_checkpoints: ack
			.last_event_checkpoints
			.into_iter()
			.map(convert_actor_checkpoint_v1_to_v2)
			.collect(),
	}
}

fn convert_to_envoy_ack_events_v2_to_v1(ack: v2::ToEnvoyAckEvents) -> v1::ToEnvoyAckEvents {
	v1::ToEnvoyAckEvents {
		last_event_checkpoints: ack
			.last_event_checkpoints
			.into_iter()
			.map(convert_actor_checkpoint_v2_to_v1)
			.collect(),
	}
}

fn convert_to_envoy_ack_events_v2_to_v3(ack: v2::ToEnvoyAckEvents) -> v3::ToEnvoyAckEvents {
	v3::ToEnvoyAckEvents {
		last_event_checkpoints: ack
			.last_event_checkpoints
			.into_iter()
			.map(convert_actor_checkpoint_v2_to_v3)
			.collect(),
	}
}

fn convert_to_envoy_ack_events_v3_to_v2(ack: v3::ToEnvoyAckEvents) -> v2::ToEnvoyAckEvents {
	v2::ToEnvoyAckEvents {
		last_event_checkpoints: ack
			.last_event_checkpoints
			.into_iter()
			.map(convert_actor_checkpoint_v3_to_v2)
			.collect(),
	}
}

fn convert_to_envoy_ping_v1_to_v2(ping: v1::ToEnvoyPing) -> v2::ToEnvoyPing {
	v2::ToEnvoyPing { ts: ping.ts }
}

fn convert_to_envoy_ping_v2_to_v1(ping: v2::ToEnvoyPing) -> v1::ToEnvoyPing {
	v1::ToEnvoyPing { ts: ping.ts }
}

fn convert_to_envoy_ping_v2_to_v3(ping: v2::ToEnvoyPing) -> v3::ToEnvoyPing {
	v3::ToEnvoyPing { ts: ping.ts }
}

fn convert_to_envoy_ping_v3_to_v2(ping: v3::ToEnvoyPing) -> v2::ToEnvoyPing {
	v2::ToEnvoyPing { ts: ping.ts }
}

fn convert_to_envoy_conn_ping_v1_to_v2(ping: v1::ToEnvoyConnPing) -> v2::ToEnvoyConnPing {
	v2::ToEnvoyConnPing {
		gateway_id: ping.gateway_id,
		request_id: ping.request_id,
		ts: ping.ts,
	}
}

fn convert_to_envoy_conn_ping_v2_to_v1(ping: v2::ToEnvoyConnPing) -> v1::ToEnvoyConnPing {
	v1::ToEnvoyConnPing {
		gateway_id: ping.gateway_id,
		request_id: ping.request_id,
		ts: ping.ts,
	}
}

fn convert_to_envoy_conn_ping_v2_to_v3(ping: v2::ToEnvoyConnPing) -> v3::ToEnvoyConnPing {
	v3::ToEnvoyConnPing {
		gateway_id: ping.gateway_id,
		request_id: ping.request_id,
		ts: ping.ts,
	}
}

fn convert_to_envoy_conn_ping_v3_to_v2(ping: v3::ToEnvoyConnPing) -> v2::ToEnvoyConnPing {
	v2::ToEnvoyConnPing {
		gateway_id: ping.gateway_id,
		request_id: ping.request_id,
		ts: ping.ts,
	}
}

// MARK: ToRivet small types

fn convert_to_rivet_metadata_v1_to_v2(metadata: v1::ToRivetMetadata) -> v2::ToRivetMetadata {
	v2::ToRivetMetadata {
		prepopulate_actor_names: metadata.prepopulate_actor_names.map(|map| {
			map.into_iter()
				.map(|(k, v)| (k, convert_actor_name_v1_to_v2(v)))
				.collect()
		}),
		metadata: metadata.metadata,
	}
}

fn convert_to_rivet_metadata_v2_to_v1(metadata: v2::ToRivetMetadata) -> v1::ToRivetMetadata {
	v1::ToRivetMetadata {
		prepopulate_actor_names: metadata.prepopulate_actor_names.map(|map| {
			map.into_iter()
				.map(|(k, v)| (k, convert_actor_name_v2_to_v1(v)))
				.collect()
		}),
		metadata: metadata.metadata,
	}
}

fn convert_to_rivet_metadata_v2_to_v3(metadata: v2::ToRivetMetadata) -> v3::ToRivetMetadata {
	v3::ToRivetMetadata {
		prepopulate_actor_names: metadata.prepopulate_actor_names.map(|map| {
			map.into_iter()
				.map(|(k, v)| (k, convert_actor_name_v2_to_v3(v)))
				.collect()
		}),
		metadata: metadata.metadata,
	}
}

fn convert_to_rivet_metadata_v3_to_v2(metadata: v3::ToRivetMetadata) -> v2::ToRivetMetadata {
	v2::ToRivetMetadata {
		prepopulate_actor_names: metadata.prepopulate_actor_names.map(|map| {
			map.into_iter()
				.map(|(k, v)| (k, convert_actor_name_v3_to_v2(v)))
				.collect()
		}),
		metadata: metadata.metadata,
	}
}

fn convert_actor_name_v1_to_v2(name: v1::ActorName) -> v2::ActorName {
	v2::ActorName {
		metadata: name.metadata,
	}
}

fn convert_actor_name_v2_to_v1(name: v2::ActorName) -> v1::ActorName {
	v1::ActorName {
		metadata: name.metadata,
	}
}

fn convert_actor_name_v2_to_v3(name: v2::ActorName) -> v3::ActorName {
	v3::ActorName {
		metadata: name.metadata,
	}
}

fn convert_actor_name_v3_to_v2(name: v3::ActorName) -> v2::ActorName {
	v2::ActorName {
		metadata: name.metadata,
	}
}

fn convert_to_rivet_ack_commands_v1_to_v2(ack: v1::ToRivetAckCommands) -> v2::ToRivetAckCommands {
	v2::ToRivetAckCommands {
		last_command_checkpoints: ack
			.last_command_checkpoints
			.into_iter()
			.map(convert_actor_checkpoint_v1_to_v2)
			.collect(),
	}
}

fn convert_to_rivet_ack_commands_v2_to_v1(ack: v2::ToRivetAckCommands) -> v1::ToRivetAckCommands {
	v1::ToRivetAckCommands {
		last_command_checkpoints: ack
			.last_command_checkpoints
			.into_iter()
			.map(convert_actor_checkpoint_v2_to_v1)
			.collect(),
	}
}

fn convert_to_rivet_ack_commands_v2_to_v3(ack: v2::ToRivetAckCommands) -> v3::ToRivetAckCommands {
	v3::ToRivetAckCommands {
		last_command_checkpoints: ack
			.last_command_checkpoints
			.into_iter()
			.map(convert_actor_checkpoint_v2_to_v3)
			.collect(),
	}
}

fn convert_to_rivet_ack_commands_v3_to_v2(ack: v3::ToRivetAckCommands) -> v2::ToRivetAckCommands {
	v2::ToRivetAckCommands {
		last_command_checkpoints: ack
			.last_command_checkpoints
			.into_iter()
			.map(convert_actor_checkpoint_v3_to_v2)
			.collect(),
	}
}

fn convert_to_rivet_pong_v1_to_v2(pong: v1::ToRivetPong) -> v2::ToRivetPong {
	v2::ToRivetPong { ts: pong.ts }
}

fn convert_to_rivet_pong_v2_to_v1(pong: v2::ToRivetPong) -> v1::ToRivetPong {
	v1::ToRivetPong { ts: pong.ts }
}

fn convert_to_rivet_pong_v2_to_v3(pong: v2::ToRivetPong) -> v3::ToRivetPong {
	v3::ToRivetPong { ts: pong.ts }
}

fn convert_to_rivet_pong_v3_to_v2(pong: v3::ToRivetPong) -> v2::ToRivetPong {
	v2::ToRivetPong { ts: pong.ts }
}

// MARK: ToGateway pong

fn convert_to_gateway_pong_v1_to_v2(pong: v1::ToGatewayPong) -> v2::ToGatewayPong {
	v2::ToGatewayPong {
		request_id: pong.request_id,
		ts: pong.ts,
	}
}

fn convert_to_gateway_pong_v2_to_v1(pong: v2::ToGatewayPong) -> v1::ToGatewayPong {
	v1::ToGatewayPong {
		request_id: pong.request_id,
		ts: pong.ts,
	}
}

fn convert_to_gateway_pong_v2_to_v3(pong: v2::ToGatewayPong) -> v3::ToGatewayPong {
	v3::ToGatewayPong {
		request_id: pong.request_id,
		ts: pong.ts,
	}
}

fn convert_to_gateway_pong_v3_to_v2(pong: v3::ToGatewayPong) -> v2::ToGatewayPong {
	v2::ToGatewayPong {
		request_id: pong.request_id,
		ts: pong.ts,
	}
}

// MARK: Tunnel messages

fn convert_message_id_v1_to_v2(message_id: v1::MessageId) -> v2::MessageId {
	v2::MessageId {
		gateway_id: message_id.gateway_id,
		request_id: message_id.request_id,
		message_index: message_id.message_index,
	}
}

fn convert_message_id_v2_to_v1(message_id: v2::MessageId) -> v1::MessageId {
	v1::MessageId {
		gateway_id: message_id.gateway_id,
		request_id: message_id.request_id,
		message_index: message_id.message_index,
	}
}

fn convert_message_id_v2_to_v3(message_id: v2::MessageId) -> v3::MessageId {
	v3::MessageId {
		gateway_id: message_id.gateway_id,
		request_id: message_id.request_id,
		message_index: message_id.message_index,
	}
}

fn convert_message_id_v3_to_v2(message_id: v3::MessageId) -> v2::MessageId {
	v2::MessageId {
		gateway_id: message_id.gateway_id,
		request_id: message_id.request_id,
		message_index: message_id.message_index,
	}
}

fn convert_to_envoy_tunnel_message_v1_to_v2(
	message: v1::ToEnvoyTunnelMessage,
) -> v2::ToEnvoyTunnelMessage {
	v2::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v1_to_v2(message.message_id),
		message_kind: convert_to_envoy_tunnel_message_kind_v1_to_v2(message.message_kind),
	}
}

fn convert_to_envoy_tunnel_message_v2_to_v1(
	message: v2::ToEnvoyTunnelMessage,
) -> v1::ToEnvoyTunnelMessage {
	v1::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v2_to_v1(message.message_id),
		message_kind: convert_to_envoy_tunnel_message_kind_v2_to_v1(message.message_kind),
	}
}

fn convert_to_envoy_tunnel_message_v2_to_v3(
	message: v2::ToEnvoyTunnelMessage,
) -> v3::ToEnvoyTunnelMessage {
	v3::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v2_to_v3(message.message_id),
		message_kind: convert_to_envoy_tunnel_message_kind_v2_to_v3(message.message_kind),
	}
}

fn convert_to_envoy_tunnel_message_v3_to_v2(
	message: v3::ToEnvoyTunnelMessage,
) -> v2::ToEnvoyTunnelMessage {
	v2::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v3_to_v2(message.message_id),
		message_kind: convert_to_envoy_tunnel_message_kind_v3_to_v2(message.message_kind),
	}
}

fn convert_to_envoy_tunnel_message_kind_v1_to_v2(
	kind: v1::ToEnvoyTunnelMessageKind,
) -> v2::ToEnvoyTunnelMessageKind {
	match kind {
		v1::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(start) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v2::ToEnvoyRequestStart {
				actor_id: start.actor_id,
				method: start.method,
				path: start.path,
				headers: start.headers,
				body: start.body,
				stream: start.stream,
			})
		}
		v1::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(chunk) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v2::ToEnvoyRequestChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v1::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
		}
		v1::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(open) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v2::ToEnvoyWebSocketOpen {
				actor_id: open.actor_id,
				path: open.path,
				headers: open.headers,
			})
		}
		v1::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(message) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v2::ToEnvoyWebSocketMessage {
				data: message.data,
				binary: message.binary,
			})
		}
		v1::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(close) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v2::ToEnvoyWebSocketClose {
				code: close.code,
				reason: close.reason,
			})
		}
	}
}

fn convert_to_envoy_tunnel_message_kind_v2_to_v1(
	kind: v2::ToEnvoyTunnelMessageKind,
) -> v1::ToEnvoyTunnelMessageKind {
	match kind {
		v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(start) => {
			v1::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v1::ToEnvoyRequestStart {
				actor_id: start.actor_id,
				method: start.method,
				path: start.path,
				headers: start.headers,
				body: start.body,
				stream: start.stream,
			})
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(chunk) => {
			v1::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v1::ToEnvoyRequestChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => {
			v1::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(open) => {
			v1::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v1::ToEnvoyWebSocketOpen {
				actor_id: open.actor_id,
				path: open.path,
				headers: open.headers,
			})
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(message) => {
			v1::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v1::ToEnvoyWebSocketMessage {
				data: message.data,
				binary: message.binary,
			})
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(close) => {
			v1::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v1::ToEnvoyWebSocketClose {
				code: close.code,
				reason: close.reason,
			})
		}
	}
}

fn convert_to_envoy_tunnel_message_kind_v2_to_v3(
	kind: v2::ToEnvoyTunnelMessageKind,
) -> v3::ToEnvoyTunnelMessageKind {
	match kind {
		v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(start) => {
			v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v3::ToEnvoyRequestStart {
				actor_id: start.actor_id,
				method: start.method,
				path: start.path,
				headers: start.headers,
				body: start.body,
				stream: start.stream,
			})
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(chunk) => {
			v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v3::ToEnvoyRequestChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => {
			v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(open) => {
			v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v3::ToEnvoyWebSocketOpen {
				actor_id: open.actor_id,
				path: open.path,
				headers: open.headers,
			})
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(message) => {
			v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v3::ToEnvoyWebSocketMessage {
				data: message.data,
				binary: message.binary,
			})
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(close) => {
			v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v3::ToEnvoyWebSocketClose {
				code: close.code,
				reason: close.reason,
			})
		}
	}
}

fn convert_to_envoy_tunnel_message_kind_v3_to_v2(
	kind: v3::ToEnvoyTunnelMessageKind,
) -> v2::ToEnvoyTunnelMessageKind {
	match kind {
		v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(start) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v2::ToEnvoyRequestStart {
				actor_id: start.actor_id,
				method: start.method,
				path: start.path,
				headers: start.headers,
				body: start.body,
				stream: start.stream,
			})
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(chunk) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v2::ToEnvoyRequestChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(open) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v2::ToEnvoyWebSocketOpen {
				actor_id: open.actor_id,
				path: open.path,
				headers: open.headers,
			})
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(message) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v2::ToEnvoyWebSocketMessage {
				data: message.data,
				binary: message.binary,
			})
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(close) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v2::ToEnvoyWebSocketClose {
				code: close.code,
				reason: close.reason,
			})
		}
	}
}

fn convert_to_rivet_tunnel_message_v1_to_v2(
	message: v1::ToRivetTunnelMessage,
) -> v2::ToRivetTunnelMessage {
	v2::ToRivetTunnelMessage {
		message_id: convert_message_id_v1_to_v2(message.message_id),
		message_kind: convert_to_rivet_tunnel_message_kind_v1_to_v2(message.message_kind),
	}
}

fn convert_to_rivet_tunnel_message_v2_to_v1(
	message: v2::ToRivetTunnelMessage,
) -> v1::ToRivetTunnelMessage {
	v1::ToRivetTunnelMessage {
		message_id: convert_message_id_v2_to_v1(message.message_id),
		message_kind: convert_to_rivet_tunnel_message_kind_v2_to_v1(message.message_kind),
	}
}

fn convert_to_rivet_tunnel_message_v2_to_v3(
	message: v2::ToRivetTunnelMessage,
) -> v3::ToRivetTunnelMessage {
	v3::ToRivetTunnelMessage {
		message_id: convert_message_id_v2_to_v3(message.message_id),
		message_kind: convert_to_rivet_tunnel_message_kind_v2_to_v3(message.message_kind),
	}
}

fn convert_to_rivet_tunnel_message_v3_to_v2(
	message: v3::ToRivetTunnelMessage,
) -> v2::ToRivetTunnelMessage {
	v2::ToRivetTunnelMessage {
		message_id: convert_message_id_v3_to_v2(message.message_id),
		message_kind: convert_to_rivet_tunnel_message_kind_v3_to_v2(message.message_kind),
	}
}

fn convert_to_rivet_tunnel_message_kind_v1_to_v2(
	kind: v1::ToRivetTunnelMessageKind,
) -> v2::ToRivetTunnelMessageKind {
	match kind {
		v1::ToRivetTunnelMessageKind::ToRivetResponseStart(start) => {
			v2::ToRivetTunnelMessageKind::ToRivetResponseStart(v2::ToRivetResponseStart {
				status: start.status,
				headers: start.headers,
				body: start.body,
				stream: start.stream,
			})
		}
		v1::ToRivetTunnelMessageKind::ToRivetResponseChunk(chunk) => {
			v2::ToRivetTunnelMessageKind::ToRivetResponseChunk(v2::ToRivetResponseChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v1::ToRivetTunnelMessageKind::ToRivetResponseAbort => {
			v2::ToRivetTunnelMessageKind::ToRivetResponseAbort
		}
		v1::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(open) => {
			v2::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v2::ToRivetWebSocketOpen {
				can_hibernate: open.can_hibernate,
			})
		}
		v1::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(message) => {
			v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v2::ToRivetWebSocketMessage {
				data: message.data,
				binary: message.binary,
			})
		}
		v1::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(ack) => {
			v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				v2::ToRivetWebSocketMessageAck { index: ack.index },
			)
		}
		v1::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close) => {
			v2::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v2::ToRivetWebSocketClose {
				code: close.code,
				reason: close.reason,
				hibernate: close.hibernate,
			})
		}
	}
}

fn convert_to_rivet_tunnel_message_kind_v2_to_v1(
	kind: v2::ToRivetTunnelMessageKind,
) -> v1::ToRivetTunnelMessageKind {
	match kind {
		v2::ToRivetTunnelMessageKind::ToRivetResponseStart(start) => {
			v1::ToRivetTunnelMessageKind::ToRivetResponseStart(v1::ToRivetResponseStart {
				status: start.status,
				headers: start.headers,
				body: start.body,
				stream: start.stream,
			})
		}
		v2::ToRivetTunnelMessageKind::ToRivetResponseChunk(chunk) => {
			v1::ToRivetTunnelMessageKind::ToRivetResponseChunk(v1::ToRivetResponseChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v2::ToRivetTunnelMessageKind::ToRivetResponseAbort => {
			v1::ToRivetTunnelMessageKind::ToRivetResponseAbort
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(open) => {
			v1::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v1::ToRivetWebSocketOpen {
				can_hibernate: open.can_hibernate,
			})
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(message) => {
			v1::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v1::ToRivetWebSocketMessage {
				data: message.data,
				binary: message.binary,
			})
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(ack) => {
			v1::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				v1::ToRivetWebSocketMessageAck { index: ack.index },
			)
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close) => {
			v1::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v1::ToRivetWebSocketClose {
				code: close.code,
				reason: close.reason,
				hibernate: close.hibernate,
			})
		}
	}
}

fn convert_to_rivet_tunnel_message_kind_v2_to_v3(
	kind: v2::ToRivetTunnelMessageKind,
) -> v3::ToRivetTunnelMessageKind {
	match kind {
		v2::ToRivetTunnelMessageKind::ToRivetResponseStart(start) => {
			v3::ToRivetTunnelMessageKind::ToRivetResponseStart(v3::ToRivetResponseStart {
				status: start.status,
				headers: start.headers,
				body: start.body,
				stream: start.stream,
			})
		}
		v2::ToRivetTunnelMessageKind::ToRivetResponseChunk(chunk) => {
			v3::ToRivetTunnelMessageKind::ToRivetResponseChunk(v3::ToRivetResponseChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v2::ToRivetTunnelMessageKind::ToRivetResponseAbort => {
			v3::ToRivetTunnelMessageKind::ToRivetResponseAbort
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(open) => {
			v3::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v3::ToRivetWebSocketOpen {
				can_hibernate: open.can_hibernate,
			})
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(message) => {
			v3::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v3::ToRivetWebSocketMessage {
				data: message.data,
				binary: message.binary,
			})
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(ack) => {
			v3::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				v3::ToRivetWebSocketMessageAck { index: ack.index },
			)
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close) => {
			v3::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v3::ToRivetWebSocketClose {
				code: close.code,
				reason: close.reason,
				hibernate: close.hibernate,
			})
		}
	}
}

fn convert_to_rivet_tunnel_message_kind_v3_to_v2(
	kind: v3::ToRivetTunnelMessageKind,
) -> v2::ToRivetTunnelMessageKind {
	match kind {
		v3::ToRivetTunnelMessageKind::ToRivetResponseStart(start) => {
			v2::ToRivetTunnelMessageKind::ToRivetResponseStart(v2::ToRivetResponseStart {
				status: start.status,
				headers: start.headers,
				body: start.body,
				stream: start.stream,
			})
		}
		v3::ToRivetTunnelMessageKind::ToRivetResponseChunk(chunk) => {
			v2::ToRivetTunnelMessageKind::ToRivetResponseChunk(v2::ToRivetResponseChunk {
				body: chunk.body,
				finish: chunk.finish,
			})
		}
		v3::ToRivetTunnelMessageKind::ToRivetResponseAbort => {
			v2::ToRivetTunnelMessageKind::ToRivetResponseAbort
		}
		v3::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(open) => {
			v2::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v2::ToRivetWebSocketOpen {
				can_hibernate: open.can_hibernate,
			})
		}
		v3::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(message) => {
			v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v2::ToRivetWebSocketMessage {
				data: message.data,
				binary: message.binary,
			})
		}
		v3::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(ack) => {
			v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				v2::ToRivetWebSocketMessageAck { index: ack.index },
			)
		}
		v3::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close) => {
			v2::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v2::ToRivetWebSocketClose {
				code: close.code,
				reason: close.reason,
				hibernate: close.hibernate,
			})
		}
	}
}

// MARK: SQLite

fn convert_sqlite_meta_v2_to_v3(meta: v2::SqliteMeta) -> v3::SqliteMeta {
	v3::SqliteMeta {
		generation: meta.generation,
		head_txid: meta.head_txid,
		materialized_txid: meta.materialized_txid,
		db_size_pages: meta.db_size_pages,
		page_size: meta.page_size,
		creation_ts_ms: meta.creation_ts_ms,
		max_delta_bytes: meta.max_delta_bytes,
	}
}

fn convert_sqlite_meta_v3_to_v2(meta: v3::SqliteMeta) -> v2::SqliteMeta {
	v2::SqliteMeta {
		schema_version: 2,
		generation: meta.generation,
		head_txid: meta.head_txid,
		materialized_txid: meta.materialized_txid,
		db_size_pages: meta.db_size_pages,
		page_size: meta.page_size,
		creation_ts_ms: meta.creation_ts_ms,
		max_delta_bytes: meta.max_delta_bytes,
	}
}

fn convert_sqlite_fetched_page_v2_to_v3(page: v2::SqliteFetchedPage) -> v3::SqliteFetchedPage {
	v3::SqliteFetchedPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}

fn convert_sqlite_fetched_page_v3_to_v2(page: v3::SqliteFetchedPage) -> v2::SqliteFetchedPage {
	v2::SqliteFetchedPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}

fn convert_sqlite_dirty_page_v2_to_v3(page: v2::SqliteDirtyPage) -> v3::SqliteDirtyPage {
	v3::SqliteDirtyPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}

fn convert_sqlite_dirty_page_v3_to_v2(page: v3::SqliteDirtyPage) -> v2::SqliteDirtyPage {
	v2::SqliteDirtyPage {
		pgno: page.pgno,
		bytes: page.bytes,
	}
}

fn convert_sqlite_startup_data_v2_to_v3(data: v2::SqliteStartupData) -> v3::SqliteStartupData {
	v3::SqliteStartupData {
		generation: data.generation,
		meta: convert_sqlite_meta_v2_to_v3(data.meta),
		preloaded_pages: data
			.preloaded_pages
			.into_iter()
			.map(convert_sqlite_fetched_page_v2_to_v3)
			.collect(),
	}
}

fn convert_sqlite_startup_data_v3_to_v2(data: v3::SqliteStartupData) -> v2::SqliteStartupData {
	v2::SqliteStartupData {
		generation: data.generation,
		meta: convert_sqlite_meta_v3_to_v2(data.meta),
		preloaded_pages: data
			.preloaded_pages
			.into_iter()
			.map(convert_sqlite_fetched_page_v3_to_v2)
			.collect(),
	}
}

fn convert_sqlite_fence_mismatch_v2_to_v3(
	data: v2::SqliteFenceMismatch,
) -> v3::SqliteFenceMismatch {
	v3::SqliteFenceMismatch {
		actual_meta: convert_sqlite_meta_v2_to_v3(data.actual_meta),
		reason: data.reason,
	}
}

fn convert_sqlite_fence_mismatch_v3_to_v2(
	data: v3::SqliteFenceMismatch,
) -> v2::SqliteFenceMismatch {
	v2::SqliteFenceMismatch {
		actual_meta: convert_sqlite_meta_v3_to_v2(data.actual_meta),
		reason: data.reason,
	}
}

fn convert_sqlite_error_response_v2_to_v3(err: v2::SqliteErrorResponse) -> v3::SqliteErrorResponse {
	v3::SqliteErrorResponse {
		message: err.message,
	}
}

fn convert_sqlite_error_response_v3_to_v2(err: v3::SqliteErrorResponse) -> v2::SqliteErrorResponse {
	v2::SqliteErrorResponse {
		message: err.message,
	}
}

fn convert_sqlite_get_pages_request_v2_to_v3(
	request: v2::SqliteGetPagesRequest,
) -> v3::SqliteGetPagesRequest {
	v3::SqliteGetPagesRequest {
		actor_id: request.actor_id,
		generation: request.generation,
		pgnos: request.pgnos,
	}
}

fn convert_sqlite_get_pages_request_v3_to_v2(
	request: v3::SqliteGetPagesRequest,
) -> v2::SqliteGetPagesRequest {
	v2::SqliteGetPagesRequest {
		actor_id: request.actor_id,
		generation: request.generation,
		pgnos: request.pgnos,
	}
}

fn convert_sqlite_commit_request_v2_to_v3(
	request: v2::SqliteCommitRequest,
) -> v3::SqliteCommitRequest {
	v3::SqliteCommitRequest {
		actor_id: request.actor_id,
		generation: request.generation,
		expected_head_txid: request.expected_head_txid,
		dirty_pages: request
			.dirty_pages
			.into_iter()
			.map(convert_sqlite_dirty_page_v2_to_v3)
			.collect(),
		new_db_size_pages: request.new_db_size_pages,
	}
}

fn convert_sqlite_commit_request_v3_to_v2(
	request: v3::SqliteCommitRequest,
) -> v2::SqliteCommitRequest {
	v2::SqliteCommitRequest {
		actor_id: request.actor_id,
		generation: request.generation,
		expected_head_txid: request.expected_head_txid,
		dirty_pages: request
			.dirty_pages
			.into_iter()
			.map(convert_sqlite_dirty_page_v3_to_v2)
			.collect(),
		new_db_size_pages: request.new_db_size_pages,
	}
}

fn convert_sqlite_commit_stage_begin_request_v2_to_v3(
	request: v2::SqliteCommitStageBeginRequest,
) -> v3::SqliteCommitStageBeginRequest {
	v3::SqliteCommitStageBeginRequest {
		actor_id: request.actor_id,
		generation: request.generation,
	}
}

fn convert_sqlite_commit_stage_begin_request_v3_to_v2(
	request: v3::SqliteCommitStageBeginRequest,
) -> v2::SqliteCommitStageBeginRequest {
	v2::SqliteCommitStageBeginRequest {
		actor_id: request.actor_id,
		generation: request.generation,
	}
}

fn convert_sqlite_commit_stage_request_v2_to_v3(
	request: v2::SqliteCommitStageRequest,
) -> v3::SqliteCommitStageRequest {
	v3::SqliteCommitStageRequest {
		actor_id: request.actor_id,
		generation: request.generation,
		txid: request.txid,
		chunk_idx: request.chunk_idx,
		bytes: request.bytes,
		is_last: request.is_last,
	}
}

fn convert_sqlite_commit_stage_request_v3_to_v2(
	request: v3::SqliteCommitStageRequest,
) -> v2::SqliteCommitStageRequest {
	v2::SqliteCommitStageRequest {
		actor_id: request.actor_id,
		generation: request.generation,
		txid: request.txid,
		chunk_idx: request.chunk_idx,
		bytes: request.bytes,
		is_last: request.is_last,
	}
}

fn convert_sqlite_commit_finalize_request_v2_to_v3(
	request: v2::SqliteCommitFinalizeRequest,
) -> v3::SqliteCommitFinalizeRequest {
	v3::SqliteCommitFinalizeRequest {
		actor_id: request.actor_id,
		generation: request.generation,
		expected_head_txid: request.expected_head_txid,
		txid: request.txid,
		new_db_size_pages: request.new_db_size_pages,
	}
}

fn convert_sqlite_commit_finalize_request_v3_to_v2(
	request: v3::SqliteCommitFinalizeRequest,
) -> v2::SqliteCommitFinalizeRequest {
	v2::SqliteCommitFinalizeRequest {
		actor_id: request.actor_id,
		generation: request.generation,
		expected_head_txid: request.expected_head_txid,
		txid: request.txid,
		new_db_size_pages: request.new_db_size_pages,
	}
}

fn convert_sqlite_get_pages_response_v2_to_v3(
	data: v2::SqliteGetPagesResponse,
) -> v3::SqliteGetPagesResponse {
	match data {
		v2::SqliteGetPagesResponse::SqliteGetPagesOk(ok) => {
			v3::SqliteGetPagesResponse::SqliteGetPagesOk(v3::SqliteGetPagesOk {
				pages: ok
					.pages
					.into_iter()
					.map(convert_sqlite_fetched_page_v2_to_v3)
					.collect(),
				meta: convert_sqlite_meta_v2_to_v3(ok.meta),
			})
		}
		v2::SqliteGetPagesResponse::SqliteFenceMismatch(mismatch) => {
			v3::SqliteGetPagesResponse::SqliteFenceMismatch(convert_sqlite_fence_mismatch_v2_to_v3(
				mismatch,
			))
		}
		v2::SqliteGetPagesResponse::SqliteErrorResponse(err) => {
			v3::SqliteGetPagesResponse::SqliteErrorResponse(convert_sqlite_error_response_v2_to_v3(
				err,
			))
		}
	}
}

fn convert_sqlite_get_pages_response_v3_to_v2(
	data: v3::SqliteGetPagesResponse,
) -> v2::SqliteGetPagesResponse {
	match data {
		v3::SqliteGetPagesResponse::SqliteGetPagesOk(ok) => {
			v2::SqliteGetPagesResponse::SqliteGetPagesOk(v2::SqliteGetPagesOk {
				pages: ok
					.pages
					.into_iter()
					.map(convert_sqlite_fetched_page_v3_to_v2)
					.collect(),
				meta: convert_sqlite_meta_v3_to_v2(ok.meta),
			})
		}
		v3::SqliteGetPagesResponse::SqliteFenceMismatch(mismatch) => {
			v2::SqliteGetPagesResponse::SqliteFenceMismatch(convert_sqlite_fence_mismatch_v3_to_v2(
				mismatch,
			))
		}
		v3::SqliteGetPagesResponse::SqliteErrorResponse(err) => {
			v2::SqliteGetPagesResponse::SqliteErrorResponse(convert_sqlite_error_response_v3_to_v2(
				err,
			))
		}
	}
}

fn convert_sqlite_commit_response_v2_to_v3(
	data: v2::SqliteCommitResponse,
) -> v3::SqliteCommitResponse {
	match data {
		v2::SqliteCommitResponse::SqliteCommitOk(ok) => {
			v3::SqliteCommitResponse::SqliteCommitOk(v3::SqliteCommitOk {
				new_head_txid: ok.new_head_txid,
				meta: convert_sqlite_meta_v2_to_v3(ok.meta),
			})
		}
		v2::SqliteCommitResponse::SqliteFenceMismatch(mismatch) => {
			v3::SqliteCommitResponse::SqliteFenceMismatch(convert_sqlite_fence_mismatch_v2_to_v3(
				mismatch,
			))
		}
		v2::SqliteCommitResponse::SqliteCommitTooLarge(too_large) => {
			v3::SqliteCommitResponse::SqliteCommitTooLarge(v3::SqliteCommitTooLarge {
				actual_size_bytes: too_large.actual_size_bytes,
				max_size_bytes: too_large.max_size_bytes,
			})
		}
		v2::SqliteCommitResponse::SqliteErrorResponse(err) => {
			v3::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v2_to_v3(
				err,
			))
		}
	}
}

fn convert_sqlite_commit_response_v3_to_v2(
	data: v3::SqliteCommitResponse,
) -> v2::SqliteCommitResponse {
	match data {
		v3::SqliteCommitResponse::SqliteCommitOk(ok) => {
			v2::SqliteCommitResponse::SqliteCommitOk(v2::SqliteCommitOk {
				new_head_txid: ok.new_head_txid,
				meta: convert_sqlite_meta_v3_to_v2(ok.meta),
			})
		}
		v3::SqliteCommitResponse::SqliteFenceMismatch(mismatch) => {
			v2::SqliteCommitResponse::SqliteFenceMismatch(convert_sqlite_fence_mismatch_v3_to_v2(
				mismatch,
			))
		}
		v3::SqliteCommitResponse::SqliteCommitTooLarge(too_large) => {
			v2::SqliteCommitResponse::SqliteCommitTooLarge(v2::SqliteCommitTooLarge {
				actual_size_bytes: too_large.actual_size_bytes,
				max_size_bytes: too_large.max_size_bytes,
			})
		}
		v3::SqliteCommitResponse::SqliteErrorResponse(err) => {
			v2::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v3_to_v2(
				err,
			))
		}
	}
}

fn convert_sqlite_commit_stage_begin_response_v2_to_v3(
	data: v2::SqliteCommitStageBeginResponse,
) -> v3::SqliteCommitStageBeginResponse {
	match data {
		v2::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(ok) => {
			v3::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(
				v3::SqliteCommitStageBeginOk { txid: ok.txid },
			)
		}
		v2::SqliteCommitStageBeginResponse::SqliteFenceMismatch(mismatch) => {
			v3::SqliteCommitStageBeginResponse::SqliteFenceMismatch(
				convert_sqlite_fence_mismatch_v2_to_v3(mismatch),
			)
		}
		v2::SqliteCommitStageBeginResponse::SqliteErrorResponse(err) => {
			v3::SqliteCommitStageBeginResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v2_to_v3(err),
			)
		}
	}
}

fn convert_sqlite_commit_stage_begin_response_v3_to_v2(
	data: v3::SqliteCommitStageBeginResponse,
) -> v2::SqliteCommitStageBeginResponse {
	match data {
		v3::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(ok) => {
			v2::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(
				v2::SqliteCommitStageBeginOk { txid: ok.txid },
			)
		}
		v3::SqliteCommitStageBeginResponse::SqliteFenceMismatch(mismatch) => {
			v2::SqliteCommitStageBeginResponse::SqliteFenceMismatch(
				convert_sqlite_fence_mismatch_v3_to_v2(mismatch),
			)
		}
		v3::SqliteCommitStageBeginResponse::SqliteErrorResponse(err) => {
			v2::SqliteCommitStageBeginResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v3_to_v2(err),
			)
		}
	}
}

fn convert_sqlite_commit_stage_response_v2_to_v3(
	data: v2::SqliteCommitStageResponse,
) -> v3::SqliteCommitStageResponse {
	match data {
		v2::SqliteCommitStageResponse::SqliteCommitStageOk(ok) => {
			v3::SqliteCommitStageResponse::SqliteCommitStageOk(v3::SqliteCommitStageOk {
				chunk_idx_committed: ok.chunk_idx_committed,
			})
		}
		v2::SqliteCommitStageResponse::SqliteFenceMismatch(mismatch) => {
			v3::SqliteCommitStageResponse::SqliteFenceMismatch(
				convert_sqlite_fence_mismatch_v2_to_v3(mismatch),
			)
		}
		v2::SqliteCommitStageResponse::SqliteErrorResponse(err) => {
			v3::SqliteCommitStageResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v2_to_v3(err),
			)
		}
	}
}

fn convert_sqlite_commit_stage_response_v3_to_v2(
	data: v3::SqliteCommitStageResponse,
) -> v2::SqliteCommitStageResponse {
	match data {
		v3::SqliteCommitStageResponse::SqliteCommitStageOk(ok) => {
			v2::SqliteCommitStageResponse::SqliteCommitStageOk(v2::SqliteCommitStageOk {
				chunk_idx_committed: ok.chunk_idx_committed,
			})
		}
		v3::SqliteCommitStageResponse::SqliteFenceMismatch(mismatch) => {
			v2::SqliteCommitStageResponse::SqliteFenceMismatch(
				convert_sqlite_fence_mismatch_v3_to_v2(mismatch),
			)
		}
		v3::SqliteCommitStageResponse::SqliteErrorResponse(err) => {
			v2::SqliteCommitStageResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v3_to_v2(err),
			)
		}
	}
}

fn convert_sqlite_commit_finalize_response_v2_to_v3(
	data: v2::SqliteCommitFinalizeResponse,
) -> v3::SqliteCommitFinalizeResponse {
	match data {
		v2::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(ok) => {
			v3::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(v3::SqliteCommitFinalizeOk {
				new_head_txid: ok.new_head_txid,
				meta: convert_sqlite_meta_v2_to_v3(ok.meta),
			})
		}
		v2::SqliteCommitFinalizeResponse::SqliteFenceMismatch(mismatch) => {
			v3::SqliteCommitFinalizeResponse::SqliteFenceMismatch(
				convert_sqlite_fence_mismatch_v2_to_v3(mismatch),
			)
		}
		v2::SqliteCommitFinalizeResponse::SqliteStageNotFound(not_found) => {
			v3::SqliteCommitFinalizeResponse::SqliteStageNotFound(v3::SqliteStageNotFound {
				stage_id: not_found.stage_id,
			})
		}
		v2::SqliteCommitFinalizeResponse::SqliteErrorResponse(err) => {
			v3::SqliteCommitFinalizeResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v2_to_v3(err),
			)
		}
	}
}

fn convert_sqlite_commit_finalize_response_v3_to_v2(
	data: v3::SqliteCommitFinalizeResponse,
) -> v2::SqliteCommitFinalizeResponse {
	match data {
		v3::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(ok) => {
			v2::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(v2::SqliteCommitFinalizeOk {
				new_head_txid: ok.new_head_txid,
				meta: convert_sqlite_meta_v3_to_v2(ok.meta),
			})
		}
		v3::SqliteCommitFinalizeResponse::SqliteFenceMismatch(mismatch) => {
			v2::SqliteCommitFinalizeResponse::SqliteFenceMismatch(
				convert_sqlite_fence_mismatch_v3_to_v2(mismatch),
			)
		}
		v3::SqliteCommitFinalizeResponse::SqliteStageNotFound(not_found) => {
			v2::SqliteCommitFinalizeResponse::SqliteStageNotFound(v2::SqliteStageNotFound {
				stage_id: not_found.stage_id,
			})
		}
		v3::SqliteCommitFinalizeResponse::SqliteErrorResponse(err) => {
			v2::SqliteCommitFinalizeResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v3_to_v2(err),
			)
		}
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use vbare::OwnedVersionedData;

	use super::{ActorCommandKeyData, ToEnvoy};
	use crate::generated::{v1, v3};

	#[test]
	fn v1_start_command_deserializes_into_v3_with_empty_sqlite_startup_data() -> Result<()> {
		let payload =
			serde_bare::to_vec(&v1::ToEnvoy::ToEnvoyCommands(vec![v1::CommandWrapper {
				checkpoint: v1::ActorCheckpoint {
					actor_id: "actor".into(),
					generation: 7,
					index: 3,
				},
				inner: v1::Command::CommandStartActor(v1::CommandStartActor {
					config: v1::ActorConfig {
						name: "demo".into(),
						key: Some("key".into()),
						create_ts: 42,
						input: None,
					},
					hibernating_requests: Vec::new(),
					preloaded_kv: None,
				}),
			}]))?;

		let decoded = ToEnvoy::deserialize_version(&payload, 1)?.unwrap_latest()?;
		let v3::ToEnvoy::ToEnvoyCommands(commands) = decoded else {
			panic!("expected commands");
		};
		let v3::Command::CommandStartActor(start) = &commands[0].inner else {
			panic!("expected start actor");
		};

		assert!(start.sqlite_startup_data.is_none());
		assert!(start.preloaded_kv.is_none());
		assert_eq!(commands[0].checkpoint.generation, 7);

		Ok(())
	}

	#[test]
	fn sqlite_startup_data_is_dropped_when_serializing_start_command_to_v1() -> Result<()> {
		let encoded =
			ToEnvoy::wrap_latest(v3::ToEnvoy::ToEnvoyCommands(vec![v3::CommandWrapper {
				checkpoint: v3::ActorCheckpoint {
					actor_id: "actor".into(),
					generation: 1,
					index: 0,
				},
				inner: v3::Command::CommandStartActor(v3::CommandStartActor {
					config: v3::ActorConfig {
						name: "demo".into(),
						key: None,
						create_ts: 1,
						input: None,
					},
					hibernating_requests: Vec::new(),
					preloaded_kv: None,
					sqlite_startup_data: Some(v3::SqliteStartupData {
						generation: 11,
						meta: v3::SqliteMeta {
							generation: 11,
							head_txid: 5,
							materialized_txid: 5,
							db_size_pages: 1,
							page_size: 4096,
							creation_ts_ms: 99,
							max_delta_bytes: 8 * 1024 * 1024,
						},
						preloaded_pages: Vec::new(),
					}),
				}),
			}]))
			.serialize_version(1)?;

		let decoded = ToEnvoy::deserialize_version(&encoded, 1)?.unwrap_latest()?;
		let v3::ToEnvoy::ToEnvoyCommands(commands) = decoded else {
			panic!("expected commands");
		};
		let v3::Command::CommandStartActor(start) = &commands[0].inner else {
			panic!("expected start actor");
		};

		assert!(start.sqlite_startup_data.is_none());

		Ok(())
	}

	#[test]
	fn actor_command_key_data_round_trips_to_v1_when_sqlite_startup_data_is_absent() -> Result<()> {
		let encoded = ActorCommandKeyData::wrap_latest(v3::ActorCommandKeyData::CommandStartActor(
			v3::CommandStartActor {
				config: v3::ActorConfig {
					name: "demo".into(),
					key: None,
					create_ts: 7,
					input: None,
				},
				hibernating_requests: Vec::new(),
				preloaded_kv: None,
				sqlite_startup_data: None,
			},
		))
		.serialize_version(1)?;

		let decoded = ActorCommandKeyData::deserialize_version(&encoded, 1)?.unwrap_latest()?;
		let v3::ActorCommandKeyData::CommandStartActor(start) = decoded else {
			panic!("expected start actor");
		};
		assert!(start.sqlite_startup_data.is_none());

		Ok(())
	}
}
