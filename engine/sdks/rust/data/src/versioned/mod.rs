use anyhow::{Ok, Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::*;

mod namespace_runner_config;

pub use namespace_runner_config::*;

pub enum RunnerAllocIdxKeyData {
	V1(pegboard_namespace_runner_alloc_idx_v1::Data),
	V2(pegboard_namespace_runner_alloc_idx_v2::Data),
}

impl OwnedVersionedData for RunnerAllocIdxKeyData {
	type Latest = pegboard_namespace_runner_alloc_idx_v2::Data;

	fn wrap_latest(latest: pegboard_namespace_runner_alloc_idx_v2::Data) -> Self {
		RunnerAllocIdxKeyData::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let RunnerAllocIdxKeyData::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(RunnerAllocIdxKeyData::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(RunnerAllocIdxKeyData::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			RunnerAllocIdxKeyData::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			RunnerAllocIdxKeyData::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}

pub enum MetadataKeyData {
	V1(pegboard_runner_metadata_v1::Data),
}

impl OwnedVersionedData for MetadataKeyData {
	type Latest = pegboard_runner_metadata_v1::Data;

	fn wrap_latest(latest: pegboard_runner_metadata_v1::Data) -> Self {
		MetadataKeyData::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
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

	fn wrap_latest(latest: pegboard_namespace_actor_by_key_v1::Data) -> Self {
		ActorByKeyKeyData::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
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

	fn wrap_latest(latest: pegboard_namespace_runner_by_key_v1::Data) -> Self {
		RunnerByKeyKeyData::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
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

	fn wrap_latest(latest: pegboard_namespace_actor_name_v1::Data) -> Self {
		ActorNameKeyData::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
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
