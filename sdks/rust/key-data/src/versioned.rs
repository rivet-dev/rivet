use anyhow::{Ok, Result, bail};
use versioned_data_util::OwnedVersionedData;

use crate::generated::*;

pub enum RunnerAllocIdxKeyData {
	V1(pegboard_datacenter_runner_alloc_idx_v1::Data),
}

impl OwnedVersionedData for RunnerAllocIdxKeyData {
	type Latest = pegboard_datacenter_runner_alloc_idx_v1::Data;

	fn latest(latest: pegboard_datacenter_runner_alloc_idx_v1::Data) -> Self {
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

pub enum AddressKeyData {
	V1(pegboard_runner_address_v1::Data),
}

impl OwnedVersionedData for AddressKeyData {
	type Latest = pegboard_runner_address_v1::Data;

	fn latest(latest: pegboard_runner_address_v1::Data) -> Self {
		AddressKeyData::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let AddressKeyData::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(AddressKeyData::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			AddressKeyData::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
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
