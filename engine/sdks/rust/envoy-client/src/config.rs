use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

use rivet_envoy_protocol as protocol;
use tokio::sync::{mpsc, oneshot, watch};

use crate::handle::EnvoyHandle;

pub const HTTP_BODY_STREAM_CHANNEL_CAPACITY: usize = 16;
pub const HTTP_BODY_MAX_CHUNK_SIZE: usize = 64 * 1024;

#[cfg(not(target_arch = "wasm32"))]
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

#[cfg(target_arch = "wasm32")]
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

#[derive(Clone, Debug)]
pub struct HttpRequestBodyError {
	pub reason: protocol::HttpStreamAbortReason,
}

impl std::fmt::Display for HttpRequestBodyError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self.reason.detail {
			Some(detail) => write!(f, "{:?}: {detail}", self.reason.kind),
			None => write!(f, "{:?}", self.reason.kind),
		}
	}
}

impl std::error::Error for HttpRequestBodyError {}

#[derive(Debug)]
pub struct HttpRequestBodyStream {
	rx: mpsc::Receiver<Vec<u8>>,
	abort_rx: watch::Receiver<Option<HttpRequestBodyError>>,
}

impl HttpRequestBodyStream {
	pub fn new(
		rx: mpsc::Receiver<Vec<u8>>,
		abort_rx: watch::Receiver<Option<HttpRequestBodyError>>,
	) -> Self {
		Self { rx, abort_rx }
	}

	pub async fn recv(&mut self) -> Result<Option<Vec<u8>>, HttpRequestBodyError> {
		loop {
			if let Some(error) = self.abort_rx.borrow().clone() {
				return Err(error);
			}

			tokio::select! {
				biased;
				changed = self.abort_rx.changed() => {
					if changed.is_ok() {
						continue;
					}
					return Ok(self.rx.recv().await);
				}
				chunk = self.rx.recv() => return Ok(chunk),
			}
		}
	}
}

/// HTTP request/response types used by the envoy client.
pub struct HttpRequest {
	pub method: String,
	pub path: String,
	pub headers: HashMap<String, String>,
	pub body: Option<Vec<u8>>,
	/// If the request is streamed, body chunks arrive on this channel.
	pub body_stream: Option<HttpRequestBodyStream>,
}

pub struct HttpResponse {
	pub status: u16,
	pub headers: HashMap<String, String>,
	pub body: Option<Vec<u8>>,
	/// If set, the response is streamed. The envoy client reads chunks and sends
	/// `ToRivetResponseChunk` for each one.
	pub body_stream: Option<HttpResponseBodyStream>,
}

/// A chunk in a streaming HTTP response.
pub enum ResponseChunk {
	Data { data: Vec<u8>, finish: bool },
	Error(String),
}

pub struct HttpResponseBodyStream {
	rx: mpsc::Receiver<ResponseChunk>,
	on_drop: Option<Box<dyn FnOnce() + Send>>,
}

impl HttpResponseBodyStream {
	pub fn set_on_drop(&mut self, on_drop: impl FnOnce() + Send + 'static) {
		self.on_drop = Some(Box::new(on_drop));
	}

	pub async fn recv(&mut self) -> Option<ResponseChunk> {
		self.rx.recv().await
	}
}

impl From<mpsc::Receiver<ResponseChunk>> for HttpResponseBodyStream {
	fn from(rx: mpsc::Receiver<ResponseChunk>) -> Self {
		Self { rx, on_drop: None }
	}
}

impl Drop for HttpResponseBodyStream {
	fn drop(&mut self) {
		if let Some(on_drop) = self.on_drop.take() {
			on_drop();
		}
	}
}

pub struct EnvoyConfig {
	pub version: u32,
	pub endpoint: String,
	pub token: Option<String>,
	pub namespace: String,
	pub pool_name: String,
	pub prepopulate_actor_names: HashMap<String, ActorName>,
	pub metadata: Option<serde_json::Value>,
	/// When `start_envoy` is called, create a new envoy every time instead of using a single global envoy
	/// instance for the entire runtime.
	pub not_global: bool,

	/// Debug option to inject artificial latency (in ms) into WebSocket communication.
	pub debug_latency_ms: Option<u64>,

	pub callbacks: Arc<dyn EnvoyCallbacks>,
}

pub struct ActorName {
	pub metadata: serde_json::Value,
}

/// One-shot completion handle used to defer the final stopped event until teardown is done.
#[derive(Clone)]
pub struct ActorStopHandle {
	tx: Arc<Mutex<Option<oneshot::Sender<anyhow::Result<()>>>>>,
}

impl ActorStopHandle {
	pub(crate) fn new(tx: oneshot::Sender<anyhow::Result<()>>) -> Self {
		Self {
			tx: Arc::new(Mutex::new(Some(tx))),
		}
	}

	pub fn complete(self) -> bool {
		self.finish(Ok(()))
	}

	pub fn fail(self, error: anyhow::Error) -> bool {
		self.finish(Err(error))
	}

	pub fn finish(self, result: anyhow::Result<()>) -> bool {
		let mut guard = match self.tx.lock() {
			Ok(guard) => guard,
			Err(poisoned) => poisoned.into_inner(),
		};

		let Some(tx) = guard.take() else {
			return false;
		};

		tx.send(result).is_ok()
	}
}

/// Callbacks that the consumer of the envoy client must implement.
pub trait EnvoyCallbacks: Send + Sync + 'static {
	fn on_connect(&self, _handle: EnvoyHandle) {}

	fn on_disconnect(&self, _handle: EnvoyHandle) {}

	fn on_actor_start(
		&self,
		handle: EnvoyHandle,
		actor_id: String,
		generation: u32,
		config: protocol::ActorConfig,
		preloaded_kv: Option<protocol::PreloadedKv>,
	) -> BoxFuture<anyhow::Result<()>>;

	fn on_actor_stop(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_generation: u32,
		_reason: protocol::StopActorReason,
	) -> BoxFuture<anyhow::Result<()>> {
		Box::pin(async { Ok(()) })
	}

	fn on_actor_stop_with_completion(
		&self,
		handle: EnvoyHandle,
		actor_id: String,
		generation: u32,
		reason: protocol::StopActorReason,
		stop_handle: ActorStopHandle,
	) -> BoxFuture<anyhow::Result<()>> {
		let stop_future = self.on_actor_stop(handle, actor_id, generation, reason);

		Box::pin(async move {
			stop_future.await?;
			stop_handle.complete();
			Ok(())
		})
	}

	fn on_shutdown(&self);

	fn fetch(
		&self,
		handle: EnvoyHandle,
		actor_id: String,
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
		request: HttpRequest,
	) -> BoxFuture<anyhow::Result<HttpResponse>>;

	fn websocket(
		&self,
		handle: EnvoyHandle,
		actor_id: String,
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
		request: HttpRequest,
		path: String,
		headers: HashMap<String, String>,
		is_hibernatable: bool,
		is_restoring_hibernatable: bool,
		sender: WebSocketSender,
	) -> BoxFuture<anyhow::Result<WebSocketHandler>>;

	fn can_hibernate(
		&self,
		actor_id: &str,
		gateway_id: &protocol::GatewayId,
		request_id: &protocol::RequestId,
		request: &HttpRequest,
	) -> BoxFuture<anyhow::Result<bool>>;
}

/// Handler returned by the websocket callback for receiving WebSocket events.
pub struct WebSocketHandler {
	pub on_message: Box<dyn Fn(WebSocketMessage) -> BoxFuture<()> + Send + Sync>,
	pub on_close: Box<dyn Fn(u16, String) -> BoxFuture<()> + Send + Sync>,
	pub on_open: Option<Box<dyn FnOnce(WebSocketSender) -> BoxFuture<()> + Send>>,
}

pub struct WebSocketMessage {
	pub data: Vec<u8>,
	pub binary: bool,
	pub gateway_id: protocol::GatewayId,
	pub request_id: protocol::RequestId,
	pub message_index: u16,
	/// Send data back on this WebSocket connection.
	pub sender: WebSocketSender,
}

/// Allows sending messages back on a WebSocket connection from within the on_message callback.
#[derive(Clone)]
pub struct WebSocketSender {
	pub(crate) tx: tokio::sync::mpsc::UnboundedSender<WsOutgoing>,
}

pub(crate) enum WsOutgoing {
	Message {
		data: Vec<u8>,
		binary: bool,
	},
	Flush {
		tx: tokio::sync::oneshot::Sender<()>,
	},
	Close {
		code: Option<u16>,
		reason: Option<String>,
	},
}

impl WebSocketSender {
	pub fn send(&self, data: Vec<u8>, binary: bool) {
		let _ = self.tx.send(WsOutgoing::Message { data, binary });
	}

	pub fn send_text(&self, text: &str) {
		self.send(text.as_bytes().to_vec(), false);
	}

	pub async fn flush(&self) {
		let (tx, rx) = tokio::sync::oneshot::channel();
		if self.tx.send(WsOutgoing::Flush { tx }).is_ok() {
			let _ = rx.await;
		}
	}

	pub fn close(&self, code: Option<u16>, reason: Option<String>) {
		let _ = self.tx.send(WsOutgoing::Close { code, reason });
	}
}
