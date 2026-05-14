use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicBool, AtomicI64};

use crate::async_counter::AsyncCounter;
use rivet_envoy_protocol as protocol;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::sync::mpsc;
use tokio::sync::watch;

use crate::actor::ToActor;
use crate::config::EnvoyConfig;
use crate::envoy::ToEnvoyMessage;
use crate::tunnel::HibernatingWebSocketMetadata;

pub struct SharedActorEntry {
	pub handle: mpsc::UnboundedSender<ToActor>,
	pub active_http_request_count: Arc<AsyncCounter>,
}

pub struct SharedContext {
	pub config: EnvoyConfig,
	pub envoy_key: String,
	pub envoy_tx: mpsc::UnboundedSender<ToEnvoyMessage>,
	pub actors: Arc<StdMutex<HashMap<String, HashMap<u32, SharedActorEntry>>>>,
	pub actors_notify: Arc<Notify>,
	pub live_tunnel_requests: Arc<StdMutex<HashMap<[u8; 8], String>>>,
	pub pending_hibernation_restores:
		Arc<StdMutex<HashMap<String, Vec<HibernatingWebSocketMetadata>>>>,
	pub ws_tx: Arc<Mutex<Option<mpsc::UnboundedSender<WsTxMessage>>>>,
	pub protocol_metadata: Arc<Mutex<Option<protocol::ProtocolMetadata>>>,
	pub shutting_down: AtomicBool,
	/// Epoch ms timestamp of the most recent ping packet received from the engine. Used by
	/// `EnvoyHandle::is_ping_healthy` to surface a dead engine link to upstream health checks.
	/// Initialized to the construction time so a freshly created envoy reports healthy until
	/// its first ping arrives or the threshold elapses without one.
	pub last_ping_ts: AtomicI64,
	/// Epoch ms timestamp of when the most recent pong was actually written onto the WS by the
	/// write task. Used to detect ws_tx backpressure on close.
	pub last_pong_sent_ts: AtomicI64,
	/// Current depth of the ws_tx mpsc channel. Bumped on every `ws_send` enqueue and decremented
	/// when the write task dequeues. Used to expose backpressure at the moment of WS close.
	pub ws_tx_depth: AtomicI64,
	// Latched signal fired by `envoy_loop` after its cleanup block completes.
	// Waiters observing `true` are guaranteed that the loop has exited and
	// every pending KV/SQLite request has been resolved (with `EnvoyShutdownError`
	// if it didn't complete naturally).
	pub stopped_tx: watch::Sender<bool>,
}

#[derive(Debug)]
pub enum WsTxMessage {
	Send {
		data: Vec<u8>,
		/// Epoch ms when this message was enqueued. Used by the write task to compute internal
		/// queue + write latency for diagnostic logs.
		enqueue_ts: i64,
		/// True if this message is a `ToRivetPong`. Pong-specific latency drives the engine ping
		/// timeout detection, so we log its end-to-end timing separately.
		is_pong: bool,
		message_kind: &'static str,
		gateway_id: Option<protocol::GatewayId>,
		request_id: Option<protocol::RequestId>,
		message_index: Option<u16>,
		inner_data_len: usize,
	},
	Close,
}
