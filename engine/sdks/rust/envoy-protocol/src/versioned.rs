use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::{v1, v3};

fn ensure_to_envoy_v1_compatible(message: &v3::ToEnvoy) -> Result<()> {
	match message {
		v3::ToEnvoy::ToEnvoyCommands(commands) => {
			for command in commands {
				if let v3::Command::CommandStartActor(start) = &command.inner
					&& start.sqlite_startup_data.is_some()
				{
					bail!("sqlite startup data requires envoy-protocol v2");
				}
			}

			Ok(())
		}
		v3::ToEnvoy::ToEnvoySqliteGetPagesResponse(_)
		| v3::ToEnvoy::ToEnvoySqliteGetPageRangeResponse(_)
		| v3::ToEnvoy::ToEnvoySqliteCommitResponse(_)
		| v3::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(_)
		| v3::ToEnvoy::ToEnvoySqliteCommitStageResponse(_)
		| v3::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(_)
		| v3::ToEnvoy::ToEnvoySqlitePersistPreloadHintsResponse(_) => {
			bail!("sqlite responses require envoy-protocol v2")
		}
		_ => Ok(()),
	}
}

fn ensure_to_rivet_v1_compatible(message: &v3::ToRivet) -> Result<()> {
	match message {
		v3::ToRivet::ToRivetSqliteGetPagesRequest(_)
		| v3::ToRivet::ToRivetSqliteGetPageRangeRequest(_)
		| v3::ToRivet::ToRivetSqliteCommitRequest(_)
		| v3::ToRivet::ToRivetSqliteCommitStageBeginRequest(_)
		| v3::ToRivet::ToRivetSqliteCommitStageRequest(_)
		| v3::ToRivet::ToRivetSqliteCommitFinalizeRequest(_)
		| v3::ToRivet::ToRivetSqlitePersistPreloadHintsRequest(_) => {
			bail!("sqlite requests require envoy-protocol v2")
		}
		_ => Ok(()),
	}
}

fn ensure_to_envoy_v2_compatible(message: &v3::ToEnvoy) -> Result<()> {
	match message {
		v3::ToEnvoy::ToEnvoySqliteGetPageRangeResponse(_) => {
			bail!("sqlite range responses require envoy-protocol v3")
		}
		v3::ToEnvoy::ToEnvoyInit(_)
		| v3::ToEnvoy::ToEnvoyCommands(_)
		| v3::ToEnvoy::ToEnvoyAckEvents(_)
		| v3::ToEnvoy::ToEnvoyKvResponse(_)
		| v3::ToEnvoy::ToEnvoyTunnelMessage(_)
		| v3::ToEnvoy::ToEnvoyPing(_)
		| v3::ToEnvoy::ToEnvoySqliteGetPagesResponse(_)
		| v3::ToEnvoy::ToEnvoySqliteCommitResponse(_)
		| v3::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(_)
		| v3::ToEnvoy::ToEnvoySqliteCommitStageResponse(_)
		| v3::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(_)
		| v3::ToEnvoy::ToEnvoySqlitePersistPreloadHintsResponse(_) => Ok(()),
	}
}

fn ensure_to_rivet_v2_compatible(message: &v3::ToRivet) -> Result<()> {
	match message {
		v3::ToRivet::ToRivetSqliteGetPageRangeRequest(_) => {
			bail!("sqlite range requests require envoy-protocol v3")
		}
		v3::ToRivet::ToRivetMetadata(_)
		| v3::ToRivet::ToRivetEvents(_)
		| v3::ToRivet::ToRivetAckCommands(_)
		| v3::ToRivet::ToRivetStopping
		| v3::ToRivet::ToRivetPong(_)
		| v3::ToRivet::ToRivetKvRequest(_)
		| v3::ToRivet::ToRivetTunnelMessage(_)
		| v3::ToRivet::ToRivetSqliteGetPagesRequest(_)
		| v3::ToRivet::ToRivetSqliteCommitRequest(_)
		| v3::ToRivet::ToRivetSqliteCommitStageBeginRequest(_)
		| v3::ToRivet::ToRivetSqliteCommitStageRequest(_)
		| v3::ToRivet::ToRivetSqliteCommitFinalizeRequest(_)
		| v3::ToRivet::ToRivetSqlitePersistPreloadHintsRequest(_) => Ok(()),
	}
}

macro_rules! impl_versioned_same_bytes {
	($name:ident, $latest_ty:path) => {
		pub enum $name {
			V3($latest_ty),
		}

		impl OwnedVersionedData for $name {
			type Latest = $latest_ty;

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
					1 | 2 | 3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
					_ => bail!("invalid version: {version}"),
				}
			}

			fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
				match version {
					1 | 2 | 3 => match self {
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
	};
}

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
			1 => match serde_bare::from_slice(payload) {
				Ok(data) => Ok(Self::V3(data)),
				Err(_) => Ok(Self::V3(convert_to_envoy_v1_to_v2(
					serde_bare::from_slice(payload)?,
				)?)),
			},
			2 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V3(data) => match data {
					v3::ToEnvoy::ToEnvoyCommands(commands) => {
						serde_bare::to_vec(&v1::ToEnvoy::ToEnvoyCommands(
							commands
								.into_iter()
								.map(convert_command_wrapper_v2_to_v1)
								.collect::<Result<Vec<_>>>()?,
						))
						.map_err(Into::into)
					}
					other => {
						ensure_to_envoy_v1_compatible(&other)?;
						serde_bare::to_vec(&other).map_err(Into::into)
					}
				},
			},
			2 => match self {
				Self::V3(data) => {
					ensure_to_envoy_v2_compatible(&data)?;
					serde_bare::to_vec(&data).map_err(Into::into)
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
			1 | 2 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V3(data) => {
					ensure_to_rivet_v1_compatible(&data)?;
					serde_bare::to_vec(&data).map_err(Into::into)
				}
			},
			2 => match self {
				Self::V3(data) => {
					ensure_to_rivet_v2_compatible(&data)?;
					serde_bare::to_vec(&data).map_err(Into::into)
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

impl_versioned_same_bytes!(ToEnvoyConn, v3::ToEnvoyConn);
impl_versioned_same_bytes!(ToGateway, v3::ToGateway);
impl_versioned_same_bytes!(ToOutbound, v3::ToOutbound);

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
			1 => Ok(Self::V3(convert_actor_command_key_data_v1_to_v2(
				serde_bare::from_slice(payload)?,
			)?)),
			2 | 3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V3(data) => {
					serde_bare::to_vec(&convert_actor_command_key_data_v2_to_v1(data)?)
						.map_err(Into::into)
				}
			},
			2 => match self {
				Self::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
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

fn convert_to_envoy_v1_to_v2(message: v1::ToEnvoy) -> Result<v3::ToEnvoy> {
	Ok(match message {
		v1::ToEnvoy::ToEnvoyCommands(commands) => v3::ToEnvoy::ToEnvoyCommands(
			commands
				.into_iter()
				.map(convert_command_wrapper_v1_to_v2)
				.collect::<Result<Vec<_>>>()?,
		),
		_ => bail!("unexpected envoy v1 payload requiring conversion"),
	})
}

fn convert_command_wrapper_v1_to_v2(wrapper: v1::CommandWrapper) -> Result<v3::CommandWrapper> {
	Ok(v3::CommandWrapper {
		checkpoint: v3::ActorCheckpoint {
			actor_id: wrapper.checkpoint.actor_id,
			generation: wrapper.checkpoint.generation,
			index: wrapper.checkpoint.index,
		},
		inner: convert_command_v1_to_v2(wrapper.inner)?,
	})
}

fn convert_command_wrapper_v2_to_v1(wrapper: v3::CommandWrapper) -> Result<v1::CommandWrapper> {
	Ok(v1::CommandWrapper {
		checkpoint: v1::ActorCheckpoint {
			actor_id: wrapper.checkpoint.actor_id,
			generation: wrapper.checkpoint.generation,
			index: wrapper.checkpoint.index,
		},
		inner: convert_command_v2_to_v1(wrapper.inner)?,
	})
}

fn convert_command_v1_to_v2(command: v1::Command) -> Result<v3::Command> {
	Ok(match command {
		v1::Command::CommandStartActor(start) => {
			v3::Command::CommandStartActor(convert_command_start_actor_v1_to_v2(start))
		}
		v1::Command::CommandStopActor(stop) => {
			v3::Command::CommandStopActor(v3::CommandStopActor {
				reason: convert_stop_actor_reason_v1_to_v2(stop.reason),
			})
		}
	})
}

fn convert_command_v2_to_v1(command: v3::Command) -> Result<v1::Command> {
	Ok(match command {
		v3::Command::CommandStartActor(start) => {
			v1::Command::CommandStartActor(convert_command_start_actor_v2_to_v1(start)?)
		}
		v3::Command::CommandStopActor(stop) => {
			v1::Command::CommandStopActor(v1::CommandStopActor {
				reason: convert_stop_actor_reason_v2_to_v1(stop.reason),
			})
		}
	})
}

fn convert_command_start_actor_v1_to_v2(start: v1::CommandStartActor) -> v3::CommandStartActor {
	v3::CommandStartActor {
		config: v3::ActorConfig {
			name: start.config.name,
			key: start.config.key,
			create_ts: start.config.create_ts,
			input: start.config.input,
		},
		hibernating_requests: start
			.hibernating_requests
			.into_iter()
			.map(|request| v3::HibernatingRequest {
				gateway_id: request.gateway_id,
				request_id: request.request_id,
			})
			.collect(),
		preloaded_kv: start.preloaded_kv.map(convert_preloaded_kv_v1_to_v2),
		sqlite_startup_data: None,
	}
}

fn convert_command_start_actor_v2_to_v1(
	start: v3::CommandStartActor,
) -> Result<v1::CommandStartActor> {
	if start.sqlite_startup_data.is_some() {
		bail!("sqlite startup data requires envoy-protocol v2");
	}

	Ok(v1::CommandStartActor {
		config: v1::ActorConfig {
			name: start.config.name,
			key: start.config.key,
			create_ts: start.config.create_ts,
			input: start.config.input,
		},
		hibernating_requests: start
			.hibernating_requests
			.into_iter()
			.map(|request| v1::HibernatingRequest {
				gateway_id: request.gateway_id,
				request_id: request.request_id,
			})
			.collect(),
		preloaded_kv: start.preloaded_kv.map(convert_preloaded_kv_v2_to_v1),
	})
}

fn convert_preloaded_kv_v1_to_v2(preloaded: v1::PreloadedKv) -> v3::PreloadedKv {
	v3::PreloadedKv {
		entries: preloaded
			.entries
			.into_iter()
			.map(|entry| v3::PreloadedKvEntry {
				key: entry.key,
				value: entry.value,
				metadata: v3::KvMetadata {
					version: entry.metadata.version,
					update_ts: entry.metadata.update_ts,
				},
			})
			.collect(),
		requested_get_keys: preloaded.requested_get_keys,
		requested_prefixes: preloaded.requested_prefixes,
	}
}

fn convert_preloaded_kv_v2_to_v1(preloaded: v3::PreloadedKv) -> v1::PreloadedKv {
	v1::PreloadedKv {
		entries: preloaded
			.entries
			.into_iter()
			.map(|entry| v1::PreloadedKvEntry {
				key: entry.key,
				value: entry.value,
				metadata: v1::KvMetadata {
					version: entry.metadata.version,
					update_ts: entry.metadata.update_ts,
				},
			})
			.collect(),
		requested_get_keys: preloaded.requested_get_keys,
		requested_prefixes: preloaded.requested_prefixes,
	}
}

fn convert_actor_command_key_data_v1_to_v2(
	data: v1::ActorCommandKeyData,
) -> Result<v3::ActorCommandKeyData> {
	Ok(match data {
		v1::ActorCommandKeyData::CommandStartActor(start) => {
			v3::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v1_to_v2(start))
		}
		v1::ActorCommandKeyData::CommandStopActor(stop) => {
			v3::ActorCommandKeyData::CommandStopActor(v3::CommandStopActor {
				reason: convert_stop_actor_reason_v1_to_v2(stop.reason),
			})
		}
	})
}

fn convert_actor_command_key_data_v2_to_v1(
	data: v3::ActorCommandKeyData,
) -> Result<v1::ActorCommandKeyData> {
	Ok(match data {
		v3::ActorCommandKeyData::CommandStartActor(start) => {
			v1::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v2_to_v1(start)?)
		}
		v3::ActorCommandKeyData::CommandStopActor(stop) => {
			v1::ActorCommandKeyData::CommandStopActor(v1::CommandStopActor {
				reason: convert_stop_actor_reason_v2_to_v1(stop.reason),
			})
		}
	})
}

fn convert_stop_actor_reason_v1_to_v2(reason: v1::StopActorReason) -> v3::StopActorReason {
	match reason {
		v1::StopActorReason::SleepIntent => v3::StopActorReason::SleepIntent,
		v1::StopActorReason::StopIntent => v3::StopActorReason::StopIntent,
		v1::StopActorReason::Destroy => v3::StopActorReason::Destroy,
		v1::StopActorReason::GoingAway => v3::StopActorReason::GoingAway,
		v1::StopActorReason::Lost => v3::StopActorReason::Lost,
	}
}

fn convert_stop_actor_reason_v2_to_v1(reason: v3::StopActorReason) -> v1::StopActorReason {
	match reason {
		v3::StopActorReason::SleepIntent => v1::StopActorReason::SleepIntent,
		v3::StopActorReason::StopIntent => v1::StopActorReason::StopIntent,
		v3::StopActorReason::Destroy => v1::StopActorReason::Destroy,
		v3::StopActorReason::GoingAway => v1::StopActorReason::GoingAway,
		v3::StopActorReason::Lost => v1::StopActorReason::Lost,
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use vbare::OwnedVersionedData;

	use super::{ActorCommandKeyData, ToEnvoy, ToRivet};
	use crate::generated::{v1, v3};

	#[test]
	fn v1_start_command_deserializes_into_v2_with_empty_sqlite_startup_data() -> Result<()> {
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
	fn sqlite_startup_data_cannot_serialize_back_to_v1() {
		let result = ToEnvoy::wrap_latest(v3::ToEnvoy::ToEnvoyCommands(vec![v3::CommandWrapper {
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
		.serialize_version(1);

		assert!(result.is_err());
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

	#[test]
	fn sqlite_range_request_requires_v3() {
		let message = ToRivet::wrap_latest(v3::ToRivet::ToRivetSqliteGetPageRangeRequest(
			v3::ToRivetSqliteGetPageRangeRequest {
				request_id: 1,
				data: v3::SqliteGetPageRangeRequest {
					actor_id: "actor".into(),
					generation: 7,
					start_pgno: 1,
					max_pages: 64,
					max_bytes: 256 * 1024,
				},
			},
		));

		assert!(message.serialize_version(2).is_err());
	}

	#[test]
	fn sqlite_range_request_serializes_at_v3() -> Result<()> {
		let message = ToRivet::wrap_latest(v3::ToRivet::ToRivetSqliteGetPageRangeRequest(
			v3::ToRivetSqliteGetPageRangeRequest {
				request_id: 1,
				data: v3::SqliteGetPageRangeRequest {
					actor_id: "actor".into(),
					generation: 7,
					start_pgno: 1,
					max_pages: 64,
					max_bytes: 256 * 1024,
				},
			},
		));

		let encoded = message.serialize(3)?;
		let decoded = ToRivet::deserialize(&encoded, 3)?;

		assert!(matches!(
			decoded,
			v3::ToRivet::ToRivetSqliteGetPageRangeRequest(_)
		));

		Ok(())
	}

	#[test]
	fn sqlite_range_response_requires_v3() {
		let message = ToEnvoy::wrap_latest(v3::ToEnvoy::ToEnvoySqliteGetPageRangeResponse(
			v3::ToEnvoySqliteGetPageRangeResponse {
				request_id: 1,
				data: v3::SqliteGetPageRangeResponse::SqliteGetPageRangeOk(
					v3::SqliteGetPageRangeOk {
						start_pgno: 1,
						pages: Vec::new(),
						meta: v3::SqliteMeta {
							generation: 7,
							head_txid: 1,
							materialized_txid: 1,
							db_size_pages: 0,
							page_size: 4096,
							creation_ts_ms: 0,
							max_delta_bytes: 8 * 1024 * 1024,
						},
					},
				),
			},
		));

		assert!(message.serialize_version(2).is_err());
	}
}
