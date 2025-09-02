use anyhow::{Ok, Result, bail};
use versioned_data_util::OwnedVersionedData;

use crate::{PROTOCOL_VERSION, generated::v1};

pub enum Request {
	V1(v1::Request),
}

impl OwnedVersionedData for Request {
	type Latest = v1::Request;

	fn latest(latest: v1::Request) -> Self {
		Request::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let Request::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Request::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Request::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl Request {
	pub fn serialize(self) -> Result<Vec<u8>> {
		<Self as OwnedVersionedData>::serialize(self, PROTOCOL_VERSION)
	}
}

pub enum Response {
	V1(v1::Response),
}

impl OwnedVersionedData for Response {
	type Latest = v1::Response;

	fn latest(latest: v1::Response) -> Self {
		Response::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let Response::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Response::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Response::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl Response {
	pub fn deserialize(buf: &[u8]) -> Result<v1::Response> {
		<Self as OwnedVersionedData>::deserialize(buf, PROTOCOL_VERSION)
	}
}

pub enum LogEntry {
	V1(v1::LogEntry),
}

impl OwnedVersionedData for LogEntry {
	type Latest = v1::LogEntry;

	fn latest(latest: v1::LogEntry) -> Self {
		LogEntry::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let LogEntry::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(LogEntry::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			LogEntry::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl LogEntry {
	pub fn serialize(self) -> Result<Vec<u8>> {
		<Self as OwnedVersionedData>::serialize(self, PROTOCOL_VERSION)
	}

	pub fn deserialize(buf: &[u8]) -> Result<v1::LogEntry> {
		<Self as OwnedVersionedData>::deserialize(buf, PROTOCOL_VERSION)
	}
}

pub enum ClusterConfig {
	V1(v1::ClusterConfig),
}

impl OwnedVersionedData for ClusterConfig {
	type Latest = v1::ClusterConfig;

	fn latest(latest: v1::ClusterConfig) -> Self {
		ClusterConfig::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ClusterConfig::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ClusterConfig::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ClusterConfig::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

impl ClusterConfig {
	pub fn serialize(self) -> Result<Vec<u8>> {
		<Self as OwnedVersionedData>::serialize(self, PROTOCOL_VERSION)
	}

	pub fn deserialize(buf: &[u8]) -> Result<v1::ClusterConfig> {
		<Self as OwnedVersionedData>::deserialize(buf, PROTOCOL_VERSION)
	}
}

pub enum Ballot {
	V1(v1::Ballot),
}

impl OwnedVersionedData for Ballot {
	type Latest = v1::Ballot;

	fn latest(latest: v1::Ballot) -> Self {
		Ballot::V1(latest)
	}

	fn into_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let Ballot::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Ballot::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Ballot::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}
