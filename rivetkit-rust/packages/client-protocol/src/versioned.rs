use anyhow::{Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::{v1, v2, v3, v4, v5};

pub enum ToClient {
	V1(v1::ToClient),
	V2(v2::ToClient),
	V3(v3::ToClient),
	V4(v4::ToClient),
	V5(v5::ToClient),
}

impl OwnedVersionedData for ToClient {
	type Latest = v5::ToClient;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V5(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V5(data) => Ok(data),
			_ => bail!("version not latest"),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
			5 => Ok(Self::V5(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid client protocol version: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match (self, version) {
			(Self::V1(data), 1) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V2(data), 2) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V3(data), 3) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V5(data), 5) => serde_bare::to_vec(&data).map_err(Into::into),
			(_, version) => bail!("unexpected client protocol version: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2, Self::v2_to_v3, Self::v3_to_v4, Self::v4_to_v5]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v5_to_v4, Self::v4_to_v3, Self::v3_to_v2, Self::v2_to_v1]
	}
}

impl ToClient {
	fn v4_to_v5(self) -> Result<Self> {
		let Self::V4(data) = self else {
			bail!("expected client protocol v4 ToClient")
		};
		let body = match data.body {
			v4::ToClientBody::Init(value) => v5::ToClientBody::Init(v5::Init {
				actor_id: value.actor_id,
				connection_id: value.connection_id,
			}),
			v4::ToClientBody::Error(value) => v5::ToClientBody::Error(v5::Error {
				group: value.group,
				code: value.code,
				message: value.message,
				metadata: value.metadata,
				action_id: value.action_id,
				actor: value.actor.map(|actor| v5::ActorSpecifier {
					actor_id: actor.actor_id,
					generation: actor.generation,
					key: actor.key,
				}),
			}),
			v4::ToClientBody::ActionResponse(value) => v5::ToClientBody::ActionResponse(v5::ActionResponse { id: value.id, output: value.output }),
			v4::ToClientBody::Event(value) => v5::ToClientBody::Event(v5::Event { name: value.name, args: value.args }),
		};
		Ok(Self::V5(v5::ToClient { body }))
	}

	fn v5_to_v4(self) -> Result<Self> {
		let Self::V5(data) = self else {
			bail!("expected client protocol v5 ToClient")
		};
		let body = match data.body {
			v5::ToClientBody::Init(value) => v4::ToClientBody::Init(v4::Init { actor_id: value.actor_id, connection_id: value.connection_id }),
			v5::ToClientBody::Error(value) => v4::ToClientBody::Error(v4::Error {
				group: value.group,
				code: value.code,
				message: value.message,
				metadata: value.metadata,
				action_id: value.action_id,
				actor: value.actor.map(|actor| v4::ActorSpecifier { actor_id: actor.actor_id, generation: actor.generation, key: actor.key }),
			}),
			v5::ToClientBody::ActionResponse(value) => v4::ToClientBody::ActionResponse(v4::ActionResponse { id: value.id, output: value.output }),
			v5::ToClientBody::Event(value) => v4::ToClientBody::Event(v4::Event { name: value.name, args: value.args }),
		};
		Ok(Self::V4(v4::ToClient { body }))
	}
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
		impl_same_fields_pair!(
			$left,
			$right,
			SubscriptionRequest {
				event_name,
				subscribe,
			}
		);
		impl_to_server_pair!($left, $right);
		impl_same_fields_pair!($left, $right, HttpActionRequest { args });
		impl_same_fields_pair!($left, $right, HttpActionResponse { output });
		impl_same_fields_pair!($left, $right, HttpResolveResponse { actor_id });
	};
}

macro_rules! impl_to_client_v2_v3_pair {
	() => {
		impl_same_fields_pair!(
			v2,
			v3,
			Init {
				actor_id,
				connection_id,
			}
		);
		impl_same_fields_pair!(
			v2,
			v3,
			Error {
				group,
				code,
				message,
				metadata,
				action_id,
			}
		);
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
impl_common_pair!(v4, v5);
impl_to_client_v2_v3_pair!();
impl_same_fields_pair!(
	v1,
	v2,
	HttpResponseError {
		group,
		code,
		message,
		metadata,
	}
);
impl_same_fields_pair!(
	v2,
	v3,
	HttpResponseError {
		group,
		code,
		message,
		metadata,
	}
);
impl_same_fields_pair!(
	v3,
	v4,
	HttpQueueSendRequest {
		body,
		name,
		wait,
		timeout,
	}
);
impl_same_fields_pair!(v3, v4, HttpQueueSendResponse { status, response });

macro_rules! impl_versioned_manual {
	($name:ident, $latest_ty:path, $v1_ty:path, $v2_ty:path, $v3_ty:path, $v4_ty:path, $v5_ty:path) => {
		pub enum $name {
			V1($v1_ty),
			V2($v2_ty),
			V3($v3_ty),
			V4($v4_ty),
			V5($v5_ty),
		}

		impl OwnedVersionedData for $name {
			type Latest = $latest_ty;

			fn wrap_latest(latest: Self::Latest) -> Self {
				Self::V5(latest)
			}

			fn unwrap_latest(self) -> Result<Self::Latest> {
				match self {
					Self::V5(data) => Ok(data),
					_ => bail!("version not latest"),
				}
			}

			fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
				match version {
					1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
					2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
					3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
					4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
					5 => Ok(Self::V5(serde_bare::from_slice(payload)?)),
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
					(Self::V5(data), 5) => serde_bare::to_vec(&data).map_err(Into::into),
					(_, version) => bail!(
						"unexpected client protocol version for {}: {version}",
						stringify!($name)
					),
				}
			}

			fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
				vec![Self::v1_to_v2, Self::v2_to_v3, Self::v3_to_v4, Self::v4_to_v5]
			}

			fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
				vec![Self::v5_to_v4, Self::v4_to_v3, Self::v3_to_v2, Self::v2_to_v1]
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

			fn v4_to_v5(self) -> Result<Self> {
				let Self::V4(data) = self else {
					bail!("expected client protocol v4 {}", stringify!($name))
				};
				Ok(Self::V5(data.into()))
			}

			fn v5_to_v4(self) -> Result<Self> {
				let Self::V5(data) = self else {
					bail!("expected client protocol v5 {}", stringify!($name))
				};
				Ok(Self::V4(data.into()))
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

impl_versioned_manual!(
	ToServer,
	v5::ToServer,
	v1::ToServer,
	v2::ToServer,
	v3::ToServer,
	v4::ToServer,
	v5::ToServer
);
impl_versioned_manual!(
	HttpActionRequest,
	v5::HttpActionRequest,
	v1::HttpActionRequest,
	v2::HttpActionRequest,
	v3::HttpActionRequest,
	v4::HttpActionRequest,
	v5::HttpActionRequest
);
impl_versioned_manual!(
	HttpActionResponse,
	v5::HttpActionResponse,
	v1::HttpActionResponse,
	v2::HttpActionResponse,
	v3::HttpActionResponse,
	v4::HttpActionResponse,
	v5::HttpActionResponse
);
impl_versioned_manual!(
	HttpResolveResponse,
	v5::HttpResolveResponse,
	v1::HttpResolveResponse,
	v2::HttpResolveResponse,
	v3::HttpResolveResponse,
	v4::HttpResolveResponse,
	v5::HttpResolveResponse
);
pub enum HttpQueueSendRequest {
	V3(v3::HttpQueueSendRequest),
	V4(v4::HttpQueueSendRequest),
	V5(v5::HttpQueueSendRequest),
}

impl OwnedVersionedData for HttpQueueSendRequest {
	type Latest = v5::HttpQueueSendRequest;
	fn wrap_latest(latest: Self::Latest) -> Self { Self::V5(latest) }
	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self { Self::V5(data) => Ok(data), _ => bail!("version not latest") }
	}
	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
			5 => Ok(Self::V5(serde_bare::from_slice(payload)?)),
			_ => bail!("HttpQueueSendRequest only exists in client protocol v3+, got {version}"),
		}
	}
	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match (self, version) {
			(Self::V3(data), 3) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V5(data), 5) => serde_bare::to_vec(&data).map_err(Into::into),
			(_, version) => bail!("unexpected HttpQueueSendRequest version {version}"),
		}
	}
	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok, Self::v3_to_v4, Self::v4_to_v5]
	}
	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v5_to_v4, Self::v4_to_v3, Ok, Ok]
	}
}

impl HttpQueueSendRequest {
	fn v3_to_v4(self) -> Result<Self> {
		let Self::V3(data) = self else { bail!("expected v3 queue send request") };
		Ok(Self::V4(data.into()))
	}
	fn v4_to_v5(self) -> Result<Self> {
		let Self::V4(data) = self else { bail!("expected v4 queue send request") };
		if data.wait.unwrap_or(false) {
			bail!("queue.wait_unsupported");
		}
		Ok(Self::V5(v5::HttpQueueSendRequest {
			body: data.body,
			name: data.name,
			dedupe_key: None,
			delay: None,
		}))
	}
	fn v5_to_v4(self) -> Result<Self> {
		let Self::V5(data) = self else { bail!("expected v5 queue send request") };
		if data.dedupe_key.is_some() || data.delay.is_some() {
			bail!("queue send dedupeKey/delay require protocol v5");
		}
		Ok(Self::V4(v4::HttpQueueSendRequest {
			body: data.body,
			name: data.name,
			wait: Some(false),
			timeout: None,
		}))
	}
	fn v4_to_v3(self) -> Result<Self> {
		let Self::V4(data) = self else { bail!("expected v4 queue send request") };
		Ok(Self::V3(data.into()))
	}
}

pub enum HttpQueueSendResponse {
	V3(v3::HttpQueueSendResponse),
	V4(v4::HttpQueueSendResponse),
	V5(v5::HttpQueueSendResponse),
}

impl OwnedVersionedData for HttpQueueSendResponse {
	type Latest = v5::HttpQueueSendResponse;
	fn wrap_latest(latest: Self::Latest) -> Self { Self::V5(latest) }
	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self { Self::V5(data) => Ok(data), _ => bail!("version not latest") }
	}
	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
			5 => Ok(Self::V5(serde_bare::from_slice(payload)?)),
			_ => bail!("HttpQueueSendResponse only exists in client protocol v3+, got {version}"),
		}
	}
	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match (self, version) {
			(Self::V3(data), 3) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V5(data), 5) => serde_bare::to_vec(&data).map_err(Into::into),
			(_, version) => bail!("unexpected HttpQueueSendResponse version {version}"),
		}
	}
	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Ok, Ok, Self::v3_to_v4, Self::v4_to_v5]
	}
	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v5_to_v4, Self::v4_to_v3, Ok, Ok]
	}
}

impl HttpQueueSendResponse {
	fn v3_to_v4(self) -> Result<Self> {
		let Self::V3(data) = self else { bail!("expected v3 queue send response") };
		Ok(Self::V4(data.into()))
	}
	fn v4_to_v5(self) -> Result<Self> {
		let Self::V4(_data) = self else { bail!("expected v4 queue send response") };
		Ok(Self::V5(v5::HttpQueueSendResponse { receipt_id: String::new(), deduplicated: false }))
	}
	fn v5_to_v4(self) -> Result<Self> {
		let Self::V5(_data) = self else { bail!("expected v5 queue send response") };
		Ok(Self::V4(v4::HttpQueueSendResponse { status: "completed".to_owned(), response: None }))
	}
	fn v4_to_v3(self) -> Result<Self> {
		let Self::V4(data) = self else { bail!("expected v4 queue send response") };
		Ok(Self::V3(data.into()))
	}
}

macro_rules! impl_versioned_v5_only {
	($name:ident) => {
		pub enum $name { V5(v5::$name) }
		impl OwnedVersionedData for $name {
			type Latest = v5::$name;
			fn wrap_latest(latest: Self::Latest) -> Self { Self::V5(latest) }
			fn unwrap_latest(self) -> Result<Self::Latest> { let Self::V5(data) = self; Ok(data) }
			fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
				if version != 5 { bail!("{} only exists in client protocol v5, got {version}", stringify!($name)); }
				Ok(Self::V5(serde_bare::from_slice(payload)?))
			}
			fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
				if version != 5 { bail!("{} only exists in client protocol v5, got {version}", stringify!($name)); }
				let Self::V5(data) = self;
				serde_bare::to_vec(&data).map_err(Into::into)
			}
			fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> { vec![Ok, Ok, Ok, Ok] }
			fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> { vec![Ok, Ok, Ok, Ok] }
		}
	};
}

impl_versioned_v5_only!(HttpQueueStatusRequest);
impl_versioned_v5_only!(HttpQueueStatusResponse);

#[cfg(test)]
mod queue_tests {
	use super::*;

	#[test]
	fn v4_waiting_queue_send_is_rejected() {
		let result = HttpQueueSendRequest::V4(v4::HttpQueueSendRequest {
			body: vec![1, 2, 3],
			name: Some("jobs".to_owned()),
			wait: Some(true),
			timeout: Some(1_000),
		})
		.v4_to_v5();
		let Err(error) = result else {
			panic!("wait=true must not silently become fire-and-forget");
		};
		assert!(format!("{error:#}").contains("queue.wait_unsupported"));
	}

	#[test]
	fn v4_non_waiting_queue_send_upgrades_without_completion_fields() {
		let upgraded = HttpQueueSendRequest::V4(v4::HttpQueueSendRequest {
			body: vec![1, 2, 3],
			name: Some("jobs".to_owned()),
			wait: Some(false),
			timeout: Some(1_000),
		})
		.v4_to_v5()
		.expect("wait=false remains compatible");
		let HttpQueueSendRequest::V5(upgraded) = upgraded else {
			panic!("expected v5 request");
		};
		assert_eq!(upgraded.name.as_deref(), Some("jobs"));
		assert_eq!(upgraded.body, vec![1, 2, 3]);
		assert_eq!(upgraded.dedupe_key, None);
		assert_eq!(upgraded.delay, None);
	}

	#[test]
	fn v5_queue_receipt_downgrades_to_accepted_legacy_response() {
		let downgraded = HttpQueueSendResponse::V5(v5::HttpQueueSendResponse {
			receipt_id: "opaque".to_owned(),
			deduplicated: true,
		})
		.v5_to_v4()
		.expect("legacy clients receive their historical accepted response");
		let HttpQueueSendResponse::V4(downgraded) = downgraded else {
			panic!("expected v4 response");
		};
		assert_eq!(downgraded.status, "completed");
		assert_eq!(downgraded.response, None);
	}
}

pub enum HttpResponseError {
	V1(v1::HttpResponseError),
	V2(v2::HttpResponseError),
	V3(v3::HttpResponseError),
	V4(v4::HttpResponseError),
	V5(v5::HttpResponseError),
}

impl OwnedVersionedData for HttpResponseError {
	type Latest = v5::HttpResponseError;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V5(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V5(data) => Ok(data),
			_ => bail!("version not latest"),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
			5 => Ok(Self::V5(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid client protocol version for HttpResponseError: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match (self, version) {
			(Self::V1(data), 1) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V2(data), 2) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V3(data), 3) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V5(data), 5) => serde_bare::to_vec(&data).map_err(Into::into),
			(_, version) => {
				bail!("unexpected client protocol version for HttpResponseError: {version}")
			}
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2, Self::v2_to_v3, Self::v3_to_v4, Self::v4_to_v5]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v5_to_v4, Self::v4_to_v3, Self::v3_to_v2, Self::v2_to_v1]
	}
}

impl HttpResponseError {
	fn v4_to_v5(self) -> Result<Self> {
		let Self::V4(data) = self else { bail!("expected client protocol v4 HttpResponseError") };
		Ok(Self::V5(v5::HttpResponseError {
			group: data.group,
			code: data.code,
			message: data.message,
			metadata: data.metadata,
			actor: data.actor.map(|actor| v5::ActorSpecifier { actor_id: actor.actor_id, generation: actor.generation, key: actor.key }),
		}))
	}

	fn v5_to_v4(self) -> Result<Self> {
		let Self::V5(data) = self else { bail!("expected client protocol v5 HttpResponseError") };
		Ok(Self::V4(v4::HttpResponseError {
			group: data.group,
			code: data.code,
			message: data.message,
			metadata: data.metadata,
			actor: data.actor.map(|actor| v4::ActorSpecifier { actor_id: actor.actor_id, generation: actor.generation, key: actor.key }),
		}))
	}
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
