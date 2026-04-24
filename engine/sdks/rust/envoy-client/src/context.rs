use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::AtomicBool;

use rivet_envoy_protocol as protocol;
use rivet_util::async_counter::AsyncCounter;
use tokio::sync::Mutex;
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
	pub live_tunnel_requests: Arc<StdMutex<HashMap<[u8; 8], String>>>,
	pub pending_hibernation_restores:
		Arc<StdMutex<HashMap<String, Vec<HibernatingWebSocketMetadata>>>>,
	pub ws_tx: Arc<Mutex<Option<mpsc::UnboundedSender<WsTxMessage>>>>,
	pub protocol_metadata: Arc<Mutex<Option<protocol::ProtocolMetadata>>>,
	pub shutting_down: AtomicBool,
	// Latched signal fired by `envoy_loop` after its cleanup block completes.
	// Waiters observing `true` are guaranteed that the loop has exited and
	// every pending KV/SQLite request has been resolved (with `EnvoyShutdownError`
	// if it didn't complete naturally).
	pub stopped_tx: watch::Sender<bool>,
}

#[derive(Debug)]
pub enum WsTxMessage {
	Send(Vec<u8>),
	Close,
}
