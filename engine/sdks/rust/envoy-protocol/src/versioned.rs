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
		Ok(Self::V3(match version {
			1 => convert_to_envoy_v2_to_v3(convert_to_envoy_v1_to_v2(
				serde_bare::from_slice(payload)?,
			)?)?,
			2 => convert_to_envoy_v2_to_v3(serde_bare::from_slice(payload)?)?,
			3 => serde_bare::from_slice(payload)?,
			_ => bail!("invalid version: {version}"),
		}))
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		let Self::V3(data) = self;
		match version {
			1 => serde_bare::to_vec(&convert_to_envoy_v2_to_v1(convert_to_envoy_v3_to_v2(data)?)?)
				.map_err(Into::into),
			2 => serde_bare::to_vec(&convert_to_envoy_v3_to_v2(data)?).map_err(Into::into),
			3 => serde_bare::to_vec(&data).map_err(Into::into),
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
		Ok(Self::V3(match version {
			1 | 2 => convert_to_rivet_v2_to_v3(serde_bare::from_slice(payload)?)?,
			3 => serde_bare::from_slice(payload)?,
			_ => bail!("invalid version: {version}"),
		}))
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		let Self::V3(data) = self;
		match version {
			1 | 2 => serde_bare::to_vec(&convert_to_rivet_v3_to_v2(data)?).map_err(Into::into),
			3 => serde_bare::to_vec(&data).map_err(Into::into),
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
		Ok(Self::V3(match version {
			1 => convert_to_envoy_conn_v1_to_v3(serde_bare::from_slice(payload)?)?,
			2 => convert_to_envoy_conn_v2_to_v3(serde_bare::from_slice(payload)?)?,
			3 => serde_bare::from_slice(payload)?,
			_ => bail!("invalid version: {version}"),
		}))
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		let Self::V3(data) = self;
		match version {
			1 => {
				serde_bare::to_vec(&convert_to_envoy_conn_v3_to_v1(data)?).map_err(Into::into)
			}
			2 => {
				serde_bare::to_vec(&convert_to_envoy_conn_v3_to_v2(data)?).map_err(Into::into)
			}
			3 => serde_bare::to_vec(&data).map_err(Into::into),
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
		Ok(Self::V3(match version {
			1 => convert_to_gateway_v1_to_v3(serde_bare::from_slice(payload)?),
			2 => convert_to_gateway_v2_to_v3(serde_bare::from_slice(payload)?),
			3 => serde_bare::from_slice(payload)?,
			_ => bail!("invalid version: {version}"),
		}))
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		let Self::V3(data) = self;
		match version {
			1 => serde_bare::to_vec(&convert_to_gateway_v3_to_v1(data)).map_err(Into::into),
			2 => serde_bare::to_vec(&convert_to_gateway_v3_to_v2(data)).map_err(Into::into),
			3 => serde_bare::to_vec(&data).map_err(Into::into),
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
		Ok(Self::V3(match version {
			1 => convert_to_outbound_v1_to_v3(serde_bare::from_slice(payload)?),
			2 => convert_to_outbound_v2_to_v3(serde_bare::from_slice(payload)?),
			3 => serde_bare::from_slice(payload)?,
			_ => bail!("invalid version: {version}"),
		}))
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		let Self::V3(data) = self;
		match version {
			1 => serde_bare::to_vec(&convert_to_outbound_v3_to_v1(data)).map_err(Into::into),
			2 => serde_bare::to_vec(&convert_to_outbound_v3_to_v2(data)).map_err(Into::into),
			3 => serde_bare::to_vec(&data).map_err(Into::into),
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
		Ok(Self::V3(match version {
			1 => convert_actor_command_key_data_v1_to_v3(serde_bare::from_slice(payload)?),
			2 => convert_actor_command_key_data_v2_to_v3(serde_bare::from_slice(payload)?),
			3 => serde_bare::from_slice(payload)?,
			_ => bail!("invalid version: {version}"),
		}))
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		let Self::V3(data) = self;
		match version {
			1 => {
				serde_bare::to_vec(&convert_actor_command_key_data_v3_to_v1(data))
					.map_err(Into::into)
			}
			2 => {
				serde_bare::to_vec(&convert_actor_command_key_data_v3_to_v2(data))
					.map_err(Into::into)
			}
			3 => serde_bare::to_vec(&data).map_err(Into::into),
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

fn convert_to_envoy_v2_to_v3(message: v2::ToEnvoy) -> Result<v3::ToEnvoy> {
	Ok(match message {
		v2::ToEnvoy::ToEnvoyInit(init) => v3::ToEnvoy::ToEnvoyInit(v3::ToEnvoyInit {
			metadata: convert_protocol_metadata_v2_to_v3(init.metadata),
		}),
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
			v3::ToEnvoy::ToEnvoyPing(v3::ToEnvoyPing { ts: ping.ts })
		}
		v2::ToEnvoy::ToEnvoySqliteGetPagesResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitStageResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(_) => {
			bail!("legacy sqlite responses require envoy-protocol v2")
		}
	})
}

fn convert_to_envoy_v3_to_v2(message: v3::ToEnvoy) -> Result<v2::ToEnvoy> {
	Ok(match message {
		v3::ToEnvoy::ToEnvoyInit(init) => v2::ToEnvoy::ToEnvoyInit(v2::ToEnvoyInit {
			metadata: convert_protocol_metadata_v3_to_v2(init.metadata),
		}),
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
			v2::ToEnvoy::ToEnvoyPing(v2::ToEnvoyPing { ts: ping.ts })
		}
		v3::ToEnvoy::ToEnvoySqliteGetPagesResponse(_)
		| v3::ToEnvoy::ToEnvoySqliteCommitResponse(_) => {
			bail!("stateless sqlite responses require envoy-protocol v3")
		}
	})
}

fn convert_to_rivet_v2_to_v3(message: v2::ToRivet) -> Result<v3::ToRivet> {
	Ok(match message {
		v2::ToRivet::ToRivetMetadata(metadata) => {
			v3::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v2_to_v3(metadata))
		}
		v2::ToRivet::ToRivetEvents(events) => v3::ToRivet::ToRivetEvents(
			events.into_iter().map(convert_event_wrapper_v2_to_v3).collect(),
		),
		v2::ToRivet::ToRivetAckCommands(ack) => {
			v3::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v2_to_v3(ack))
		}
		v2::ToRivet::ToRivetStopping => v3::ToRivet::ToRivetStopping,
		v2::ToRivet::ToRivetPong(pong) => v3::ToRivet::ToRivetPong(v3::ToRivetPong { ts: pong.ts }),
		v2::ToRivet::ToRivetKvRequest(request) => {
			v3::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v2_to_v3(request))
		}
		v2::ToRivet::ToRivetTunnelMessage(message) => {
			v3::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v2_to_v3(message))
		}
		v2::ToRivet::ToRivetSqliteGetPagesRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitStageBeginRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitStageRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitFinalizeRequest(_) => {
			bail!("legacy sqlite requests require envoy-protocol v2")
		}
	})
}

fn convert_to_rivet_v3_to_v2(message: v3::ToRivet) -> Result<v2::ToRivet> {
	Ok(match message {
		v3::ToRivet::ToRivetMetadata(metadata) => {
			v2::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v3_to_v2(metadata))
		}
		v3::ToRivet::ToRivetEvents(events) => v2::ToRivet::ToRivetEvents(
			events.into_iter().map(convert_event_wrapper_v3_to_v2).collect(),
		),
		v3::ToRivet::ToRivetAckCommands(ack) => {
			v2::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v3_to_v2(ack))
		}
		v3::ToRivet::ToRivetStopping => v2::ToRivet::ToRivetStopping,
		v3::ToRivet::ToRivetPong(pong) => v2::ToRivet::ToRivetPong(v2::ToRivetPong { ts: pong.ts }),
		v3::ToRivet::ToRivetKvRequest(request) => {
			v2::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v3_to_v2(request))
		}
		v3::ToRivet::ToRivetTunnelMessage(message) => {
			v2::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v3_to_v2(message))
		}
		v3::ToRivet::ToRivetSqliteGetPagesRequest(_)
		| v3::ToRivet::ToRivetSqliteCommitRequest(_) => {
			bail!("stateless sqlite requests require envoy-protocol v3")
		}
	})
}

fn convert_to_envoy_conn_v1_to_v3(message: v1::ToEnvoyConn) -> Result<v3::ToEnvoyConn> {
	Ok(match message {
		v1::ToEnvoyConn::ToEnvoyConnPing(ping) => {
			v3::ToEnvoyConn::ToEnvoyConnPing(v3::ToEnvoyConnPing {
				gateway_id: ping.gateway_id,
				request_id: ping.request_id,
				ts: ping.ts,
			})
		}
		v1::ToEnvoyConn::ToEnvoyConnClose => v3::ToEnvoyConn::ToEnvoyConnClose,
		v1::ToEnvoyConn::ToEnvoyCommands(commands) => v3::ToEnvoyConn::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v1_to_v3)
				.collect(),
		),
		v1::ToEnvoyConn::ToEnvoyAckEvents(ack) => {
			v3::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v1_to_v3(ack))
		}
		v1::ToEnvoyConn::ToEnvoyTunnelMessage(message) => {
			v3::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v1_to_v3(message))
		}
	})
}

fn convert_to_envoy_conn_v2_to_v3(message: v2::ToEnvoyConn) -> Result<v3::ToEnvoyConn> {
	Ok(match message {
		v2::ToEnvoyConn::ToEnvoyConnPing(ping) => {
			v3::ToEnvoyConn::ToEnvoyConnPing(v3::ToEnvoyConnPing {
				gateway_id: ping.gateway_id,
				request_id: ping.request_id,
				ts: ping.ts,
			})
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

fn convert_to_envoy_conn_v3_to_v1(message: v3::ToEnvoyConn) -> Result<v1::ToEnvoyConn> {
	Ok(match message {
		v3::ToEnvoyConn::ToEnvoyConnPing(ping) => {
			v1::ToEnvoyConn::ToEnvoyConnPing(v1::ToEnvoyConnPing {
				gateway_id: ping.gateway_id,
				request_id: ping.request_id,
				ts: ping.ts,
			})
		}
		v3::ToEnvoyConn::ToEnvoyConnClose => v1::ToEnvoyConn::ToEnvoyConnClose,
		v3::ToEnvoyConn::ToEnvoyCommands(commands) => v1::ToEnvoyConn::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v3_to_v1)
				.collect(),
		),
		v3::ToEnvoyConn::ToEnvoyAckEvents(ack) => {
			v1::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v3_to_v1(ack))
		}
		v3::ToEnvoyConn::ToEnvoyTunnelMessage(message) => {
			v1::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v3_to_v1(message))
		}
	})
}

fn convert_to_envoy_conn_v3_to_v2(message: v3::ToEnvoyConn) -> Result<v2::ToEnvoyConn> {
	Ok(match message {
		v3::ToEnvoyConn::ToEnvoyConnPing(ping) => {
			v2::ToEnvoyConn::ToEnvoyConnPing(v2::ToEnvoyConnPing {
				gateway_id: ping.gateway_id,
				request_id: ping.request_id,
				ts: ping.ts,
			})
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
	})
}

fn convert_to_gateway_v1_to_v3(message: v1::ToGateway) -> v3::ToGateway {
	match message {
		v1::ToGateway::ToGatewayPong(pong) => v3::ToGateway::ToGatewayPong(v3::ToGatewayPong {
			request_id: pong.request_id,
			ts: pong.ts,
		}),
		v1::ToGateway::ToRivetTunnelMessage(message) => {
			v3::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v1_to_v3(message))
		}
	}
}

fn convert_to_gateway_v2_to_v3(message: v2::ToGateway) -> v3::ToGateway {
	match message {
		v2::ToGateway::ToGatewayPong(pong) => v3::ToGateway::ToGatewayPong(v3::ToGatewayPong {
			request_id: pong.request_id,
			ts: pong.ts,
		}),
		v2::ToGateway::ToRivetTunnelMessage(message) => {
			v3::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v2_to_v3(message))
		}
	}
}

fn convert_to_gateway_v3_to_v1(message: v3::ToGateway) -> v1::ToGateway {
	match message {
		v3::ToGateway::ToGatewayPong(pong) => v1::ToGateway::ToGatewayPong(v1::ToGatewayPong {
			request_id: pong.request_id,
			ts: pong.ts,
		}),
		v3::ToGateway::ToRivetTunnelMessage(message) => {
			v1::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v3_to_v1(message))
		}
	}
}

fn convert_to_gateway_v3_to_v2(message: v3::ToGateway) -> v2::ToGateway {
	match message {
		v3::ToGateway::ToGatewayPong(pong) => v2::ToGateway::ToGatewayPong(v2::ToGatewayPong {
			request_id: pong.request_id,
			ts: pong.ts,
		}),
		v3::ToGateway::ToRivetTunnelMessage(message) => {
			v2::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v3_to_v2(message))
		}
	}
}

fn convert_to_outbound_v1_to_v3(message: v1::ToOutbound) -> v3::ToOutbound {
	match message {
		v1::ToOutbound::ToOutboundActorStart(start) => {
			v3::ToOutbound::ToOutboundActorStart(v3::ToOutboundActorStart {
				namespace_id: start.namespace_id,
				pool_name: start.pool_name,
				checkpoint: convert_actor_checkpoint_v1_to_v3(start.checkpoint),
				actor_config: convert_actor_config_v1_to_v3(start.actor_config),
			})
		}
	}
}

fn convert_to_outbound_v2_to_v3(message: v2::ToOutbound) -> v3::ToOutbound {
	match message {
		v2::ToOutbound::ToOutboundActorStart(start) => {
			v3::ToOutbound::ToOutboundActorStart(v3::ToOutboundActorStart {
				namespace_id: start.namespace_id,
				pool_name: start.pool_name,
				checkpoint: convert_actor_checkpoint_v2_to_v3(start.checkpoint),
				actor_config: convert_actor_config_v2_to_v3(start.actor_config),
			})
		}
	}
}

fn convert_to_outbound_v3_to_v1(message: v3::ToOutbound) -> v1::ToOutbound {
	match message {
		v3::ToOutbound::ToOutboundActorStart(start) => {
			v1::ToOutbound::ToOutboundActorStart(v1::ToOutboundActorStart {
				namespace_id: start.namespace_id,
				pool_name: start.pool_name,
				checkpoint: convert_actor_checkpoint_v3_to_v1(start.checkpoint),
				actor_config: convert_actor_config_v3_to_v1(start.actor_config),
			})
		}
	}
}

fn convert_to_outbound_v3_to_v2(message: v3::ToOutbound) -> v2::ToOutbound {
	match message {
		v3::ToOutbound::ToOutboundActorStart(start) => {
			v2::ToOutbound::ToOutboundActorStart(v2::ToOutboundActorStart {
				namespace_id: start.namespace_id,
				pool_name: start.pool_name,
				checkpoint: convert_actor_checkpoint_v3_to_v2(start.checkpoint),
				actor_config: convert_actor_config_v3_to_v2(start.actor_config),
			})
		}
	}
}

fn convert_to_envoy_v1_to_v2(message: v1::ToEnvoy) -> Result<v2::ToEnvoy> {
	Ok(match message {
		v1::ToEnvoy::ToEnvoyCommands(commands) => v2::ToEnvoy::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v1_to_v2)
				.collect::<Result<Vec<_>>>()?,
		),
		v1::ToEnvoy::ToEnvoyInit(init) => v2::ToEnvoy::ToEnvoyInit(v2::ToEnvoyInit {
			metadata: convert_protocol_metadata_v1_to_v2(init.metadata),
		}),
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
			v2::ToEnvoy::ToEnvoyPing(v2::ToEnvoyPing { ts: ping.ts })
		}
	})
}

fn convert_to_envoy_v2_to_v1(message: v2::ToEnvoy) -> Result<v1::ToEnvoy> {
	Ok(match message {
		v2::ToEnvoy::ToEnvoyCommands(commands) => v1::ToEnvoy::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v2_to_v1)
				.collect::<Result<Vec<_>>>()?,
		),
		v2::ToEnvoy::ToEnvoyInit(init) => v1::ToEnvoy::ToEnvoyInit(v1::ToEnvoyInit {
			metadata: convert_protocol_metadata_v2_to_v1(init.metadata),
		}),
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
			v1::ToEnvoy::ToEnvoyPing(v1::ToEnvoyPing { ts: ping.ts })
		}
		v2::ToEnvoy::ToEnvoySqliteGetPagesResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitStageResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(_) => {
			bail!("sqlite responses require envoy-protocol v2")
		}
	})
}

fn convert_command_wrapper_v1_to_v2(wrapper: v1::CommandWrapper) -> Result<v2::CommandWrapper> {
	Ok(v2::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v1_to_v2(wrapper.checkpoint),
		inner: convert_command_v1_to_v2(wrapper.inner)?,
	})
}

fn convert_command_wrapper_v2_to_v1(wrapper: v2::CommandWrapper) -> Result<v1::CommandWrapper> {
	Ok(v1::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v2_to_v1(wrapper.checkpoint),
		inner: convert_command_v2_to_v1(wrapper.inner)?,
	})
}

fn convert_command_wrapper_v1_to_v3(wrapper: v1::CommandWrapper) -> v3::CommandWrapper {
	v3::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v1_to_v3(wrapper.checkpoint),
		inner: convert_command_v1_to_v3(wrapper.inner),
	}
}

fn convert_command_wrapper_v2_to_v3(wrapper: v2::CommandWrapper) -> v3::CommandWrapper {
	v3::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v2_to_v3(wrapper.checkpoint),
		inner: convert_command_v2_to_v3(wrapper.inner),
	}
}

fn convert_command_wrapper_v3_to_v1(wrapper: v3::CommandWrapper) -> v1::CommandWrapper {
	v1::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v3_to_v1(wrapper.checkpoint),
		inner: convert_command_v3_to_v1(wrapper.inner),
	}
}

fn convert_command_wrapper_v3_to_v2(wrapper: v3::CommandWrapper) -> v2::CommandWrapper {
	v2::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v3_to_v2(wrapper.checkpoint),
		inner: convert_command_v3_to_v2(wrapper.inner),
	}
}

fn convert_command_v1_to_v2(command: v1::Command) -> Result<v2::Command> {
	Ok(match command {
		v1::Command::CommandStartActor(start) => {
			v2::Command::CommandStartActor(convert_command_start_actor_v1_to_v2(start))
		}
		v1::Command::CommandStopActor(stop) => {
			v2::Command::CommandStopActor(v2::CommandStopActor {
				reason: convert_stop_actor_reason_v1_to_v2(stop.reason),
			})
		}
	})
}

fn convert_command_v2_to_v1(command: v2::Command) -> Result<v1::Command> {
	Ok(match command {
		v2::Command::CommandStartActor(start) => {
			v1::Command::CommandStartActor(convert_command_start_actor_v2_to_v1(start)?)
		}
		v2::Command::CommandStopActor(stop) => {
			v1::Command::CommandStopActor(v1::CommandStopActor {
				reason: convert_stop_actor_reason_v2_to_v1(stop.reason),
			})
		}
	})
}

fn convert_command_v1_to_v3(command: v1::Command) -> v3::Command {
	match command {
		v1::Command::CommandStartActor(start) => {
			v3::Command::CommandStartActor(convert_command_start_actor_v1_to_v3(start))
		}
		v1::Command::CommandStopActor(stop) => {
			v3::Command::CommandStopActor(v3::CommandStopActor {
				reason: convert_stop_actor_reason_v1_to_v3(stop.reason),
			})
		}
	}
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

fn convert_command_v3_to_v1(command: v3::Command) -> v1::Command {
	match command {
		v3::Command::CommandStartActor(start) => {
			v1::Command::CommandStartActor(convert_command_start_actor_v3_to_v1(start))
		}
		v3::Command::CommandStopActor(stop) => {
			v1::Command::CommandStopActor(v1::CommandStopActor {
				reason: convert_stop_actor_reason_v3_to_v1(stop.reason),
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

fn convert_command_start_actor_v2_to_v1(
	start: v2::CommandStartActor,
) -> Result<v1::CommandStartActor> {
	if start.sqlite_startup_data.is_some() {
		bail!("sqlite startup data requires envoy-protocol v2");
	}

	Ok(v1::CommandStartActor {
		config: convert_actor_config_v2_to_v1(start.config),
		hibernating_requests: start
			.hibernating_requests
			.into_iter()
			.map(convert_hibernating_request_v2_to_v1)
			.collect(),
		preloaded_kv: start.preloaded_kv.map(convert_preloaded_kv_v2_to_v1),
	})
}

fn convert_command_start_actor_v1_to_v3(start: v1::CommandStartActor) -> v3::CommandStartActor {
	v3::CommandStartActor {
		config: convert_actor_config_v1_to_v3(start.config),
		hibernating_requests: start
			.hibernating_requests
			.into_iter()
			.map(convert_hibernating_request_v1_to_v3)
			.collect(),
		preloaded_kv: start.preloaded_kv.map(convert_preloaded_kv_v1_to_v3),
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
	}
}

fn convert_command_start_actor_v3_to_v1(start: v3::CommandStartActor) -> v1::CommandStartActor {
	v1::CommandStartActor {
		config: convert_actor_config_v3_to_v1(start.config),
		hibernating_requests: start
			.hibernating_requests
			.into_iter()
			.map(convert_hibernating_request_v3_to_v1)
			.collect(),
		preloaded_kv: start.preloaded_kv.map(convert_preloaded_kv_v3_to_v1),
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
		sqlite_startup_data: None,
	}
}

fn convert_actor_command_key_data_v1_to_v3(data: v1::ActorCommandKeyData) -> v3::ActorCommandKeyData {
	match data {
		v1::ActorCommandKeyData::CommandStartActor(start) => {
			v3::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v1_to_v3(start))
		}
		v1::ActorCommandKeyData::CommandStopActor(stop) => {
			v3::ActorCommandKeyData::CommandStopActor(v3::CommandStopActor {
				reason: convert_stop_actor_reason_v1_to_v3(stop.reason),
			})
		}
	}
}

fn convert_actor_command_key_data_v2_to_v3(data: v2::ActorCommandKeyData) -> v3::ActorCommandKeyData {
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

fn convert_actor_command_key_data_v3_to_v1(data: v3::ActorCommandKeyData) -> v1::ActorCommandKeyData {
	match data {
		v3::ActorCommandKeyData::CommandStartActor(start) => {
			v1::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v3_to_v1(start))
		}
		v3::ActorCommandKeyData::CommandStopActor(stop) => {
			v1::ActorCommandKeyData::CommandStopActor(v1::CommandStopActor {
				reason: convert_stop_actor_reason_v3_to_v1(stop.reason),
			})
		}
	}
}

fn convert_actor_command_key_data_v3_to_v2(data: v3::ActorCommandKeyData) -> v2::ActorCommandKeyData {
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

fn convert_protocol_metadata_v1_to_v2(value: v1::ProtocolMetadata) -> v2::ProtocolMetadata {
	v2::ProtocolMetadata {
		envoy_lost_threshold: value.envoy_lost_threshold,
		actor_stop_threshold: value.actor_stop_threshold,
		max_response_payload_size: value.max_response_payload_size,
	}
}

fn convert_protocol_metadata_v2_to_v1(value: v2::ProtocolMetadata) -> v1::ProtocolMetadata {
	v1::ProtocolMetadata {
		envoy_lost_threshold: value.envoy_lost_threshold,
		actor_stop_threshold: value.actor_stop_threshold,
		max_response_payload_size: value.max_response_payload_size,
	}
}

fn convert_protocol_metadata_v2_to_v3(value: v2::ProtocolMetadata) -> v3::ProtocolMetadata {
	v3::ProtocolMetadata {
		envoy_lost_threshold: value.envoy_lost_threshold,
		actor_stop_threshold: value.actor_stop_threshold,
		max_response_payload_size: value.max_response_payload_size,
	}
}

fn convert_protocol_metadata_v3_to_v2(value: v3::ProtocolMetadata) -> v2::ProtocolMetadata {
	v2::ProtocolMetadata {
		envoy_lost_threshold: value.envoy_lost_threshold,
		actor_stop_threshold: value.actor_stop_threshold,
		max_response_payload_size: value.max_response_payload_size,
	}
}

fn convert_actor_config_v1_to_v2(value: v1::ActorConfig) -> v2::ActorConfig {
	v2::ActorConfig { name: value.name, key: value.key, create_ts: value.create_ts, input: value.input }
}
fn convert_actor_config_v2_to_v1(value: v2::ActorConfig) -> v1::ActorConfig {
	v1::ActorConfig { name: value.name, key: value.key, create_ts: value.create_ts, input: value.input }
}
fn convert_actor_config_v1_to_v3(value: v1::ActorConfig) -> v3::ActorConfig {
	v3::ActorConfig { name: value.name, key: value.key, create_ts: value.create_ts, input: value.input }
}
fn convert_actor_config_v2_to_v3(value: v2::ActorConfig) -> v3::ActorConfig {
	v3::ActorConfig { name: value.name, key: value.key, create_ts: value.create_ts, input: value.input }
}
fn convert_actor_config_v3_to_v1(value: v3::ActorConfig) -> v1::ActorConfig {
	v1::ActorConfig { name: value.name, key: value.key, create_ts: value.create_ts, input: value.input }
}
fn convert_actor_config_v3_to_v2(value: v3::ActorConfig) -> v2::ActorConfig {
	v2::ActorConfig { name: value.name, key: value.key, create_ts: value.create_ts, input: value.input }
}

fn convert_actor_checkpoint_v1_to_v2(value: v1::ActorCheckpoint) -> v2::ActorCheckpoint {
	v2::ActorCheckpoint { actor_id: value.actor_id, generation: value.generation, index: value.index }
}
fn convert_actor_checkpoint_v2_to_v1(value: v2::ActorCheckpoint) -> v1::ActorCheckpoint {
	v1::ActorCheckpoint { actor_id: value.actor_id, generation: value.generation, index: value.index }
}
fn convert_actor_checkpoint_v1_to_v3(value: v1::ActorCheckpoint) -> v3::ActorCheckpoint {
	v3::ActorCheckpoint { actor_id: value.actor_id, generation: value.generation, index: value.index }
}
fn convert_actor_checkpoint_v2_to_v3(value: v2::ActorCheckpoint) -> v3::ActorCheckpoint {
	v3::ActorCheckpoint { actor_id: value.actor_id, generation: value.generation, index: value.index }
}
fn convert_actor_checkpoint_v3_to_v1(value: v3::ActorCheckpoint) -> v1::ActorCheckpoint {
	v1::ActorCheckpoint { actor_id: value.actor_id, generation: value.generation, index: value.index }
}
fn convert_actor_checkpoint_v3_to_v2(value: v3::ActorCheckpoint) -> v2::ActorCheckpoint {
	v2::ActorCheckpoint { actor_id: value.actor_id, generation: value.generation, index: value.index }
}

fn convert_hibernating_request_v1_to_v2(value: v1::HibernatingRequest) -> v2::HibernatingRequest {
	v2::HibernatingRequest { gateway_id: value.gateway_id, request_id: value.request_id }
}
fn convert_hibernating_request_v2_to_v1(value: v2::HibernatingRequest) -> v1::HibernatingRequest {
	v1::HibernatingRequest { gateway_id: value.gateway_id, request_id: value.request_id }
}
fn convert_hibernating_request_v1_to_v3(value: v1::HibernatingRequest) -> v3::HibernatingRequest {
	v3::HibernatingRequest { gateway_id: value.gateway_id, request_id: value.request_id }
}
fn convert_hibernating_request_v2_to_v3(value: v2::HibernatingRequest) -> v3::HibernatingRequest {
	v3::HibernatingRequest { gateway_id: value.gateway_id, request_id: value.request_id }
}
fn convert_hibernating_request_v3_to_v1(value: v3::HibernatingRequest) -> v1::HibernatingRequest {
	v1::HibernatingRequest { gateway_id: value.gateway_id, request_id: value.request_id }
}
fn convert_hibernating_request_v3_to_v2(value: v3::HibernatingRequest) -> v2::HibernatingRequest {
	v2::HibernatingRequest { gateway_id: value.gateway_id, request_id: value.request_id }
}

fn convert_preloaded_kv_v1_to_v2(preloaded: v1::PreloadedKv) -> v2::PreloadedKv {
	v2::PreloadedKv {
		entries: preloaded.entries.into_iter().map(convert_preloaded_kv_entry_v1_to_v2).collect(),
		requested_get_keys: preloaded.requested_get_keys,
		requested_prefixes: preloaded.requested_prefixes,
	}
}
fn convert_preloaded_kv_v2_to_v1(preloaded: v2::PreloadedKv) -> v1::PreloadedKv {
	v1::PreloadedKv {
		entries: preloaded.entries.into_iter().map(convert_preloaded_kv_entry_v2_to_v1).collect(),
		requested_get_keys: preloaded.requested_get_keys,
		requested_prefixes: preloaded.requested_prefixes,
	}
}
fn convert_preloaded_kv_v1_to_v3(preloaded: v1::PreloadedKv) -> v3::PreloadedKv {
	v3::PreloadedKv {
		entries: preloaded.entries.into_iter().map(convert_preloaded_kv_entry_v1_to_v3).collect(),
		requested_get_keys: preloaded.requested_get_keys,
		requested_prefixes: preloaded.requested_prefixes,
	}
}
fn convert_preloaded_kv_v2_to_v3(preloaded: v2::PreloadedKv) -> v3::PreloadedKv {
	v3::PreloadedKv {
		entries: preloaded.entries.into_iter().map(convert_preloaded_kv_entry_v2_to_v3).collect(),
		requested_get_keys: preloaded.requested_get_keys,
		requested_prefixes: preloaded.requested_prefixes,
	}
}
fn convert_preloaded_kv_v3_to_v1(preloaded: v3::PreloadedKv) -> v1::PreloadedKv {
	v1::PreloadedKv {
		entries: preloaded.entries.into_iter().map(convert_preloaded_kv_entry_v3_to_v1).collect(),
		requested_get_keys: preloaded.requested_get_keys,
		requested_prefixes: preloaded.requested_prefixes,
	}
}
fn convert_preloaded_kv_v3_to_v2(preloaded: v3::PreloadedKv) -> v2::PreloadedKv {
	v2::PreloadedKv {
		entries: preloaded.entries.into_iter().map(convert_preloaded_kv_entry_v3_to_v2).collect(),
		requested_get_keys: preloaded.requested_get_keys,
		requested_prefixes: preloaded.requested_prefixes,
	}
}

fn convert_preloaded_kv_entry_v1_to_v2(entry: v1::PreloadedKvEntry) -> v2::PreloadedKvEntry {
	v2::PreloadedKvEntry { key: entry.key, value: entry.value, metadata: convert_kv_metadata_v1_to_v2(entry.metadata) }
}
fn convert_preloaded_kv_entry_v2_to_v1(entry: v2::PreloadedKvEntry) -> v1::PreloadedKvEntry {
	v1::PreloadedKvEntry { key: entry.key, value: entry.value, metadata: convert_kv_metadata_v2_to_v1(entry.metadata) }
}
fn convert_preloaded_kv_entry_v1_to_v3(entry: v1::PreloadedKvEntry) -> v3::PreloadedKvEntry {
	v3::PreloadedKvEntry { key: entry.key, value: entry.value, metadata: convert_kv_metadata_v1_to_v3(entry.metadata) }
}
fn convert_preloaded_kv_entry_v2_to_v3(entry: v2::PreloadedKvEntry) -> v3::PreloadedKvEntry {
	v3::PreloadedKvEntry { key: entry.key, value: entry.value, metadata: convert_kv_metadata_v2_to_v3(entry.metadata) }
}
fn convert_preloaded_kv_entry_v3_to_v1(entry: v3::PreloadedKvEntry) -> v1::PreloadedKvEntry {
	v1::PreloadedKvEntry { key: entry.key, value: entry.value, metadata: convert_kv_metadata_v3_to_v1(entry.metadata) }
}
fn convert_preloaded_kv_entry_v3_to_v2(entry: v3::PreloadedKvEntry) -> v2::PreloadedKvEntry {
	v2::PreloadedKvEntry { key: entry.key, value: entry.value, metadata: convert_kv_metadata_v3_to_v2(entry.metadata) }
}

fn convert_kv_metadata_v1_to_v2(value: v1::KvMetadata) -> v2::KvMetadata {
	v2::KvMetadata { version: value.version, update_ts: value.update_ts }
}
fn convert_kv_metadata_v2_to_v1(value: v2::KvMetadata) -> v1::KvMetadata {
	v1::KvMetadata { version: value.version, update_ts: value.update_ts }
}
fn convert_kv_metadata_v1_to_v3(value: v1::KvMetadata) -> v3::KvMetadata {
	v3::KvMetadata { version: value.version, update_ts: value.update_ts }
}
fn convert_kv_metadata_v2_to_v3(value: v2::KvMetadata) -> v3::KvMetadata {
	v3::KvMetadata { version: value.version, update_ts: value.update_ts }
}
fn convert_kv_metadata_v3_to_v1(value: v3::KvMetadata) -> v1::KvMetadata {
	v1::KvMetadata { version: value.version, update_ts: value.update_ts }
}
fn convert_kv_metadata_v3_to_v2(value: v3::KvMetadata) -> v2::KvMetadata {
	v2::KvMetadata { version: value.version, update_ts: value.update_ts }
}

include!("versioned_conversions.in");

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use vbare::OwnedVersionedData;

	use super::{ActorCommandKeyData, ToEnvoy};
	use crate::{
		PROTOCOL_VERSION,
		generated::{v1, v2, v3},
	};

	#[test]
	fn protocol_version_constant_matches_schema_version() {
		assert_eq!(PROTOCOL_VERSION, 3);
	}

	#[test]
	fn v1_start_command_deserializes_into_v3_without_sqlite_startup_data() -> Result<()> {
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

		assert!(start.preloaded_kv.is_none());
		assert_eq!(commands[0].checkpoint.generation, 7);

		Ok(())
	}

	#[test]
	fn v2_sqlite_response_does_not_deserialize_to_stateless_protocol() -> Result<()> {
		let payload = serde_bare::to_vec(&v2::ToEnvoy::ToEnvoySqliteCommitResponse(
			v2::ToEnvoySqliteCommitResponse {
				request_id: 1,
				data: v2::SqliteCommitResponse::SqliteErrorResponse(v2::SqliteErrorResponse {
					message: "old sqlite".into(),
				}),
			},
		))?;

		assert!(ToEnvoy::deserialize_version(&payload, 2).is_err());
		Ok(())
	}

	#[test]
	fn actor_command_key_data_round_trips_to_v1() -> Result<()> {
		let encoded = ActorCommandKeyData::wrap_latest(
			v3::ActorCommandKeyData::CommandStartActor(v3::CommandStartActor {
				config: v3::ActorConfig {
					name: "demo".into(),
					key: None,
					create_ts: 7,
					input: None,
				},
				hibernating_requests: Vec::new(),
				preloaded_kv: None,
			}),
		)
		.serialize_version(1)?;

		let decoded = ActorCommandKeyData::deserialize_version(&encoded, 1)?.unwrap_latest()?;
		let v3::ActorCommandKeyData::CommandStartActor(start) = decoded else {
			panic!("expected start actor");
		};
		assert_eq!(start.config.name, "demo");

		Ok(())
	}
}
