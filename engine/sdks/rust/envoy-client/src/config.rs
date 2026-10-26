use std::{collections::HashMap, sync::Arc, time::Duration};

// Retain the established `config::*` API while keeping implementations in their domain modules.
pub use crate::{
	callbacks::{ActorStopHandle, BoxFuture, EnvoyCallbacks},
	http::{
		HTTP_BODY_MAX_CHUNK_SIZE, HTTP_BODY_STREAM_CHANNEL_CAPACITY, HttpRequest,
		HttpRequestBodyError, HttpRequestBodyQueue, HttpRequestBodyStream, HttpResponse,
		HttpResponseBodyStream, ResponseChunk,
	},
	websocket::{WebSocketHandler, WebSocketMessage, WebSocketSender},
};

/// Default for `EnvoyConfig::stopped_generation_floor_capacity`.
pub const DEFAULT_STOPPED_GENERATION_FLOOR_CAPACITY: usize = 65_536;

/// Default for `EnvoyConfig::stopped_generation_floor_ttl`. Well above the engine's
/// default command attempt deadline plus its Envoy lost threshold, so it also covers a
/// raised command deadline.
pub const DEFAULT_STOPPED_GENERATION_FLOOR_TTL: Duration = Duration::from_secs(120);

/// Default for `EnvoyConfig::max_queued_requests_per_actor`.
pub const DEFAULT_MAX_QUEUED_REQUESTS_PER_ACTOR: usize = 1024;

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

	/// How long an actor's highest stopped generation is remembered, so a late duplicate
	/// start for a stopped generation is dropped. Must exceed the engine's command attempt
	/// deadline plus its Envoy lost threshold.
	pub stopped_generation_floor_ttl: Duration,

	/// Most actors whose stopped generation floor is remembered at once. A backstop for
	/// memory; the least recently stopped actor is forgotten first at the cap.
	pub stopped_generation_floor_capacity: usize,

	/// Most HTTP requests admitted to one actor that its task has not taken from its inbox yet.
	/// Requests wait there while the actor starts. A request past the cap is refused with
	/// `envoy.actor_overloaded` instead of growing the inbox.
	pub max_queued_requests_per_actor: usize,

	pub callbacks: Arc<dyn EnvoyCallbacks>,

	/// Opens the byte stream the WebSocket runs over, for hosts with their own transport such
	/// as a simulated network. `None` connects to `endpoint` over TCP, with TLS for `wss`.
	pub connector: Option<Arc<dyn EnvoyConnector>>,
}

/// The byte stream a connector returns. Only native builds can supply one.
#[cfg(not(target_arch = "wasm32"))]
pub type EnvoyStream = Box<dyn EnvoyIo>;

#[cfg(target_arch = "wasm32")]
pub type EnvoyStream = std::convert::Infallible;

#[cfg(not(target_arch = "wasm32"))]
pub trait EnvoyIo: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}

#[cfg(not(target_arch = "wasm32"))]
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send> EnvoyIo for T {}

/// Opens the stream for the WebSocket at `url` (a `ws://` or `wss://` URL with the Envoy's
/// connect query). The handshake then runs over it unchanged.
pub trait EnvoyConnector: Send + Sync {
	fn connect(&self, url: &str) -> BoxFuture<anyhow::Result<EnvoyStream>>;
}

pub struct ActorName {
	pub metadata: serde_json::Value,
}
