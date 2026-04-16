use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::{v1, v2};

const SQLITE_SCHEMA_VERSION_V1: u32 = 1;
#[cfg(test)]
const SQLITE_SCHEMA_VERSION_V2: u32 = 2;

fn ensure_to_envoy_v1_compatible(message: &v2::ToEnvoy) -> Result<()> {
	match message {
		v2::ToEnvoy::ToEnvoyCommands(commands) => {
			for command in commands {
				if let v2::Command::CommandStartActor(start) = &command.inner
					&& (start.sqlite_schema_version != SQLITE_SCHEMA_VERSION_V1
						|| start.sqlite_startup_data.is_some())
				{
					bail!("sqlite v2 startup data requires envoy-protocol v2");
				}
			}

			Ok(())
		}
		v2::ToEnvoy::ToEnvoySqliteGetPagesResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitStageResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(_) => {
			bail!("sqlite responses require envoy-protocol v2")
		}
		_ => Ok(()),
	}
}

fn ensure_to_rivet_v1_compatible(message: &v2::ToRivet) -> Result<()> {
	match message {
		v2::ToRivet::ToRivetSqliteGetPagesRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitStageBeginRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitStageRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitFinalizeRequest(_) => {
			bail!("sqlite requests require envoy-protocol v2")
		}
		_ => Ok(()),
	}
}

macro_rules! impl_versioned_same_bytes {
	($name:ident, $latest_ty:path) => {
		pub enum $name {
			V2($latest_ty),
		}

		impl OwnedVersionedData for $name {
			type Latest = $latest_ty;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V2(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				match self {
					Self::V2(data) => Ok(data),
				}
			}

			fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
				match version {
					1 | 2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
					_ => bail!("invalid version: {version}"),
				}
			}

			fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
				match version {
					1 | 2 => match self {
						Self::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
					},
					_ => bail!("invalid version: {version}"),
				}
			}

			fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
				vec![Ok]
			}

			fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
				vec![Ok]
			}
		}
	};
}

pub enum ToEnvoy {
	V2(v2::ToEnvoy),
}

impl OwnedVersionedData for ToEnvoy {
	type Latest = v2::ToEnvoy;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V2(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => match serde_bare::from_slice(payload) {
				Ok(data) => Ok(Self::V2(data)),
				Err(_) => Ok(Self::V2(convert_to_envoy_v1_to_v2(
					serde_bare::from_slice(payload)?,
				)?)),
			},
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V2(data) => match data {
					v2::ToEnvoy::ToEnvoyCommands(commands) => {
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
				Self::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
			},
			_ => bail!("invalid version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}
}

pub enum ToRivet {
	V2(v2::ToRivet),
}

impl OwnedVersionedData for ToRivet {
	type Latest = v2::ToRivet;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V2(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 | 2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V2(data) => {
					ensure_to_rivet_v1_compatible(&data)?;
					serde_bare::to_vec(&data).map_err(Into::into)
				}
			},
			2 => match self {
				Self::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
			},
			_ => bail!("invalid version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}
}

impl_versioned_same_bytes!(ToEnvoyConn, v2::ToEnvoyConn);
impl_versioned_same_bytes!(ToGateway, v2::ToGateway);
impl_versioned_same_bytes!(ToOutbound, v2::ToOutbound);

pub enum ActorCommandKeyData {
	V2(v2::ActorCommandKeyData),
}

impl OwnedVersionedData for ActorCommandKeyData {
	type Latest = v2::ActorCommandKeyData;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V2(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V2(convert_actor_command_key_data_v1_to_v2(
				serde_bare::from_slice(payload)?,
			)?)),
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match version {
			1 => match self {
				Self::V2(data) => {
					serde_bare::to_vec(&convert_actor_command_key_data_v2_to_v1(data)?)
						.map_err(Into::into)
				}
			},
			2 => match self {
				Self::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
			},
			_ => bail!("invalid version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
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
		_ => bail!("unexpected envoy v1 payload requiring conversion"),
	})
}

fn convert_command_wrapper_v1_to_v2(wrapper: v1::CommandWrapper) -> Result<v2::CommandWrapper> {
	Ok(v2::CommandWrapper {
		checkpoint: v2::ActorCheckpoint {
			actor_id: wrapper.checkpoint.actor_id,
			generation: wrapper.checkpoint.generation,
			index: wrapper.checkpoint.index,
		},
		inner: convert_command_v1_to_v2(wrapper.inner)?,
	})
}

fn convert_command_wrapper_v2_to_v1(wrapper: v2::CommandWrapper) -> Result<v1::CommandWrapper> {
	Ok(v1::CommandWrapper {
		checkpoint: v1::ActorCheckpoint {
			actor_id: wrapper.checkpoint.actor_id,
			generation: wrapper.checkpoint.generation,
			index: wrapper.checkpoint.index,
		},
		inner: convert_command_v2_to_v1(wrapper.inner)?,
	})
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

fn convert_command_start_actor_v1_to_v2(start: v1::CommandStartActor) -> v2::CommandStartActor {
	v2::CommandStartActor {
		config: v2::ActorConfig {
			name: start.config.name,
			key: start.config.key,
			create_ts: start.config.create_ts,
			input: start.config.input,
		},
		hibernating_requests: start
			.hibernating_requests
			.into_iter()
			.map(|request| v2::HibernatingRequest {
				gateway_id: request.gateway_id,
				request_id: request.request_id,
			})
			.collect(),
		preloaded_kv: start.preloaded_kv.map(convert_preloaded_kv_v1_to_v2),
		sqlite_schema_version: SQLITE_SCHEMA_VERSION_V1,
		sqlite_startup_data: None,
	}
}

fn convert_command_start_actor_v2_to_v1(
	start: v2::CommandStartActor,
) -> Result<v1::CommandStartActor> {
	if start.sqlite_schema_version != SQLITE_SCHEMA_VERSION_V1 {
		bail!("sqlite schema version requires envoy-protocol v2");
	}
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

fn convert_preloaded_kv_v1_to_v2(preloaded: v1::PreloadedKv) -> v2::PreloadedKv {
	v2::PreloadedKv {
		entries: preloaded
			.entries
			.into_iter()
			.map(|entry| v2::PreloadedKvEntry {
				key: entry.key,
				value: entry.value,
				metadata: v2::KvMetadata {
					version: entry.metadata.version,
					update_ts: entry.metadata.update_ts,
				},
			})
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
) -> Result<v2::ActorCommandKeyData> {
	Ok(match data {
		v1::ActorCommandKeyData::CommandStartActor(start) => {
			v2::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v1_to_v2(start))
		}
		v1::ActorCommandKeyData::CommandStopActor(stop) => {
			v2::ActorCommandKeyData::CommandStopActor(v2::CommandStopActor {
				reason: convert_stop_actor_reason_v1_to_v2(stop.reason),
			})
		}
	})
}

fn convert_actor_command_key_data_v2_to_v1(
	data: v2::ActorCommandKeyData,
) -> Result<v1::ActorCommandKeyData> {
	Ok(match data {
		v2::ActorCommandKeyData::CommandStartActor(start) => {
			v1::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v2_to_v1(start)?)
		}
		v2::ActorCommandKeyData::CommandStopActor(stop) => {
			v1::ActorCommandKeyData::CommandStopActor(v1::CommandStopActor {
				reason: convert_stop_actor_reason_v2_to_v1(stop.reason),
			})
		}
	})
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

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use vbare::OwnedVersionedData;

	use super::{ActorCommandKeyData, SQLITE_SCHEMA_VERSION_V1, SQLITE_SCHEMA_VERSION_V2, ToEnvoy};
	use crate::generated::{v1, v2};

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
		let v2::ToEnvoy::ToEnvoyCommands(commands) = decoded else {
			panic!("expected commands");
		};
		let v2::Command::CommandStartActor(start) = &commands[0].inner else {
			panic!("expected start actor");
		};

		assert!(start.sqlite_startup_data.is_none());
		assert_eq!(start.sqlite_schema_version, SQLITE_SCHEMA_VERSION_V1);
		assert!(start.preloaded_kv.is_none());
		assert_eq!(commands[0].checkpoint.generation, 7);

		Ok(())
	}

	#[test]
	fn sqlite_startup_data_cannot_serialize_back_to_v1() {
		let result = ToEnvoy::wrap_latest(v2::ToEnvoy::ToEnvoyCommands(vec![v2::CommandWrapper {
			checkpoint: v2::ActorCheckpoint {
				actor_id: "actor".into(),
				generation: 1,
				index: 0,
			},
			inner: v2::Command::CommandStartActor(v2::CommandStartActor {
				config: v2::ActorConfig {
					name: "demo".into(),
					key: None,
					create_ts: 1,
					input: None,
				},
				hibernating_requests: Vec::new(),
				preloaded_kv: None,
				sqlite_schema_version: SQLITE_SCHEMA_VERSION_V2,
				sqlite_startup_data: Some(v2::SqliteStartupData {
					generation: 11,
					meta: v2::SqliteMeta {
						schema_version: 2,
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
		let encoded = ActorCommandKeyData::wrap_latest(v2::ActorCommandKeyData::CommandStartActor(
			v2::CommandStartActor {
				config: v2::ActorConfig {
					name: "demo".into(),
					key: None,
					create_ts: 7,
					input: None,
				},
				hibernating_requests: Vec::new(),
				preloaded_kv: None,
				sqlite_schema_version: SQLITE_SCHEMA_VERSION_V1,
				sqlite_startup_data: None,
			},
		))
		.serialize_version(1)?;

		let decoded = ActorCommandKeyData::deserialize_version(&encoded, 1)?.unwrap_latest()?;
		let v2::ActorCommandKeyData::CommandStartActor(start) = decoded else {
			panic!("expected start actor");
		};
		assert_eq!(start.sqlite_schema_version, SQLITE_SCHEMA_VERSION_V1);
		assert!(start.sqlite_startup_data.is_none());

		Ok(())
	}
}
