use anyhow::{Ok, Result, bail};
use versioned_data_util::OwnedVersionedData;

use crate::{PROTOCOL_VERSION, generated::v1};

pub enum TunnelMessage {
	V1(v1::TunnelMessage),
}

impl OwnedVersionedData for TunnelMessage {
	type Latest = v1::TunnelMessage;

	fn latest(latest: v1::TunnelMessage) -> Self {
		TunnelMessage::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let TunnelMessage::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(TunnelMessage::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			TunnelMessage::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl TunnelMessage {
	pub fn deserialize(buf: &[u8]) -> Result<v1::TunnelMessage> {
		<Self as OwnedVersionedData>::deserialize(buf, PROTOCOL_VERSION)
	}

	pub fn serialize(self) -> Result<Vec<u8>> {
		<Self as OwnedVersionedData>::serialize(self, PROTOCOL_VERSION)
	}
}
