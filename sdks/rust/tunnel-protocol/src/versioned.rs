use anyhow::{Ok, Result, bail};
use versioned_data_util::OwnedVersionedData;

use crate::{PROTOCOL_VERSION, generated::v1};

pub enum RunnerMessage {
	V1(v1::RunnerMessage),
}

impl OwnedVersionedData for RunnerMessage {
	type Latest = v1::RunnerMessage;

	fn latest(latest: v1::RunnerMessage) -> Self {
		RunnerMessage::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let RunnerMessage::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(RunnerMessage::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			RunnerMessage::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl RunnerMessage {
	pub fn deserialize(buf: &[u8]) -> Result<v1::RunnerMessage> {
		<Self as OwnedVersionedData>::deserialize(buf, PROTOCOL_VERSION)
	}

	pub fn serialize(self) -> Result<Vec<u8>> {
		<Self as OwnedVersionedData>::serialize(self, PROTOCOL_VERSION)
	}
}

pub enum PubSubMessage {
	V1(v1::PubSubMessage),
}

impl OwnedVersionedData for PubSubMessage {
	type Latest = v1::PubSubMessage;

	fn latest(latest: v1::PubSubMessage) -> Self {
		PubSubMessage::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let PubSubMessage::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(PubSubMessage::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			PubSubMessage::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl PubSubMessage {
	pub fn deserialize(buf: &[u8]) -> Result<v1::PubSubMessage> {
		<Self as OwnedVersionedData>::deserialize(buf, PROTOCOL_VERSION)
	}

	pub fn serialize(self) -> Result<Vec<u8>> {
		<Self as OwnedVersionedData>::serialize(self, PROTOCOL_VERSION)
	}
}
