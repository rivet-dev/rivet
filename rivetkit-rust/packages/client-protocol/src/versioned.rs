use anyhow::{Result, bail};
use serde::{Serialize, de::DeserializeOwned};
use vbare::OwnedVersionedData;

use crate::generated::{v1, v2, v3, v4};

pub enum ToClient {
	V1(v1::ToClient),
	V2(v2::ToClient),
	V3(v3::ToClient),
	V4(v4::ToClient),
}

impl OwnedVersionedData for ToClient {
	type Latest = v4::ToClient;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V4(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V4(data) => Ok(data),
			_ => bail!("version not latest"),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid client protocol version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match (self, version) {
			(Self::V1(data), 1) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V2(data), 2) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V3(data), 3) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
			(_, version) => bail!("unexpected client protocol version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2, Self::v2_to_v3, Self::v3_to_v4]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v4_to_v3, Self::v3_to_v2, Self::v2_to_v1]
	}
}

impl ToClient {
	fn v1_to_v2(self) -> Result<Self> {
		let Self::V1(data) = self else {
			bail!("expected client protocol v1 ToClient")
		};

		let body = match data.body {
			v1::ToClientBody::Init(init) => v2::ToClientBody::Init(v2::Init {
				actor_id: init.actor_id,
				connection_id: init.connection_id,
			}),
			v1::ToClientBody::Error(error) => v2::ToClientBody::Error(v2::Error {
				group: error.group,
				code: error.code,
				message: error.message,
				metadata: error.metadata,
				action_id: error.action_id,
			}),
			v1::ToClientBody::ActionResponse(response) => {
				v2::ToClientBody::ActionResponse(v2::ActionResponse {
					id: response.id,
					output: response.output,
				})
			}
			v1::ToClientBody::Event(event) => v2::ToClientBody::Event(v2::Event {
				name: event.name,
				args: event.args,
			}),
		};

		Ok(Self::V2(v2::ToClient { body }))
	}

	fn v2_to_v3(self) -> Result<Self> {
		let Self::V2(data) = self else {
			bail!("expected client protocol v2 ToClient")
		};
		Ok(Self::V3(transcode_version(data)?))
	}

	fn v3_to_v4(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected client protocol v3 ToClient")
		};

		let body = match data.body {
			v3::ToClientBody::Init(init) => v4::ToClientBody::Init(v4::Init {
				actor_id: init.actor_id,
				connection_id: init.connection_id,
			}),
			v3::ToClientBody::Error(error) => v4::ToClientBody::Error(v4::Error {
				group: error.group,
				code: error.code,
				message: error.message,
				metadata: error.metadata,
				action_id: error.action_id,
				actor: None,
			}),
			v3::ToClientBody::ActionResponse(response) => {
				v4::ToClientBody::ActionResponse(v4::ActionResponse {
					id: response.id,
					output: response.output,
				})
			}
			v3::ToClientBody::Event(event) => v4::ToClientBody::Event(v4::Event {
				name: event.name,
				args: event.args,
			}),
		};

		Ok(Self::V4(v4::ToClient { body }))
	}

	fn v4_to_v3(self) -> Result<Self> {
		let Self::V4(data) = self else {
			bail!("expected client protocol v4 ToClient")
		};

		let body = match data.body {
			v4::ToClientBody::Init(init) => v3::ToClientBody::Init(v3::Init {
				actor_id: init.actor_id,
				connection_id: init.connection_id,
			}),
			v4::ToClientBody::Error(error) => v3::ToClientBody::Error(v3::Error {
				group: error.group,
				code: error.code,
				message: error.message,
				metadata: error.metadata,
				action_id: error.action_id,
			}),
			v4::ToClientBody::ActionResponse(response) => {
				v3::ToClientBody::ActionResponse(v3::ActionResponse {
					id: response.id,
					output: response.output,
				})
			}
			v4::ToClientBody::Event(event) => v3::ToClientBody::Event(v3::Event {
				name: event.name,
				args: event.args,
			}),
		};

		Ok(Self::V3(v3::ToClient { body }))
	}

	fn v3_to_v2(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected client protocol v3 ToClient")
		};
		Ok(Self::V2(transcode_version(data)?))
	}

	fn v2_to_v1(self) -> Result<Self> {
		let Self::V2(data) = self else {
			bail!("expected client protocol v2 ToClient")
		};

		let body = match data.body {
			v2::ToClientBody::Init(init) => v1::ToClientBody::Init(v1::Init {
				actor_id: init.actor_id,
				connection_id: init.connection_id,
				connection_token: String::new(),
			}),
			v2::ToClientBody::Error(error) => v1::ToClientBody::Error(v1::Error {
				group: error.group,
				code: error.code,
				message: error.message,
				metadata: error.metadata,
				action_id: error.action_id,
			}),
			v2::ToClientBody::ActionResponse(response) => {
				v1::ToClientBody::ActionResponse(v1::ActionResponse {
					id: response.id,
					output: response.output,
				})
			}
			v2::ToClientBody::Event(event) => v1::ToClientBody::Event(v1::Event {
				name: event.name,
				args: event.args,
			}),
		};

		Ok(Self::V1(v1::ToClient { body }))
	}
}

macro_rules! impl_versioned_transcoded {
	($name:ident, $latest_ty:path, $v1_ty:path, $v2_ty:path, $v3_ty:path, $v4_ty:path) => {
		pub enum $name {
			V1($v1_ty),
			V2($v2_ty),
			V3($v3_ty),
			V4($v4_ty),
		}

		impl OwnedVersionedData for $name {
			type Latest = $latest_ty;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V4(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				match self {
					Self::V4(data) => Ok(data),
					_ => bail!("version not latest"),
				}
			}

			fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
				match version {
					1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
					2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
					3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
					4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
					_ => bail!(
						"invalid client protocol version for {}: {version}",
						stringify!($name)
					),
				}
			}

			fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
				match (self, version) {
					(Self::V1(data), 1) => serde_bare::to_vec(&data).map_err(Into::into),
					(Self::V2(data), 2) => serde_bare::to_vec(&data).map_err(Into::into),
					(Self::V3(data), 3) => serde_bare::to_vec(&data).map_err(Into::into),
					(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
					(_, version) => bail!(
						"unexpected client protocol version for {}: {version}",
						stringify!($name)
					),
				}
			}

			fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
				vec![Self::v1_to_v2, Self::v2_to_v3, Self::v3_to_v4]
			}

			fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
				vec![Self::v4_to_v3, Self::v3_to_v2, Self::v2_to_v1]
			}
		}

		impl $name {
			fn v1_to_v2(self) -> Result<Self> {
				let Self::V1(data) = self else {
					bail!("expected client protocol v1 {}", stringify!($name))
				};
				Ok(Self::V2(transcode_version(data)?))
			}

			fn v2_to_v3(self) -> Result<Self> {
				let Self::V2(data) = self else {
					bail!("expected client protocol v2 {}", stringify!($name))
				};
				Ok(Self::V3(transcode_version(data)?))
			}

			fn v3_to_v4(self) -> Result<Self> {
				let Self::V3(data) = self else {
					bail!("expected client protocol v3 {}", stringify!($name))
				};
				Ok(Self::V4(transcode_version(data)?))
			}

			fn v4_to_v3(self) -> Result<Self> {
				let Self::V4(data) = self else {
					bail!("expected client protocol v4 {}", stringify!($name))
				};
				Ok(Self::V3(transcode_version(data)?))
			}

			fn v3_to_v2(self) -> Result<Self> {
				let Self::V3(data) = self else {
					bail!("expected client protocol v3 {}", stringify!($name))
				};
				Ok(Self::V2(transcode_version(data)?))
			}

			fn v2_to_v1(self) -> Result<Self> {
				let Self::V2(data) = self else {
					bail!("expected client protocol v2 {}", stringify!($name))
				};
				Ok(Self::V1(transcode_version(data)?))
			}
		}
	};
}

macro_rules! impl_versioned_v3_only {
	($name:ident, $latest_ty:path) => {
		pub enum $name {
			V3(v3::$name),
			V4($latest_ty),
		}

		impl OwnedVersionedData for $name {
			type Latest = $latest_ty;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V4(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				match self {
					Self::V4(data) => Ok(data),
					_ => bail!("version not latest"),
				}
			}

			fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
				match version {
					3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
					4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
					_ => bail!(
						"{} only exists in client protocol v3, got {version}",
						stringify!($name)
					),
				}
			}

			fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
				match (self, version) {
					(Self::V3(data), 3) => serde_bare::to_vec(&data).map_err(Into::into),
					(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
					(_, version) => bail!(
						"{} only exists in client protocol v3, got {version}",
						stringify!($name)
					),
				}
			}

			fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
				vec![Ok, Ok, Self::v3_to_v4]
			}

			fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
				vec![Self::v4_to_v3, Ok, Ok]
			}
		}

		impl $name {
			fn v3_to_v4(self) -> Result<Self> {
				let Self::V3(data) = self else {
					bail!("expected client protocol v3 {}", stringify!($name))
				};
				Ok(Self::V4(transcode_version(data)?))
			}

			fn v4_to_v3(self) -> Result<Self> {
				let Self::V4(data) = self else {
					bail!("expected client protocol v4 {}", stringify!($name))
				};
				Ok(Self::V3(transcode_version(data)?))
			}
		}
	};
}

impl_versioned_transcoded!(
	ToServer,
	v4::ToServer,
	v1::ToServer,
	v2::ToServer,
	v3::ToServer,
	v4::ToServer
);
impl_versioned_transcoded!(
	HttpActionRequest,
	v4::HttpActionRequest,
	v1::HttpActionRequest,
	v2::HttpActionRequest,
	v3::HttpActionRequest,
	v4::HttpActionRequest
);
impl_versioned_transcoded!(
	HttpActionResponse,
	v4::HttpActionResponse,
	v1::HttpActionResponse,
	v2::HttpActionResponse,
	v3::HttpActionResponse,
	v4::HttpActionResponse
);
impl_versioned_transcoded!(
	HttpResolveResponse,
	v4::HttpResolveResponse,
	v1::HttpResolveResponse,
	v2::HttpResolveResponse,
	v3::HttpResolveResponse,
	v4::HttpResolveResponse
);
impl_versioned_v3_only!(HttpQueueSendRequest, v4::HttpQueueSendRequest);
impl_versioned_v3_only!(HttpQueueSendResponse, v4::HttpQueueSendResponse);

pub enum HttpResponseError {
	V1(v1::HttpResponseError),
	V2(v2::HttpResponseError),
	V3(v3::HttpResponseError),
	V4(v4::HttpResponseError),
}

impl OwnedVersionedData for HttpResponseError {
	type Latest = v4::HttpResponseError;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V4(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V4(data) => Ok(data),
			_ => bail!("version not latest"),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid client protocol version for HttpResponseError: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match (self, version) {
			(Self::V1(data), 1) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V2(data), 2) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V3(data), 3) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
			(_, version) => bail!("unexpected client protocol version for HttpResponseError: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2, Self::v2_to_v3, Self::v3_to_v4]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v4_to_v3, Self::v3_to_v2, Self::v2_to_v1]
	}
}

impl HttpResponseError {
	fn v1_to_v2(self) -> Result<Self> {
		let Self::V1(data) = self else {
			bail!("expected client protocol v1 HttpResponseError")
		};
		Ok(Self::V2(transcode_version(data)?))
	}

	fn v2_to_v3(self) -> Result<Self> {
		let Self::V2(data) = self else {
			bail!("expected client protocol v2 HttpResponseError")
		};
		Ok(Self::V3(transcode_version(data)?))
	}

	fn v3_to_v4(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected client protocol v3 HttpResponseError")
		};
		Ok(Self::V4(v4::HttpResponseError {
			group: data.group,
			code: data.code,
			message: data.message,
			metadata: data.metadata,
			actor: None,
		}))
	}

	fn v4_to_v3(self) -> Result<Self> {
		let Self::V4(data) = self else {
			bail!("expected client protocol v4 HttpResponseError")
		};
		Ok(Self::V3(v3::HttpResponseError {
			group: data.group,
			code: data.code,
			message: data.message,
			metadata: data.metadata,
		}))
	}

	fn v3_to_v2(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected client protocol v3 HttpResponseError")
		};
		Ok(Self::V2(transcode_version(data)?))
	}

	fn v2_to_v1(self) -> Result<Self> {
		let Self::V2(data) = self else {
			bail!("expected client protocol v2 HttpResponseError")
		};
		Ok(Self::V1(transcode_version(data)?))
	}
}

fn transcode_version<From, To>(data: From) -> Result<To>
where
	From: Serialize,
	To: DeserializeOwned,
{
	let encoded = serde_bare::to_vec(&data)?;
	serde_bare::from_slice(&encoded).map_err(Into::into)
}
