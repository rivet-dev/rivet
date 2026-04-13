use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::v2;

pub enum CommittedValue {
	V2(v2::CommittedValue),
}

impl OwnedVersionedData for CommittedValue {
	type Latest = v2::CommittedValue;

	fn wrap_latest(latest: v2::CommittedValue) -> Self {
		CommittedValue::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let CommittedValue::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			2 => Ok(CommittedValue::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			CommittedValue::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}
}

pub enum CachedValue {
	V2(v2::CachedValue),
}

impl OwnedVersionedData for CachedValue {
	type Latest = v2::CachedValue;

	fn wrap_latest(latest: v2::CachedValue) -> Self {
		CachedValue::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let CachedValue::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			2 => Ok(CachedValue::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			CachedValue::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}
}
