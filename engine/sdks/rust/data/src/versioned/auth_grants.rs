use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::auth_grants_v1;

pub enum AuthGrantsData {
	V1(auth_grants_v1::Data),
}

impl OwnedVersionedData for AuthGrantsData {
	type Latest = auth_grants_v1::Data;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let Self::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest")
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}
