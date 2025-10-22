use anyhow::{Ok, Result, bail};
use vbare::OwnedVersionedData;

use crate::{PROTOCOL_VERSION, generated::v1};

pub enum ToClient {
	V1(v1::ToClient),
}

impl OwnedVersionedData for ToClient {
	type Latest = v1::ToClient;

	fn latest(latest: v1::ToClient) -> Self {
		ToClient::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToClient::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToClient::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToClient::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub enum ToServer {
	V1(v1::ToServer),
}

impl OwnedVersionedData for ToServer {
	type Latest = v1::ToServer;

	fn latest(latest: v1::ToServer) -> Self {
		ToServer::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToServer::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToServer::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToServer::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub enum ToGateway {
	V1(v1::ToGateway),
}

impl OwnedVersionedData for ToGateway {
	type Latest = v1::ToGateway;

	fn latest(latest: v1::ToGateway) -> Self {
		ToGateway::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToGateway::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToGateway::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToGateway::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl ToGateway {
	pub fn serialize(self) -> Result<Vec<u8>> {
		<Self as OwnedVersionedData>::serialize(self, PROTOCOL_VERSION)
	}
}

pub enum ToServerlessServer {
	V1(v1::ToServerlessServer),
}

impl OwnedVersionedData for ToServerlessServer {
	type Latest = v1::ToServerlessServer;

	fn latest(latest: v1::ToServerlessServer) -> Self {
		ToServerlessServer::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToServerlessServer::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToServerlessServer::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToServerlessServer::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}
