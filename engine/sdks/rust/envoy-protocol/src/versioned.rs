use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::v1;

pub enum ToEnvoy {
	V1(v1::ToEnvoy),
}

impl OwnedVersionedData for ToEnvoy {
	type Latest = v1::ToEnvoy;

	fn wrap_latest(latest: v1::ToEnvoy) -> Self {
		ToEnvoy::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToEnvoy::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToEnvoy::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToEnvoy::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub enum ToEnvoyConn {
	V1(v1::ToEnvoyConn),
}

impl OwnedVersionedData for ToEnvoyConn {
	type Latest = v1::ToEnvoyConn;

	fn wrap_latest(latest: v1::ToEnvoyConn) -> Self {
		ToEnvoyConn::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToEnvoyConn::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToEnvoyConn::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToEnvoyConn::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub enum ToRivet {
	V1(v1::ToRivet),
}

impl OwnedVersionedData for ToRivet {
	type Latest = v1::ToRivet;

	fn wrap_latest(latest: v1::ToRivet) -> Self {
		ToRivet::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToRivet::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToRivet::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToRivet::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub enum ToGateway {
	V1(v1::ToGateway),
}

impl OwnedVersionedData for ToGateway {
	type Latest = v1::ToGateway;

	fn wrap_latest(latest: v1::ToGateway) -> Self {
		ToGateway::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
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

pub enum ToOutbound {
	V1(v1::ToOutbound),
}

impl OwnedVersionedData for ToOutbound {
	type Latest = v1::ToOutbound;

	fn wrap_latest(latest: v1::ToOutbound) -> Self {
		ToOutbound::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ToOutbound::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ToOutbound::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ToOutbound::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub enum ActorCommandKeyData {
	V1(v1::ActorCommandKeyData),
}

impl OwnedVersionedData for ActorCommandKeyData {
	type Latest = v1::ActorCommandKeyData;

	fn wrap_latest(latest: v1::ActorCommandKeyData) -> Self {
		ActorCommandKeyData::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let ActorCommandKeyData::V1(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(ActorCommandKeyData::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			ActorCommandKeyData::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}
