use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::actor::connection::ConnHandle;
use crate::actor::context::ActorContext;
use crate::types::ConnId;
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

pub struct Reply<T> {
	tx: Option<oneshot::Sender<Result<T>>>,
}

impl<T> Reply<T> {
	pub fn send(mut self, result: Result<T>) {
		if let Some(tx) = self.tx.take() {
			let _ = tx.send(result);
		}
	}
}

impl<T> Drop for Reply<T> {
	fn drop(&mut self) {
		if let Some(tx) = self.tx.take() {
			let _ = tx.send(Err(crate::error::ActorLifecycle::DroppedReply.build()));
		}
	}
}

impl<T> std::fmt::Debug for Reply<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Reply")
			.field("pending", &self.tx.is_some())
			.finish()
	}
}

impl<T> From<oneshot::Sender<Result<T>>> for Reply<T> {
	fn from(tx: oneshot::Sender<Result<T>>) -> Self {
		Self { tx: Some(tx) }
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateDelta {
	ActorState(Vec<u8>),
	ConnHibernation {
		conn: ConnId,
		bytes: Vec<u8>,
	},
	ConnHibernationRemoved(ConnId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SerializeStateReason {
	Save,
	Inspector,
}

#[derive(Debug)]
pub enum ActorEvent {
	Action {
		name: String,
		args: Vec<u8>,
		conn: Option<ConnHandle>,
		reply: Reply<Vec<u8>>,
	},
	HttpRequest {
		request: Request,
		reply: Reply<Response>,
	},
	WebSocketOpen {
		ws: WebSocket,
		request: Option<Request>,
		reply: Reply<()>,
	},
	ConnectionOpen {
		conn: ConnHandle,
		params: Vec<u8>,
		request: Option<Request>,
		reply: Reply<()>,
	},
	ConnectionClosed {
		conn: ConnHandle,
	},
	SubscribeRequest {
		conn: ConnHandle,
		event_name: String,
		reply: Reply<()>,
	},
	SerializeState {
		reason: SerializeStateReason,
		reply: Reply<Vec<StateDelta>>,
	},
	BeginSleep,
	FinalizeSleep {
		reply: Reply<()>,
	},
	Destroy {
		reply: Reply<()>,
	},
	WorkflowHistoryRequested {
		reply: Reply<Option<Vec<u8>>>,
	},
	WorkflowReplayRequested {
		entry_id: Option<String>,
		reply: Reply<Option<Vec<u8>>>,
	},
}

pub struct ActorEvents(mpsc::Receiver<ActorEvent>);

impl ActorEvents {
	pub async fn recv(&mut self) -> Option<ActorEvent> {
		self.0.recv().await
	}

	pub fn try_recv(&mut self) -> Option<ActorEvent> {
		self.0.try_recv().ok()
	}
}

impl std::fmt::Debug for ActorEvents {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("ActorEvents(..)")
	}
}

impl From<mpsc::Receiver<ActorEvent>> for ActorEvents {
	fn from(value: mpsc::Receiver<ActorEvent>) -> Self {
		Self(value)
	}
}

#[derive(Debug)]
pub struct ActorStart {
	pub ctx: ActorContext,
	pub input: Option<Vec<u8>>,
	pub snapshot: Option<Vec<u8>>,
	pub hibernated: Vec<(ConnHandle, Vec<u8>)>,
	pub events: ActorEvents,
}

#[cfg(test)]
#[path = "../../tests/modules/callbacks.rs"]
mod tests;
