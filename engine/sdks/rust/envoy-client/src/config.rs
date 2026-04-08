use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use rivet_envoy_protocol as protocol;
use tokio::sync::mpsc;

use crate::handle::EnvoyHandle;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// HTTP request/response types used by the envoy client.
pub struct HttpRequest {
	pub method: String,
	pub path: String,
	pub headers: HashMap<String, String>,
	pub body: Option<Vec<u8>>,
	/// If the request is streamed, body chunks arrive on this channel.
	pub body_stream: Option<mpsc::UnboundedReceiver<Vec<u8>>>,
}

pub struct HttpResponse {
	pub status: u16,
	pub headers: HashMap<String, String>,
	pub body: Option<Vec<u8>>,
	/// If set, the response is streamed. The envoy client reads chunks and sends
	/// `ToRivetResponseChunk` for each one.
	pub body_stream: Option<mpsc::UnboundedReceiver<ResponseChunk>>,
}

/// A chunk in a streaming HTTP response.
pub struct ResponseChunk {
	pub data: Vec<u8>,
	pub finish: bool,
}

pub struct EnvoyConfig {
	pub version: u32,
	pub endpoint: String,
	pub token: Option<String>,
	pub namespace: String,
	pub pool_name: String,
	pub prepopulate_actor_names: HashMap<String, PrepopulatedActor>,
	pub metadata: Option<HashMap<String, String>>,
	/// When `start_envoy` is called, create a new envoy every time instead of using a single global envoy
	/// instance for the entire runtime.
	pub not_global: bool,

	/// Debug option to inject artificial latency (in ms) into WebSocket communication.
	pub debug_latency_ms: Option<u64>,

	pub callbacks: Arc<dyn EnvoyCallbacks>,
}

pub struct PrepopulatedActor {
	pub metadata: String,
}

/// Callbacks that the consumer of the envoy client must implement.
pub trait EnvoyCallbacks: Send + Sync + 'static {
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
		handle: EnvoyHandle,
		actor_id: String,
		generation: u32,
		reason: protocol::StopActorReason,
	) -> BoxFuture<anyhow::Result<()>>;

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
	) -> bool;
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

	pub fn close(&self, code: Option<u16>, reason: Option<String>) {
		let _ = self.tx.send(WsOutgoing::Close { code, reason });
	}
}
