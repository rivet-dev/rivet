//! Reproduction for the runner-side send stall seen on 2026-09-14 (ACT-6470).
//!
//! Production observation on `thread-actors-runner-597457cc5d-97wvb`, from Google Managed
//! Prometheus, compared against its ReplicaSet siblings:
//!
//! - `rivetkit_envoy_client_ws_tx_lock_hold_duration_seconds` summed 12-15 s/min, against a
//!   sibling baseline near 0.5 s/min, with a p99.9 tail of 1-2.5 s.
//! - `rivetkit_envoy_client_ws_tx_lock_wait_duration_seconds` summed 145-222 s/min, against a
//!   baseline near 2 s/min.
//! - The pod drained its engine WebSocket at roughly 2-3 MB/s while this was happening.
//!
//! Hypothesis under test: `ws_send_for_session` serializes the message *inside* the `ws_tx`
//! critical section, so one large actor-to-client WebSocket message blocks every other sender on
//! that socket, pongs included, for as long as its encode takes. The sibling function
//! `ws_send_http_for_session` encodes before taking its lock, so the two paths disagree.
//!
//! This test asserts nothing about a fix. It drives the real `ws_send` path and reads the same two
//! histograms that were read from production, so the hypothesis can be falsified:
//!
//! - If a 16 MiB message holds the lock for microseconds, encode-under-lock costs nothing here and
//!   the hypothesis is wrong.
//! - If pong wait does not rise while a large send runs concurrently, the lock is not what couples
//!   them and the hypothesis is wrong.
//!
//! Scale caveat: this build has `serde_bytes` on the generated payload types, which production did
//! not. Bulk byte encoding is roughly 9x cheaper than the per-byte encoding that ran during the
//! incident, so the numbers here are a lower bound on the production effect rather than a match for
//! it.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rivet_envoy_client::config::{
	BoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse, WebSocketHandler,
	WebSocketSender,
};
use rivet_envoy_client::connection::ws_send;
use rivet_envoy_client::context::{SharedContext, WsTxMessage};
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::metrics::METRICS;
use rivet_envoy_protocol as protocol;
use tokio::sync::mpsc;

/// Large enough that a per-message encode is measurable. Amp's reported mean for the messages
/// involved was about 1 MiB, with a much larger tail.
const PAYLOAD_BYTES: usize = 16 * 1024 * 1024;

/// How long each phase drives small senders. A fixed duration rather than a fixed count, because a
/// counted loop finishes in microseconds and never overlaps the large encode it is supposed to
/// contend with.
const PHASE: Duration = Duration::from_millis(500);

/// Head start for the large sender, so it holds the lock before small senders arrive.
const HEAD_START: Duration = Duration::from_millis(50);

struct IdleCallbacks;

impl EnvoyCallbacks for IdleCallbacks {
	fn on_actor_start(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_generation: u32,
		_config: protocol::ActorConfig,
		_preloaded_kv: Option<protocol::PreloadedKv>,
	) -> BoxFuture<anyhow::Result<()>> {
		Box::pin(async { Ok(()) })
	}

	fn on_shutdown(&self) {}

	fn fetch(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_gateway_id: protocol::GatewayId,
		_request_id: protocol::RequestId,
		_request: HttpRequest,
	) -> BoxFuture<anyhow::Result<HttpResponse>> {
		Box::pin(async { anyhow::bail!("fetch should not be called") })
	}

	fn websocket(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_gateway_id: protocol::GatewayId,
		_request_id: protocol::RequestId,
		_request: HttpRequest,
		_path: String,
		_headers: HashMap<String, String>,
		_is_hibernatable: bool,
		_is_restoring_hibernatable: bool,
		_sender: WebSocketSender,
	) -> BoxFuture<anyhow::Result<WebSocketHandler>> {
		Box::pin(async { anyhow::bail!("websocket should not be called") })
	}

	fn can_hibernate(
		&self,
		_actor_id: &str,
		_gateway_id: &protocol::GatewayId,
		_request_id: &protocol::RequestId,
		_request: &HttpRequest,
	) -> BoxFuture<anyhow::Result<bool>> {
		Box::pin(async { Ok(false) })
	}
}

fn new_shared_context() -> Arc<SharedContext> {
	let (envoy_tx, _envoy_rx) = mpsc::unbounded_channel();
	Arc::new(SharedContext {
		config: EnvoyConfig {
			version: 1,
			endpoint: "http://127.0.0.1:1".to_string(),
			token: None,
			namespace: "test".to_string(),
			pool_name: "test".to_string(),
			prepopulate_actor_names: HashMap::new(),
			metadata: None,
			not_global: true,
			debug_latency_ms: None,
			callbacks: Arc::new(IdleCallbacks),
		},
		envoy_key: "test-envoy".to_string(),
		envoy_tx,
		actors: Arc::new(std::sync::Mutex::new(HashMap::new())),
		actors_notify: Arc::new(tokio::sync::Notify::new()),
		live_tunnel_requests: Arc::new(std::sync::Mutex::new(HashMap::new())),
		pending_hibernation_restores: Arc::new(std::sync::Mutex::new(HashMap::new())),
		ws_tx: Arc::new(tokio::sync::Mutex::new(
			None::<mpsc::UnboundedSender<WsTxMessage>>,
		)),
		http_ws_tx: Arc::new(tokio::sync::Mutex::new(None)),
		connection_session: std::sync::atomic::AtomicU64::new(0),
		next_connection_session: std::sync::atomic::AtomicU64::new(0),
		connection_session_tx: tokio::sync::watch::channel(0).0,
		protocol_metadata: Arc::new(tokio::sync::Mutex::new(None)),
		shutting_down: std::sync::atomic::AtomicBool::new(false),
		last_ping_ts: std::sync::atomic::AtomicI64::new(0),
		stopped_tx: tokio::sync::watch::channel(true).0,
	})
}

fn pong() -> protocol::ToRivet {
	protocol::ToRivet::ToRivetPong(protocol::ToRivetPong { ts: 0 })
}

fn large_actor_message() -> protocol::ToRivet {
	protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
		message_id: protocol::MessageId {
			gateway_id: [1; 4],
			request_id: [2; 4],
			message_index: 0,
		},
		message_kind: protocol::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
			protocol::ToRivetWebSocketMessage {
				data: vec![0xAB; PAYLOAD_BYTES],
				binary: true,
			},
		),
	})
}

/// Running totals of one histogram, so each phase can be measured as a delta. The metrics are
/// process-global, so absolute values carry work from earlier phases.
fn wait_totals(kind: &str) -> (f64, u64) {
	let metric = METRICS
		.ws_tx_lock_wait_duration_seconds
		.with_label_values(&[kind]);
	(metric.get_sample_sum(), metric.get_sample_count())
}

fn hold_totals(kind: &str) -> (f64, u64) {
	let metric = METRICS
		.ws_tx_lock_hold_duration_seconds
		.with_label_values(&[kind]);
	(metric.get_sample_sum(), metric.get_sample_count())
}

fn mean_ms(before: (f64, u64), after: (f64, u64)) -> f64 {
	let sum = after.0 - before.0;
	let count = after.1.saturating_sub(before.1);
	if count == 0 {
		return 0.0;
	}
	sum / count as f64 * 1000.0
}

/// Drives small sends for a fixed window and returns their mean lock wait and how many ran.
async fn pong_phase(shared: &Arc<SharedContext>) -> (f64, u64) {
	let before = wait_totals("pong");
	let deadline = Instant::now() + PHASE;
	let mut sends = 0_u64;
	while Instant::now() < deadline {
		ws_send(shared, pong()).await;
		sends += 1;
	}
	(mean_ms(before, wait_totals("pong")), sends)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn large_send_blocks_small_senders_on_ws_tx() {
	let shared = new_shared_context();
	let (tx, mut rx) = mpsc::unbounded_channel::<WsTxMessage>();
	*shared.ws_tx.lock().await = Some(tx);

	// Drain the writer side so encoded buffers are freed instead of accumulating. The real writer
	// task does the same thing, minus the socket.
	tokio::spawn(async move { while rx.recv().await.is_some() {} });

	// Phase 1: small senders with nothing else holding the lock.
	let (uncontended_wait_ms, uncontended_sends) = pong_phase(&shared).await;

	// Phase 2: the same small senders, while large actor messages are encoded under the lock.
	let stop = Arc::new(AtomicBool::new(false));
	let large_sender = tokio::spawn({
		let shared = shared.clone();
		let stop = stop.clone();
		async move {
			let mut sent = 0_u64;
			while !stop.load(Ordering::Relaxed) {
				ws_send(&shared, large_actor_message()).await;
				sent += 1;
			}
			sent
		}
	});

	// Let the large sender get the lock before small senders start arriving.
	tokio::time::sleep(HEAD_START).await;

	let hold_before = hold_totals("tunnel_message");
	let (contended_wait_ms, contended_sends) = pong_phase(&shared).await;

	stop.store(true, Ordering::Relaxed);
	let large_sends = large_sender.await.expect("large sender task");
	let large_hold_ms = mean_ms(hold_before, hold_totals("tunnel_message"));

	println!(
		"payload {} MiB | large sends during phase {large_sends} | large hold mean {large_hold_ms:.3} ms",
		PAYLOAD_BYTES / 1024 / 1024,
	);
	println!(
		"pong wait uncontended {uncontended_wait_ms:.4} ms over {uncontended_sends} sends | \
		 contended {contended_wait_ms:.4} ms over {contended_sends} sends",
	);

	// An overlap of zero means the phases did not actually contend and the run proves nothing.
	assert!(
		large_sends > 0,
		"no large send completed during the contended phase, so nothing was measured",
	);

	// Falsification 1: encoding a large message under the lock has to cost something measurable.
	assert!(
		large_hold_ms > 1.0,
		"encoding {} MiB held the lock for only {large_hold_ms:.3} ms, so encode-under-lock is not \
		 the amplifier this reproduction assumes",
		PAYLOAD_BYTES / 1024 / 1024,
	);

	// Falsification 2: if the lock is what couples them, small senders must wait on that encode.
	assert!(
		contended_wait_ms > uncontended_wait_ms * 5.0,
		"pong wait barely moved ({uncontended_wait_ms:.3} ms -> {contended_wait_ms:.3} ms), so the \
		 ws_tx lock is not what couples large sends to small ones",
	);
}
