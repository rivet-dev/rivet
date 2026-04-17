use std::collections::HashMap;
use std::fmt;
use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use futures::future::BoxFuture;

use crate::actor::connection::ConnHandle;
use crate::actor::context::ActorContext;
use crate::websocket::WebSocket;

#[derive(Clone, Debug)]
pub struct Request(http::Request<Vec<u8>>);

impl Request {
	pub fn new(body: Vec<u8>) -> Self {
		Self(http::Request::new(body))
	}

	pub fn from_parts(
		method: &str,
		uri: &str,
		headers: HashMap<String, String>,
		body: Vec<u8>,
	) -> Result<Self> {
		let method = method
			.parse::<http::Method>()
			.map_err(|error| anyhow!("invalid request method `{method}`: {error}"))?;
		let uri = uri
			.parse::<http::Uri>()
			.map_err(|error| anyhow!("invalid request uri `{uri}`: {error}"))?;
		let mut request = http::Request::builder()
			.method(method)
			.uri(uri)
			.body(body)?;

		for (name, value) in headers {
			let header_name: http::header::HeaderName = name
				.parse()
				.map_err(|error| anyhow!("invalid request header name `{name}`: {error}"))?;
			let header_value: http::header::HeaderValue = value
				.parse()
				.map_err(|error| anyhow!("invalid request header `{name}` value: {error}"))?;
			request.headers_mut().insert(header_name, header_value);
		}

		Ok(Self(request))
	}

	pub fn to_parts(&self) -> (String, String, HashMap<String, String>, Vec<u8>) {
		(
			self.method().to_string(),
			self.uri().to_string(),
			self
				.headers()
				.iter()
				.map(|(name, value)| {
					(
						name.to_string(),
						String::from_utf8_lossy(value.as_bytes()).into_owned(),
					)
				})
				.collect(),
			self.body().clone(),
		)
	}

	pub fn into_inner(self) -> http::Request<Vec<u8>> {
		self.0
	}

	pub fn into_body(self) -> Vec<u8> {
		self.0.into_body()
	}
}

impl Default for Request {
	fn default() -> Self {
		Self::new(Vec::new())
	}
}

impl Deref for Request {
	type Target = http::Request<Vec<u8>>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for Request {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl From<http::Request<Vec<u8>>> for Request {
	fn from(value: http::Request<Vec<u8>>) -> Self {
		Self(value)
	}
}

impl From<Request> for http::Request<Vec<u8>> {
	fn from(value: Request) -> Self {
		value.0
	}
}

#[derive(Clone, Debug)]
pub struct Response(http::Response<Vec<u8>>);

impl Response {
	pub fn new(body: Vec<u8>) -> Self {
		Self(http::Response::new(body))
	}

	pub fn from_parts(
		status: u16,
		headers: HashMap<String, String>,
		body: Vec<u8>,
	) -> Result<Self> {
		let mut response = http::Response::new(body);
		*response.status_mut() = status
			.try_into()
			.map_err(|error| anyhow!("invalid http response status `{status}`: {error}"))?;

		for (name, value) in headers {
			let header_name: http::header::HeaderName = name
				.parse()
				.map_err(|error| anyhow!("invalid response header name `{name}`: {error}"))?;
			let header_value: http::header::HeaderValue = value
				.parse()
				.map_err(|error| anyhow!("invalid response header `{name}` value: {error}"))?;
			response.headers_mut().insert(header_name, header_value);
		}

		Ok(Self(response))
	}

	pub fn to_parts(&self) -> (u16, HashMap<String, String>, Vec<u8>) {
		(
			self.status().as_u16(),
			self
				.headers()
				.iter()
				.map(|(name, value)| {
					(
						name.to_string(),
						String::from_utf8_lossy(value.as_bytes()).into_owned(),
					)
				})
				.collect(),
			self.body().clone(),
		)
	}

	pub fn into_inner(self) -> http::Response<Vec<u8>> {
		self.0
	}

	pub fn into_body(self) -> Vec<u8> {
		self.0.into_body()
	}
}

impl Default for Response {
	fn default() -> Self {
		Self::new(Vec::new())
	}
}

impl Deref for Response {
	type Target = http::Response<Vec<u8>>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for Response {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl From<http::Response<Vec<u8>>> for Response {
	fn from(value: http::Response<Vec<u8>>) -> Self {
		Self(value)
	}
}

impl From<Response> for http::Response<Vec<u8>> {
	fn from(value: Response) -> Self {
		value.0
	}
}

pub type LifecycleCallback<T> =
	Box<dyn Fn(T) -> BoxFuture<'static, Result<()>> + Send + Sync>;
pub type RequestCallback =
	Box<dyn Fn(OnRequestRequest) -> BoxFuture<'static, Result<Response>> + Send + Sync>;
pub type ActionHandler =
	Box<dyn Fn(ActionRequest) -> BoxFuture<'static, Result<Vec<u8>>> + Send + Sync>;
pub type BeforeActionResponseCallback = Box<
	dyn Fn(OnBeforeActionResponseRequest) -> BoxFuture<'static, Result<Vec<u8>>> + Send + Sync,
>;

#[derive(Clone, Debug)]
pub struct OnWakeRequest {
	pub ctx: ActorContext,
}

#[derive(Clone, Debug)]
pub struct OnSleepRequest {
	pub ctx: ActorContext,
}

#[derive(Clone, Debug)]
pub struct OnDestroyRequest {
	pub ctx: ActorContext,
}

#[derive(Clone, Debug)]
pub struct OnStateChangeRequest {
	pub ctx: ActorContext,
	pub new_state: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct OnRequestRequest {
	pub ctx: ActorContext,
	pub request: Request,
}

#[derive(Clone, Debug)]
pub struct OnWebSocketRequest {
	pub ctx: ActorContext,
	pub ws: WebSocket,
}

#[derive(Clone, Debug)]
pub struct OnBeforeConnectRequest {
	pub ctx: ActorContext,
	pub params: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct OnConnectRequest {
	pub ctx: ActorContext,
	pub conn: ConnHandle,
}

#[derive(Clone, Debug)]
pub struct OnDisconnectRequest {
	pub ctx: ActorContext,
	pub conn: ConnHandle,
}

#[derive(Clone, Debug)]
pub struct ActionRequest {
	pub ctx: ActorContext,
	pub conn: ConnHandle,
	pub name: String,
	pub args: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct OnBeforeActionResponseRequest {
	pub ctx: ActorContext,
	pub name: String,
	pub args: Vec<u8>,
	pub output: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct RunRequest {
	pub ctx: ActorContext,
}

#[derive(Default)]
pub struct ActorInstanceCallbacks {
	pub on_wake: Option<LifecycleCallback<OnWakeRequest>>,
	pub on_sleep: Option<LifecycleCallback<OnSleepRequest>>,
	pub on_destroy: Option<LifecycleCallback<OnDestroyRequest>>,
	pub on_state_change: Option<LifecycleCallback<OnStateChangeRequest>>,
	pub on_request: Option<RequestCallback>,
	pub on_websocket: Option<LifecycleCallback<OnWebSocketRequest>>,
	pub on_before_connect: Option<LifecycleCallback<OnBeforeConnectRequest>>,
	pub on_connect: Option<LifecycleCallback<OnConnectRequest>>,
	pub on_disconnect: Option<LifecycleCallback<OnDisconnectRequest>>,
	pub actions: HashMap<String, ActionHandler>,
	pub on_before_action_response: Option<BeforeActionResponseCallback>,
	pub run: Option<LifecycleCallback<RunRequest>>,
}

impl fmt::Debug for ActorInstanceCallbacks {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ActorInstanceCallbacks")
			.field("on_wake", &self.on_wake.is_some())
			.field("on_sleep", &self.on_sleep.is_some())
			.field("on_destroy", &self.on_destroy.is_some())
			.field("on_state_change", &self.on_state_change.is_some())
			.field("on_request", &self.on_request.is_some())
			.field("on_websocket", &self.on_websocket.is_some())
			.field("on_before_connect", &self.on_before_connect.is_some())
			.field("on_connect", &self.on_connect.is_some())
			.field("on_disconnect", &self.on_disconnect.is_some())
			.field("actions", &self.actions.keys().collect::<Vec<_>>())
			.field(
				"on_before_action_response",
				&self.on_before_action_response.is_some(),
			)
			.field("run", &self.run.is_some())
			.finish()
	}
}

#[cfg(test)]
#[path = "../../tests/modules/callbacks.rs"]
mod tests;
