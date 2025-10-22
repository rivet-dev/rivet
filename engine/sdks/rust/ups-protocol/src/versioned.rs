use anyhow::{Ok, Result, bail};
use vbare::OwnedVersionedData;

use crate::{PROTOCOL_VERSION, generated::v1};

pub enum UpsMessage {
	V1(v1::UpsMessage),
}

impl OwnedVersionedData for UpsMessage {
	type Latest = v1::UpsMessage;

	fn latest(latest: v1::UpsMessage) -> Self {
		UpsMessage::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let UpsMessage::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(UpsMessage::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			UpsMessage::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl UpsMessage {
	pub fn deserialize(buf: &[u8]) -> Result<v1::UpsMessage> {
		<Self as OwnedVersionedData>::deserialize(buf, PROTOCOL_VERSION)
	}

	pub fn serialize(self) -> Result<Vec<u8>> {
		<Self as OwnedVersionedData>::serialize(self, PROTOCOL_VERSION)
	}
}
