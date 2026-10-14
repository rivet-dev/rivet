use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::auth_webhook_key_ring_v1;

pub enum AuthWebhookKeyRingData {
	V1(auth_webhook_key_ring_v1::Data),
}

impl OwnedVersionedData for AuthWebhookKeyRingData {
	type Latest = auth_webhook_key_ring_v1::Data;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(value) => Ok(value),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("unsupported webhook key ring version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(value) => Ok(serde_bare::to_vec(&value)?),
		}
	}
}
