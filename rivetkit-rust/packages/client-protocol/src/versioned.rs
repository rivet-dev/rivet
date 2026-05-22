use anyhow::{Result, bail};
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
		Ok(Self::V3(data.into()))
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
		Ok(Self::V2(data.into()))
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

macro_rules! impl_same_fields_pair {
	($left:ident, $right:ident, $ty:ident { $($field:ident),+ $(,)? }) => {
		impl From<$left::$ty> for $right::$ty {
			fn from(value: $left::$ty) -> Self {
				Self {
					$($field: value.$field),+
				}
			}
		}

		impl From<$right::$ty> for $left::$ty {
			fn from(value: $right::$ty) -> Self {
				Self {
					$($field: value.$field),+
				}
			}
		}
	};
}

macro_rules! impl_to_server_pair {
	($left:ident, $right:ident) => {
		impl From<$left::ToServerBody> for $right::ToServerBody {
			fn from(value: $left::ToServerBody) -> Self {
				match value {
					$left::ToServerBody::ActionRequest(request) => {
						Self::ActionRequest(request.into())
					}
					$left::ToServerBody::SubscriptionRequest(request) => {
						Self::SubscriptionRequest(request.into())
					}
				}
			}
		}

		impl From<$right::ToServerBody> for $left::ToServerBody {
			fn from(value: $right::ToServerBody) -> Self {
				match value {
					$right::ToServerBody::ActionRequest(request) => {
						Self::ActionRequest(request.into())
					}
					$right::ToServerBody::SubscriptionRequest(request) => {
						Self::SubscriptionRequest(request.into())
					}
				}
			}
		}

		impl From<$left::ToServer> for $right::ToServer {
			fn from(value: $left::ToServer) -> Self {
				Self {
					body: value.body.into(),
				}
			}
		}

		impl From<$right::ToServer> for $left::ToServer {
			fn from(value: $right::ToServer) -> Self {
				Self {
					body: value.body.into(),
				}
			}
		}
	};
}

macro_rules! impl_common_pair {
	($left:ident, $right:ident) => {
		impl_same_fields_pair!($left, $right, ActionRequest { id, name, args });
		impl_same_fields_pair!($left, $right, SubscriptionRequest {
			event_name,
			subscribe,
		});
		impl_to_server_pair!($left, $right);
		impl_same_fields_pair!($left, $right, HttpActionRequest { args });
		impl_same_fields_pair!($left, $right, HttpActionResponse { output });
		impl_same_fields_pair!($left, $right, HttpResolveResponse { actor_id });
	};
}

macro_rules! impl_to_client_v2_v3_pair {
	() => {
		impl_same_fields_pair!(v2, v3, Init {
			actor_id,
			connection_id,
		});
		impl_same_fields_pair!(v2, v3, Error {
			group,
			code,
			message,
			metadata,
			action_id,
		});
		impl_same_fields_pair!(v2, v3, ActionResponse { id, output });
		impl_same_fields_pair!(v2, v3, Event { name, args });

		impl From<v2::ToClientBody> for v3::ToClientBody {
			fn from(value: v2::ToClientBody) -> Self {
				match value {
					v2::ToClientBody::Init(init) => Self::Init(init.into()),
					v2::ToClientBody::Error(error) => Self::Error(error.into()),
					v2::ToClientBody::ActionResponse(response) => {
						Self::ActionResponse(response.into())
					}
					v2::ToClientBody::Event(event) => Self::Event(event.into()),
				}
			}
		}

		impl From<v3::ToClientBody> for v2::ToClientBody {
			fn from(value: v3::ToClientBody) -> Self {
				match value {
					v3::ToClientBody::Init(init) => Self::Init(init.into()),
					v3::ToClientBody::Error(error) => Self::Error(error.into()),
					v3::ToClientBody::ActionResponse(response) => {
						Self::ActionResponse(response.into())
					}
					v3::ToClientBody::Event(event) => Self::Event(event.into()),
				}
			}
		}

		impl From<v2::ToClient> for v3::ToClient {
			fn from(value: v2::ToClient) -> Self {
				Self {
					body: value.body.into(),
				}
			}
		}

		impl From<v3::ToClient> for v2::ToClient {
			fn from(value: v3::ToClient) -> Self {
				Self {
					body: value.body.into(),
				}
			}
		}
	};
}

impl_common_pair!(v1, v2);
impl_common_pair!(v2, v3);
impl_common_pair!(v3, v4);
impl_to_client_v2_v3_pair!();
impl_same_fields_pair!(v1, v2, HttpResponseError {
	group,
	code,
	message,
	metadata,
});
impl_same_fields_pair!(v2, v3, HttpResponseError {
	group,
	code,
	message,
	metadata,
});
impl_same_fields_pair!(v3, v4, HttpQueueSendRequest {
	body,
	name,
	wait,
	timeout,
});
impl_same_fields_pair!(v3, v4, HttpQueueSendResponse { status, response });

macro_rules! impl_versioned_manual {
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
				Ok(Self::V2(data.into()))
			}

			fn v2_to_v3(self) -> Result<Self> {
				let Self::V2(data) = self else {
					bail!("expected client protocol v2 {}", stringify!($name))
				};
				Ok(Self::V3(data.into()))
			}

			fn v3_to_v4(self) -> Result<Self> {
				let Self::V3(data) = self else {
					bail!("expected client protocol v3 {}", stringify!($name))
				};
				Ok(Self::V4(data.into()))
			}

			fn v4_to_v3(self) -> Result<Self> {
				let Self::V4(data) = self else {
					bail!("expected client protocol v4 {}", stringify!($name))
				};
				Ok(Self::V3(data.into()))
			}

			fn v3_to_v2(self) -> Result<Self> {
				let Self::V3(data) = self else {
					bail!("expected client protocol v3 {}", stringify!($name))
				};
				Ok(Self::V2(data.into()))
			}

			fn v2_to_v1(self) -> Result<Self> {
				let Self::V2(data) = self else {
					bail!("expected client protocol v2 {}", stringify!($name))
				};
				Ok(Self::V1(data.into()))
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
				Ok(Self::V4(data.into()))
			}

			fn v4_to_v3(self) -> Result<Self> {
				let Self::V4(data) = self else {
					bail!("expected client protocol v4 {}", stringify!($name))
				};
				Ok(Self::V3(data.into()))
			}
		}
	};
}

impl_versioned_manual!(
	ToServer,
	v4::ToServer,
	v1::ToServer,
	v2::ToServer,
	v3::ToServer,
	v4::ToServer
);
impl_versioned_manual!(
	HttpActionRequest,
	v4::HttpActionRequest,
	v1::HttpActionRequest,
	v2::HttpActionRequest,
	v3::HttpActionRequest,
	v4::HttpActionRequest
);
impl_versioned_manual!(
	HttpActionResponse,
	v4::HttpActionResponse,
	v1::HttpActionResponse,
	v2::HttpActionResponse,
	v3::HttpActionResponse,
	v4::HttpActionResponse
);
impl_versioned_manual!(
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
		Ok(Self::V2(data.into()))
	}

	fn v2_to_v3(self) -> Result<Self> {
		let Self::V2(data) = self else {
			bail!("expected client protocol v2 HttpResponseError")
		};
		Ok(Self::V3(data.into()))
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
		Ok(Self::V2(data.into()))
	}

	fn v2_to_v1(self) -> Result<Self> {
		let Self::V2(data) = self else {
			bail!("expected client protocol v2 HttpResponseError")
		};
		Ok(Self::V1(data.into()))
	}
}
