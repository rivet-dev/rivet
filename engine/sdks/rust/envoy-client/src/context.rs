use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64};

use crate::async_counter::AsyncCounter;
use crate::time::Instant;
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
	pub envoy_key: String,
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
	/// What this envoy can rely on about the engine's liveness clock. Drives the engine ping
	/// silence check. Only the connection task updates it.
	pub engine_liveness: EngineLiveness,
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

/// Tracks the earliest time the engine could have last refreshed this envoy's liveness timestamp.
///
/// The engine declares the envoy's actors lost once that timestamp is older than
/// `envoy_lost_threshold`. It refreshes the timestamp when it claims a connection, which happens
/// after it sends `ToEnvoyInit`, and before each ping. The engine sends commands and pings on a
/// connection only after claiming it, so either one proves the claim. Until then the engine may
/// still be counting from a ping on an earlier connection, for example when the claim fails.
/// Times are monotonic so wall clock jumps cannot trigger or delay the check.
#[derive(Default)]
pub struct EngineLiveness(parking_lot::Mutex<EngineLivenessState>);

#[derive(Default, Clone, Copy)]
struct EngineLivenessState {
	/// Latest time the engine is known to have refreshed its liveness timestamp, on this or an
	/// earlier connection: a ping, or the install time of a connection the engine claimed.
	last_refresh: Option<Instant>,
	/// When the client started opening the current connection.
	connection_installed_at: Option<Instant>,
	/// Whether the engine has claimed the current connection.
	connection_claimed: bool,
}

impl EngineLiveness {
	/// A new connection was installed. `connect_started_at` is when the client started opening it,
	/// which precedes any claim even when the engine claims it before the client finishes the
	/// handshake. The engine has not proven a claim yet, so the last refresh is kept until it does.
	pub fn connection_installed(&self, connect_started_at: Instant) {
		let mut state = self.0.lock();
		state.connection_installed_at = Some(connect_started_at);
		state.connection_claimed = false;
	}

	/// A ping arrived on the current connection. Returns true when this is the first evidence that
	/// the engine claimed the connection.
	pub fn ping_received(&self, now: Instant) -> bool {
		let mut state = self.0.lock();
		state.last_refresh = Some(now);
		let newly_claimed = !state.connection_claimed;
		state.connection_claimed = true;
		newly_claimed
	}

	/// Commands arrived on the current connection's WebSocket. Returns true when this is the first
	/// evidence that the engine claimed the connection. Serverless start payloads injected over
	/// HTTP do not count.
	pub fn commands_received(&self) -> bool {
		let mut state = self.0.lock();
		let newly_claimed = !state.connection_claimed;
		state.connection_claimed = true;
		// The claim refreshed the engine's timestamp after the install, so the install time is a
		// safe lower bound for it.
		state.last_refresh = match (state.last_refresh, state.connection_installed_at) {
			(Some(refresh), Some(installed)) => Some(refresh.max(installed)),
			(refresh, installed) => refresh.or(installed),
		};
		newly_claimed
	}

	/// The time the engine ping silence deadline counts from. Until the engine claims a
	/// connection it may still be counting from the last refresh, so a reconnect never moves the
	/// baseline forward on its own. An envoy the engine never refreshed counts from the install,
	/// which is earlier than any timestamp the engine could hold for it.
	pub fn baseline(&self) -> Option<Instant> {
		let state = *self.0.lock();
		state.last_refresh.or(state.connection_installed_at)
	}
}
