use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64};

use crate::async_counter::AsyncCounter;
use rivet_envoy_protocol as protocol;
use tokio::sync::{Mutex, Notify, Semaphore, mpsc, oneshot, watch};

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
	/// Registration key the engine knows this process by. Rotated by the envoy loop when the lost
	/// threshold discards every actor, so the engine treats the old registration as a dead envoy
	/// instead of believing those actors still run here. Read it through `envoy_key()`.
	pub envoy_key: parking_lot::RwLock<String>,
	pub envoy_tx: mpsc::UnboundedSender<ToEnvoyMessage>,
	pub actors: Arc<StdMutex<HashMap<String, HashMap<u32, SharedActorEntry>>>>,
	pub actors_notify: Arc<Notify>,
	pub live_tunnel_requests: Arc<StdMutex<HashMap<[u8; 8], String>>>,
	pub pending_hibernation_restores:
		Arc<StdMutex<HashMap<String, Vec<HibernatingWebSocketMetadata>>>>,
	pub ws_tx: Arc<Mutex<Option<mpsc::UnboundedSender<WsTxMessage>>>>,
	pub http_ws_tx: Arc<Mutex<Option<HttpConnectionTx>>>,
	/// The currently connected WebSocket session, or zero while disconnected.
	///
	/// Session IDs are actor-client-local and never cross the wire. Remote SQLite
	/// transactions use them to prevent a statement from being admitted on a
	/// replacement WebSocket after the server-side connection (and its SQLite
	/// handle) has already been torn down.
	pub connection_session: AtomicU64,
	pub next_connection_session: AtomicU64,
	pub connection_session_tx: watch::Sender<u64>,
	pub protocol_metadata: Arc<Mutex<Option<protocol::ProtocolMetadata>>>,
	pub shutting_down: AtomicBool,
	/// Epoch ms timestamp of the most recent ping packet received from the engine. Used by
	/// `EnvoyHandle::is_ping_healthy` to surface a dead engine link to upstream health checks.
	/// Zero means no ping has been received yet.
	pub last_ping_ts: AtomicI64,
	// Latched signal fired by `envoy_loop` after its cleanup block completes.
	// Waiters observing `true` are guaranteed that the loop has exited and
	// every pending KV/SQLite request has been resolved (with `EnvoyShutdownError`
	// if it didn't complete naturally).
	pub stopped_tx: watch::Sender<bool>,
}

impl SharedContext {
	/// Current registration key. Clone it rather than holding the guard; the envoy loop may rotate it.
	pub fn envoy_key(&self) -> String {
		self.envoy_key.read().clone()
	}

	/// Replaces the registration key with a fresh one and returns `(old, new)`. The next websocket
	/// connection registers under the new key; the engine expires the old key like a dead envoy.
	pub(crate) fn rotate_envoy_key(&self) -> (String, String) {
		let new_key = uuid::Uuid::new_v4().to_string();
		let mut guard = self.envoy_key.write();
		let old_key = std::mem::replace(&mut *guard, new_key.clone());
		(old_key, new_key)
	}
}

#[derive(Debug)]
pub enum WsTxMessage {
	Send(Vec<u8>),
	Close,
}

#[derive(Clone)]
pub struct HttpConnectionTx {
	pub session: u64,
	pub tx: mpsc::Sender<HttpWsTxMessage>,
	pub byte_budget: Arc<Semaphore>,
}

pub struct HttpWsTxMessage {
	pub data: Vec<u8>,
	pub _byte_permit: tokio::sync::OwnedSemaphorePermit,
	pub written: oneshot::Sender<anyhow::Result<()>>,
}
