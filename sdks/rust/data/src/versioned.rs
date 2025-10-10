use anyhow::{Ok, Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::*;

pub enum RunnerAllocIdxKeyData {
	V1(pegboard_namespace_runner_alloc_idx_v1::Data),
}

impl OwnedVersionedData for RunnerAllocIdxKeyData {
	type Latest = pegboard_namespace_runner_alloc_idx_v1::Data;

	fn latest(latest: pegboard_namespace_runner_alloc_idx_v1::Data) -> Self {
		RunnerAllocIdxKeyData::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let RunnerAllocIdxKeyData::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(RunnerAllocIdxKeyData::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			RunnerAllocIdxKeyData::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub enum MetadataKeyData {
	V1(pegboard_runner_metadata_v1::Data),
}

impl OwnedVersionedData for MetadataKeyData {
	type Latest = pegboard_runner_metadata_v1::Data;

	fn latest(latest: pegboard_runner_metadata_v1::Data) -> Self {
		MetadataKeyData::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let MetadataKeyData::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(MetadataKeyData::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			MetadataKeyData::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub enum ActorByKeyKeyData {
	V1(pegboard_namespace_actor_by_key_v1::Data),
}

impl OwnedVersionedData for ActorByKeyKeyData {
	type Latest = pegboard_namespace_actor_by_key_v1::Data;

	fn latest(latest: pegboard_namespace_actor_by_key_v1::Data) -> Self {
		ActorByKeyKeyData::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ActorByKeyKeyData::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ActorByKeyKeyData::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ActorByKeyKeyData::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub enum RunnerByKeyKeyData {
	V1(pegboard_namespace_runner_by_key_v1::Data),
}

impl OwnedVersionedData for RunnerByKeyKeyData {
	type Latest = pegboard_namespace_runner_by_key_v1::Data;

	fn latest(latest: pegboard_namespace_runner_by_key_v1::Data) -> Self {
		RunnerByKeyKeyData::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let RunnerByKeyKeyData::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(RunnerByKeyKeyData::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			RunnerByKeyKeyData::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub enum ActorNameKeyData {
	V1(pegboard_namespace_actor_name_v1::Data),
}

impl OwnedVersionedData for ActorNameKeyData {
	type Latest = pegboard_namespace_actor_name_v1::Data;

	fn latest(latest: pegboard_namespace_actor_name_v1::Data) -> Self {
		ActorNameKeyData::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ActorNameKeyData::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ActorNameKeyData::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ActorNameKeyData::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub enum NamespaceRunnerConfig {
	V1(namespace_runner_config_v1::Data),
	V2(namespace_runner_config_v2::RunnerConfig),
}

impl OwnedVersionedData for NamespaceRunnerConfig {
	type Latest = namespace_runner_config_v2::RunnerConfig;

	fn latest(latest: namespace_runner_config_v2::RunnerConfig) -> Self {
		NamespaceRunnerConfig::V2(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		match self {
			NamespaceRunnerConfig::V1(data) => match data {
				namespace_runner_config_v1::Data::Serverless(serverless) => {
					Ok(namespace_runner_config_v2::RunnerConfig {
						kind: namespace_runner_config_v2::RunnerConfigKind::Serverless(
							namespace_runner_config_v2::Serverless {
								url: serverless.url,
								headers: serverless.headers,
								request_lifespan: serverless.request_lifespan,
								slots_per_runner: serverless.slots_per_runner,
								min_runners: serverless.min_runners,
								max_runners: serverless.max_runners,
								runners_margin: serverless.runners_margin,
							},
						),
						metadata: None,
					})
				}
			},
			NamespaceRunnerConfig::V2(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(NamespaceRunnerConfig::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(NamespaceRunnerConfig::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			NamespaceRunnerConfig::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			NamespaceRunnerConfig::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}
