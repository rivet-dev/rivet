use anyhow::{Result, bail};
use serde::{Serialize, de::DeserializeOwned};
use vbare::OwnedVersionedData;

use crate::generated::{v1, v2};

fn reencode<T, U>(value: T) -> Result<U>
where
	T: Serialize,
	U: DeserializeOwned,
{
	let payload = serde_bare::to_vec(&value)?;
	serde_bare::from_slice(&payload).map_err(Into::into)
}

pub enum ToEnvoy {
	V1(v1::ToEnvoy),
	V2(v2::ToEnvoy),
}

impl OwnedVersionedData for ToEnvoy {
	type Latest = v2::ToEnvoy;

	fn wrap_latest(latest: v2::ToEnvoy) -> Self {
		ToEnvoy::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		if let ToEnvoy::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToEnvoy::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(ToEnvoy::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToEnvoy::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ToEnvoy::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}

impl ToEnvoy {
	fn v1_to_v2(self) -> Result<Self> {
		if let ToEnvoy::V1(message) = self {
			let inner = match message {
				v1::ToEnvoy::ToEnvoyInit(init) => v2::ToEnvoy::ToEnvoyInit(v2::ToEnvoyInit {
					metadata: convert_protocol_metadata_v1_to_v2(init.metadata),
				}),
				v1::ToEnvoy::ToEnvoyCommands(commands) => {
					v2::ToEnvoy::ToEnvoyCommands(reencode(commands)?)
				}
				v1::ToEnvoy::ToEnvoyAckEvents(ack) => v2::ToEnvoy::ToEnvoyAckEvents(reencode(ack)?),
				v1::ToEnvoy::ToEnvoyKvResponse(response) => {
					v2::ToEnvoy::ToEnvoyKvResponse(reencode(response)?)
				}
				v1::ToEnvoy::ToEnvoyTunnelMessage(message) => {
					v2::ToEnvoy::ToEnvoyTunnelMessage(reencode(message)?)
				}
				v1::ToEnvoy::ToEnvoyPing(ping) => v2::ToEnvoy::ToEnvoyPing(reencode(ping)?),
			};

			Ok(ToEnvoy::V2(inner))
		} else {
			bail!("unexpected version");
		}
	}

	fn v2_to_v1(self) -> Result<Self> {
		if let ToEnvoy::V2(message) = self {
			let inner = match message {
				v2::ToEnvoy::ToEnvoyInit(init) => v1::ToEnvoy::ToEnvoyInit(v1::ToEnvoyInit {
					metadata: convert_protocol_metadata_v2_to_v1(init.metadata),
				}),
				v2::ToEnvoy::ToEnvoyCommands(commands) => {
					v1::ToEnvoy::ToEnvoyCommands(reencode(commands)?)
				}
				v2::ToEnvoy::ToEnvoyAckEvents(ack) => v1::ToEnvoy::ToEnvoyAckEvents(reencode(ack)?),
				v2::ToEnvoy::ToEnvoyKvResponse(response) => {
					v1::ToEnvoy::ToEnvoyKvResponse(reencode(response)?)
				}
				v2::ToEnvoy::ToEnvoyTunnelMessage(message) => {
					v1::ToEnvoy::ToEnvoyTunnelMessage(reencode(message)?)
				}
				v2::ToEnvoy::ToEnvoyPing(ping) => v1::ToEnvoy::ToEnvoyPing(reencode(ping)?),
			};

			Ok(ToEnvoy::V1(inner))
		} else {
			bail!("unexpected version");
		}
	}
}

pub enum ToEnvoyConn {
	V1(v1::ToEnvoyConn),
	V2(v2::ToEnvoyConn),
}

impl OwnedVersionedData for ToEnvoyConn {
	type Latest = v2::ToEnvoyConn;

	fn wrap_latest(latest: v2::ToEnvoyConn) -> Self {
		ToEnvoyConn::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		if let ToEnvoyConn::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToEnvoyConn::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(ToEnvoyConn::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToEnvoyConn::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ToEnvoyConn::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}

impl ToEnvoyConn {
	fn v1_to_v2(self) -> Result<Self> {
		if let ToEnvoyConn::V1(message) = self {
			Ok(ToEnvoyConn::V2(reencode(message)?))
		} else {
			bail!("unexpected version");
		}
	}

	fn v2_to_v1(self) -> Result<Self> {
		if let ToEnvoyConn::V2(message) = self {
			Ok(ToEnvoyConn::V1(reencode(message)?))
		} else {
			bail!("unexpected version");
		}
	}
}

pub enum ToRivet {
	V1(v1::ToRivet),
	V2(v2::ToRivet),
}

impl OwnedVersionedData for ToRivet {
	type Latest = v2::ToRivet;

	fn wrap_latest(latest: v2::ToRivet) -> Self {
		ToRivet::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		if let ToRivet::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToRivet::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(ToRivet::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToRivet::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ToRivet::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}

impl ToRivet {
	fn v1_to_v2(self) -> Result<Self> {
		if let ToRivet::V1(message) = self {
			let inner = match message {
				v1::ToRivet::ToRivetMetadata(metadata) => {
					v2::ToRivet::ToRivetMetadata(reencode(metadata)?)
				}
				v1::ToRivet::ToRivetEvents(events) => v2::ToRivet::ToRivetEvents(reencode(events)?),
				v1::ToRivet::ToRivetAckCommands(ack) => {
					v2::ToRivet::ToRivetAckCommands(reencode(ack)?)
				}
				v1::ToRivet::ToRivetStopping => v2::ToRivet::ToRivetStopping,
				v1::ToRivet::ToRivetPong(pong) => v2::ToRivet::ToRivetPong(reencode(pong)?),
				v1::ToRivet::ToRivetKvRequest(request) => {
					v2::ToRivet::ToRivetKvRequest(v2::ToRivetKvRequest {
						actor_id: request.actor_id,
						request_id: request.request_id,
						data: convert_kv_request_data_v1_to_v2(request.data),
					})
				}
				v1::ToRivet::ToRivetTunnelMessage(message) => {
					v2::ToRivet::ToRivetTunnelMessage(reencode(message)?)
				}
			};

			Ok(ToRivet::V2(inner))
		} else {
			bail!("unexpected version");
		}
	}

	fn v2_to_v1(self) -> Result<Self> {
		if let ToRivet::V2(message) = self {
			let inner = match message {
				v2::ToRivet::ToRivetMetadata(metadata) => {
					v1::ToRivet::ToRivetMetadata(reencode(metadata)?)
				}
				v2::ToRivet::ToRivetEvents(events) => v1::ToRivet::ToRivetEvents(reencode(events)?),
				v2::ToRivet::ToRivetAckCommands(ack) => {
					v1::ToRivet::ToRivetAckCommands(reencode(ack)?)
				}
				v2::ToRivet::ToRivetStopping => v1::ToRivet::ToRivetStopping,
				v2::ToRivet::ToRivetPong(pong) => v1::ToRivet::ToRivetPong(reencode(pong)?),
				v2::ToRivet::ToRivetKvRequest(request) => {
					v1::ToRivet::ToRivetKvRequest(v1::ToRivetKvRequest {
						actor_id: request.actor_id,
						request_id: request.request_id,
						data: convert_kv_request_data_v2_to_v1(request.data)?,
					})
				}
				v2::ToRivet::ToRivetTunnelMessage(message) => {
					v1::ToRivet::ToRivetTunnelMessage(reencode(message)?)
				}
			};

			Ok(ToRivet::V1(inner))
		} else {
			bail!("unexpected version");
		}
	}
}

pub enum ToGateway {
	V1(v1::ToGateway),
	V2(v2::ToGateway),
}

impl OwnedVersionedData for ToGateway {
	type Latest = v2::ToGateway;

	fn wrap_latest(latest: v2::ToGateway) -> Self {
		ToGateway::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		if let ToGateway::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToGateway::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(ToGateway::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToGateway::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ToGateway::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}

impl ToGateway {
	fn v1_to_v2(self) -> Result<Self> {
		if let ToGateway::V1(message) = self {
			Ok(ToGateway::V2(reencode(message)?))
		} else {
			bail!("unexpected version");
		}
	}

	fn v2_to_v1(self) -> Result<Self> {
		if let ToGateway::V2(message) = self {
			Ok(ToGateway::V1(reencode(message)?))
		} else {
			bail!("unexpected version");
		}
	}
}

pub enum ToOutbound {
	V1(v1::ToOutbound),
	V2(v2::ToOutbound),
}

impl OwnedVersionedData for ToOutbound {
	type Latest = v2::ToOutbound;

	fn wrap_latest(latest: v2::ToOutbound) -> Self {
		ToOutbound::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		if let ToOutbound::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToOutbound::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(ToOutbound::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToOutbound::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ToOutbound::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}

impl ToOutbound {
	fn v1_to_v2(self) -> Result<Self> {
		if let ToOutbound::V1(message) = self {
			Ok(ToOutbound::V2(reencode(message)?))
		} else {
			bail!("unexpected version");
		}
	}

	fn v2_to_v1(self) -> Result<Self> {
		if let ToOutbound::V2(message) = self {
			Ok(ToOutbound::V1(reencode(message)?))
		} else {
			bail!("unexpected version");
		}
	}
}

pub enum ActorCommandKeyData {
	V1(v1::ActorCommandKeyData),
	V2(v2::ActorCommandKeyData),
}

impl OwnedVersionedData for ActorCommandKeyData {
	type Latest = v2::ActorCommandKeyData;

	fn wrap_latest(latest: v2::ActorCommandKeyData) -> Self {
		ActorCommandKeyData::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		if let ActorCommandKeyData::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ActorCommandKeyData::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(ActorCommandKeyData::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ActorCommandKeyData::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			ActorCommandKeyData::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}

impl ActorCommandKeyData {
	fn v1_to_v2(self) -> Result<Self> {
		if let ActorCommandKeyData::V1(data) = self {
			Ok(ActorCommandKeyData::V2(reencode(data)?))
		} else {
			bail!("unexpected version");
		}
	}

	fn v2_to_v1(self) -> Result<Self> {
		if let ActorCommandKeyData::V2(data) = self {
			Ok(ActorCommandKeyData::V1(reencode(data)?))
		} else {
			bail!("unexpected version");
		}
	}
}

fn convert_protocol_metadata_v1_to_v2(metadata: v1::ProtocolMetadata) -> v2::ProtocolMetadata {
	v2::ProtocolMetadata {
		envoy_lost_threshold: metadata.envoy_lost_threshold,
		actor_stop_threshold: metadata.actor_stop_threshold,
		max_response_payload_size: metadata.max_response_payload_size,
		sqlite_fast_path: None,
	}
}

fn convert_protocol_metadata_v2_to_v1(metadata: v2::ProtocolMetadata) -> v1::ProtocolMetadata {
	v1::ProtocolMetadata {
		envoy_lost_threshold: metadata.envoy_lost_threshold,
		actor_stop_threshold: metadata.actor_stop_threshold,
		max_response_payload_size: metadata.max_response_payload_size,
	}
}

fn convert_kv_request_data_v1_to_v2(data: v1::KvRequestData) -> v2::KvRequestData {
	match data {
		v1::KvRequestData::KvGetRequest(request) => {
			v2::KvRequestData::KvGetRequest(v2::KvGetRequest { keys: request.keys })
		}
		v1::KvRequestData::KvListRequest(request) => {
			v2::KvRequestData::KvListRequest(v2::KvListRequest {
				query: reencode(request.query).expect("v1 and v2 list queries match"),
				reverse: request.reverse,
				limit: request.limit,
			})
		}
		v1::KvRequestData::KvPutRequest(request) => {
			v2::KvRequestData::KvPutRequest(v2::KvPutRequest {
				keys: request.keys,
				values: request.values,
			})
		}
		v1::KvRequestData::KvDeleteRequest(request) => {
			v2::KvRequestData::KvDeleteRequest(v2::KvDeleteRequest { keys: request.keys })
		}
		v1::KvRequestData::KvDeleteRangeRequest(request) => {
			v2::KvRequestData::KvDeleteRangeRequest(v2::KvDeleteRangeRequest {
				start: request.start,
				end: request.end,
			})
		}
		v1::KvRequestData::KvDropRequest => v2::KvRequestData::KvDropRequest,
	}
}

fn convert_kv_request_data_v2_to_v1(data: v2::KvRequestData) -> Result<v1::KvRequestData> {
	match data {
		v2::KvRequestData::KvGetRequest(request) => {
			Ok(v1::KvRequestData::KvGetRequest(v1::KvGetRequest {
				keys: request.keys,
			}))
		}
		v2::KvRequestData::KvListRequest(request) => {
			Ok(v1::KvRequestData::KvListRequest(v1::KvListRequest {
				query: reencode(request.query)?,
				reverse: request.reverse,
				limit: request.limit,
			}))
		}
		v2::KvRequestData::KvPutRequest(request) => {
			Ok(v1::KvRequestData::KvPutRequest(v1::KvPutRequest {
				keys: request.keys,
				values: request.values,
			}))
		}
		v2::KvRequestData::KvDeleteRequest(request) => {
			Ok(v1::KvRequestData::KvDeleteRequest(v1::KvDeleteRequest {
				keys: request.keys,
			}))
		}
		v2::KvRequestData::KvDeleteRangeRequest(request) => Ok(
			v1::KvRequestData::KvDeleteRangeRequest(v1::KvDeleteRangeRequest {
				start: request.start,
				end: request.end,
			}),
		),
		v2::KvRequestData::KvSqliteWriteBatchRequest(_) => {
			bail!("KvSqliteWriteBatchRequest requires envoy protocol v2")
		}
		v2::KvRequestData::KvSqliteTruncateRequest(_) => {
			bail!("KvSqliteTruncateRequest requires envoy protocol v2")
		}
		v2::KvRequestData::KvDropRequest => Ok(v1::KvRequestData::KvDropRequest),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use vbare::OwnedVersionedData;

	#[test]
	fn v1_protocol_metadata_upgrades_without_sqlite_fast_path() {
		let upgraded = convert_protocol_metadata_v1_to_v2(v1::ProtocolMetadata {
			envoy_lost_threshold: 1,
			actor_stop_threshold: 2,
			max_response_payload_size: 3,
		});

		assert_eq!(upgraded.envoy_lost_threshold, 1);
		assert_eq!(upgraded.actor_stop_threshold, 2);
		assert_eq!(upgraded.max_response_payload_size, 3);
		assert!(upgraded.sqlite_fast_path.is_none());
	}

	#[test]
	fn v2_protocol_metadata_downgrade_drops_sqlite_fast_path() {
		let downgraded = convert_protocol_metadata_v2_to_v1(v2::ProtocolMetadata {
			envoy_lost_threshold: 1,
			actor_stop_threshold: 2,
			max_response_payload_size: 3,
			sqlite_fast_path: Some(v2::SqliteFastPathCapability {
				protocol_version: 1,
				supports_write_batch: true,
				supports_truncate: false,
			}),
		});

		assert_eq!(downgraded.envoy_lost_threshold, 1);
		assert_eq!(downgraded.actor_stop_threshold, 2);
		assert_eq!(downgraded.max_response_payload_size, 3);
	}

	#[test]
	fn sqlite_write_batch_request_rejects_v1_downgrade() {
		let result = convert_kv_request_data_v2_to_v1(
			v2::KvRequestData::KvSqliteWriteBatchRequest(v2::KvSqliteWriteBatchRequest {
				file_tag: 0,
				meta_value: vec![1, 2, 3],
				page_updates: vec![v2::SqlitePageUpdate {
					chunk_index: 7,
					data: vec![4, 5, 6],
				}],
				fence: v2::SqliteFastPathFence {
					expected_fence: Some(41),
					request_fence: 42,
				},
			}),
		);

		assert!(result.is_err());
		assert_eq!(
			result.expect_err("should reject").to_string(),
			"KvSqliteWriteBatchRequest requires envoy protocol v2"
		);
	}

	#[test]
	fn sqlite_truncate_request_rejects_v1_downgrade() {
		let result = convert_kv_request_data_v2_to_v1(v2::KvRequestData::KvSqliteTruncateRequest(
			v2::KvSqliteTruncateRequest {
				file_tag: 1,
				meta_value: vec![9, 9],
				delete_chunks_from: 12,
				tail_chunk: Some(v2::SqlitePageUpdate {
					chunk_index: 11,
					data: vec![7, 8],
				}),
				fence: v2::SqliteFastPathFence {
					expected_fence: None,
					request_fence: 1,
				},
			},
		));

		assert!(result.is_err());
		assert_eq!(
			result.expect_err("should reject").to_string(),
			"KvSqliteTruncateRequest requires envoy protocol v2"
		);
	}

	#[test]
	fn to_envoy_init_downgrades_without_sqlite_fast_path_for_v1_clients() {
		let payload = <ToEnvoy as OwnedVersionedData>::wrap_latest(v2::ToEnvoy::ToEnvoyInit(
			v2::ToEnvoyInit {
				metadata: v2::ProtocolMetadata {
					envoy_lost_threshold: 11,
					actor_stop_threshold: 22,
					max_response_payload_size: 33,
					sqlite_fast_path: Some(v2::SqliteFastPathCapability {
						protocol_version: 1,
						supports_write_batch: true,
						supports_truncate: true,
					}),
				},
			},
		))
		.serialize(1)
		.expect("serialize init for v1 client");

		let decoded = <ToEnvoy as OwnedVersionedData>::deserialize_version(&payload, 1)
			.expect("deserialize downgraded init");
		let ToEnvoy::V1(v1::ToEnvoy::ToEnvoyInit(init)) = decoded else {
			panic!("expected v1 init");
		};

		assert_eq!(init.metadata.envoy_lost_threshold, 11);
		assert_eq!(init.metadata.actor_stop_threshold, 22);
		assert_eq!(init.metadata.max_response_payload_size, 33);
	}
}
