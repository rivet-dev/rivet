//! Versioned wrapper for the one persisted type, the parked record.

use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::v1;

pub enum ParkedRecord {
	V1(v1::ParkedRecord),
}

impl OwnedVersionedData for ParkedRecord {
	type Latest = v1::ParkedRecord;

	fn wrap_latest(latest: v1::ParkedRecord) -> Self {
		ParkedRecord::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			ParkedRecord::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ParkedRecord::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid parked record version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ParkedRecord::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok]
	}
}
