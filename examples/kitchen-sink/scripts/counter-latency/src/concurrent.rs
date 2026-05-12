// Concurrent + agent-concurrent mode. Owns the per-worker WS loop with
// reconnect logic, the workload trait, and the live logging that mirrors
// scripts/counter-latency.ts.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

use crate::args::{
	ACTOR_STOPPED_CLOSE_CODE, ACTOR_STOPPED_CLOSE_REASON, ConcurrentArgs, ConcurrentMode,
	EnvConfig, MESSAGE_GAP_WARN_MS,
};
use crate::endpoint::Endpoint;
use crate::log::{
	BLUE, BOLD, DIM, GREEN, RED, RESET, YELLOW, color_ms, format_actor, iso_now, pad,
};
use crate::rtt::make_key;
use crate::stats::{State, WorkerHealth};
use crate::ws::open_raw_ws;

#[derive(Clone)]
pub struct ConcurrentWorkerOptions {
	pub message_interval: u64,
	pub show_messages: bool,
	pub skip_ready_wait: bool,
	pub tokens_per_second: f64,
	pub duration_ms: u64,
}

pub struct WorkloadCtx {
	pub endpoint: Arc<Endpoint>,
	pub args: Arc<ConcurrentArgs>,
	pub env: Arc<EnvConfig>,
	pub state: Arc<State>,
}

impl WorkloadCtx {
	pub fn log_prefix(&self) -> String {
		let ts = iso_now();
		let (pending, healthy, warning, ended) = self.state.count_worker_health();
		let width = format!("{}", self.args.concurrency).len();
		let pad_num = |n: i64| format!("{:>width$}", n, width = width);
		let workers_started = self.state.workers_started();
		let concurrency_part = format!(
			"c={}/{}",
			pad_num(workers_started),
			self.args.concurrency,
		);
		let status_part = format!(
			"s={}{}{}/{}{}{}/{}{}{}/{}{}{}",
			BLUE,
			pad_num(pending),
			RESET,
			GREEN,
			pad_num(healthy),
			RESET,
			YELLOW,
			pad_num(warning),
			RESET,
			RED,
			pad_num(ended),
			RESET,
		);
		format!("{}{}{} [{} {}]", DIM, ts, RESET, concurrency_part, status_part)
	}
}

pub trait Workload: Send + Sync {
	fn key_prefix(&self) -> &'static str;
	fn suppress_generic_gap(&self) -> bool {
		false
	}
	fn actor_name(&self) -> &'static str;
	fn on_open(
		&self,
		_ctx: Arc<WorkloadCtx>,
		_worker: u32,
		_key: String,
		_options: ConcurrentWorkerOptions,
		_send_tx: mpsc::Sender<String>,
	) -> WorkloadHooks {
		WorkloadHooks::default()
	}
}

#[derive(Default)]
pub struct WorkloadHooks {
	pub on_message: Option<Box<dyn Fn(&str) + Send + Sync>>,
}

pub struct TunnelStressWorkload;

impl Workload for TunnelStressWorkload {
	fn key_prefix(&self) -> &'static str {
		"cl-t"
	}
	fn actor_name(&self) -> &'static str {
		"tunnelStress"
	}
	fn on_open(
		&self,
		_ctx: Arc<WorkloadCtx>,
		_worker: u32,
		_key: String,
		options: ConcurrentWorkerOptions,
		send_tx: mpsc::Sender<String>,
	) -> WorkloadHooks {
		tokio::spawn(async move {
			let mut sequence: u64 = 0;
			loop {
				sleep(std::time::Duration::from_millis(options.message_interval)).await;
				sequence += 1;
				let payload = serde_json::json!({
					"sequence": sequence,
					"timestamp": chrono::Utc::now().timestamp_millis(),
				})
				.to_string();
				if send_tx.send(payload).await.is_err() {
					break;
				}
			}
		});
		WorkloadHooks::default()
	}
}

pub struct AgentWorkload;

impl Workload for AgentWorkload {
	fn key_prefix(&self) -> &'static str {
		"cl-a"
	}
	fn suppress_generic_gap(&self) -> bool {
		true
	}
	fn actor_name(&self) -> &'static str {
		"loadTestAgent"
	}
	fn on_open(
		&self,
		ctx: Arc<WorkloadCtx>,
		worker: u32,
		key: String,
		options: ConcurrentWorkerOptions,
		send_tx: mpsc::Sender<String>,
	) -> WorkloadHooks {
		let pending_inference_sends: Arc<Mutex<HashMap<String, Instant>>> =
			Arc::new(Mutex::new(HashMap::new()));

		// Periodic inference sender.
		let pending_sends_clone = pending_inference_sends.clone();
		let key_for_send = key.clone();
		let tokens_per_second = options.tokens_per_second;
		let duration_ms = options.duration_ms;
		let message_interval = options.message_interval;
		tokio::spawn(async move {
			let mut sequence: u64 = 0;
			let mut first = true;
			loop {
				if !first {
					sleep(std::time::Duration::from_millis(message_interval)).await;
				}
				first = false;
				sequence += 1;
				let now_ms = chrono::Utc::now().timestamp_millis() as u64;
				let request_id = format!(
					"agent-{}-{}-{}",
					worker,
					to_base36(now_ms),
					sequence
				);
				pending_sends_clone
					.lock()
					.await
					.insert(request_id.clone(), Instant::now());
				let payload = serde_json::json!({
					"type": "inference",
					"requestId": request_id,
					"tokensPerSecond": tokens_per_second,
					"durationMs": duration_ms,
				})
				.to_string();
				if send_tx.send(payload).await.is_err() {
					break;
				}
			}
			let _ = key_for_send;
		});

		let ctx_for_hook = ctx.clone();
		let key_for_hook = key.clone();
		let on_message: Box<dyn Fn(&str) + Send + Sync> = Box::new(move |data: &str| {
			let Ok(message) = serde_json::from_str::<serde_json::Value>(data) else {
				return;
			};
			let ty = message.get("type").and_then(|v| v.as_str()).unwrap_or("");
			if ty == "inference-start" {
				if let Some(request_id) = message.get("requestId").and_then(|v| v.as_str()) {
					let pending = pending_inference_sends.clone();
					let request_id = request_id.to_string();
					let ctx_inner = ctx_for_hook.clone();
					let key_inner = key_for_hook.clone();
					tokio::spawn(async move {
						let mut map = pending.lock().await;
						if let Some(sent_at) = map.remove(&request_id) {
							let elapsed_ms =
								sent_at.elapsed().as_secs_f64() * 1000.0;
							if elapsed_ms > MESSAGE_GAP_WARN_MS {
								log_message_gap(
									&ctx_inner,
									worker,
									&key_inner,
									None,
									elapsed_ms,
								);
							}
						}
					});
				}
			} else if ty == "slow-sql" {
				let elapsed_ms = message.get("elapsedMs").and_then(|v| v.as_f64());
				let request_id =
					message.get("requestId").and_then(|v| v.as_str()).unwrap_or("?");
				let token_index = message
					.get("tokenIndex")
					.and_then(|v| v.as_i64())
					.map(|n| n.to_string())
					.unwrap_or_else(|| "?".to_string());
				if let Some(ms) = elapsed_ms {
					let detail = format!("req={} token={}", request_id, token_index);
					log_slow_sql(&ctx_for_hook, worker, &key_for_hook, None, ms, &detail);
				}
			}
		});
		WorkloadHooks {
			on_message: Some(on_message),
		}
	}
}

fn to_base36(mut n: u64) -> String {
	if n == 0 {
		return "0".to_string();
	}
	let mut s = String::new();
	const ALPHA: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
	while n > 0 {
		s.push(ALPHA[(n % 36) as usize] as char);
		n /= 36;
	}
	s.chars().rev().collect()
}

pub async fn run_concurrent_mode(
	args: ConcurrentArgs,
	env: Arc<EnvConfig>,
	endpoint: Arc<Endpoint>,
	state: Arc<State>,
) {
	let args = Arc::new(args);
	let workload: Arc<dyn Workload> = match args.mode {
		ConcurrentMode::AgentConcurrent => Arc::new(AgentWorkload),
		ConcurrentMode::Concurrent => Arc::new(TunnelStressWorkload),
	};
	let ctx = Arc::new(WorkloadCtx {
		endpoint: endpoint.clone(),
		args: args.clone(),
		env: env.clone(),
		state: state.clone(),
	});

	// Run-for-ms guard: close workers after the deadline.
	if env.run_for_ms > 0 {
		let state_clone = state.clone();
		let dur = std::time::Duration::from_millis(env.run_for_ms);
		tokio::spawn(async move {
			sleep(dur).await;
			state_clone.set_stopping();
		});
	}

	let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
	for i in 0..args.concurrency {
		let id = i + 1;
		state.set_workers_started(id as i64);
		state.set_worker_health(id, WorkerHealth::Pending);
		let ctx_clone = ctx.clone();
		let workload_clone = workload.clone();
		let options = ConcurrentWorkerOptions {
			message_interval: args.message_interval,
			show_messages: args.show_messages,
			skip_ready_wait: args.skip_ready_wait,
			tokens_per_second: args.tokens_per_second,
			duration_ms: args.duration_ms,
		};
		let handle = tokio::spawn(async move {
			run_concurrent_worker(id, workload_clone, ctx_clone, options).await;
		});
		handles.push(handle);
		if i < args.concurrency - 1 {
			sleep(std::time::Duration::from_millis(args.interval)).await;
		}
	}

	for h in handles {
		let _ = h.await;
	}
	print_concurrent_summary(&ctx, "complete");
}

async fn run_concurrent_worker(
	worker: u32,
	workload: Arc<dyn Workload>,
	ctx: Arc<WorkloadCtx>,
	options: ConcurrentWorkerOptions,
) {
	let key = make_key(worker, workload.key_prefix());
	let mut reconnect = false;
	let actor_id: Option<String> = None;

	while !ctx.state.stopping() {
		let t0 = Instant::now();
		let url = ctx.endpoint.build_raw_ws_url(
			workload.actor_name(),
			&key,
			options.skip_ready_wait,
		);

		let ws = match open_raw_ws(&url).await {
			Ok(ws) => ws,
			Err(err) => {
				let elapsed = t0.elapsed().as_secs_f64() * 1000.0;
				log_connect_error(
					&ctx,
					worker,
					&key,
					actor_id.as_deref(),
					elapsed,
					&err.to_string(),
				);
				return;
			}
		};
		let connect_ms = t0.elapsed().as_secs_f64() * 1000.0;
		log_connect(&ctx, worker, &key, actor_id.as_deref(), connect_ms, reconnect);
		reconnect = false;

		let (mut sink, mut stream) = ws.split();
		let (send_tx, mut send_rx) = mpsc::channel::<String>(64);
		let hooks =
			workload.on_open(ctx.clone(), worker, key.clone(), options.clone(), send_tx);

		let mut first_message_logged = false;
		let mut last_message_at: Option<Instant> = None;
		let mut saw_websocket_error = false;
		let mut close_info: Option<(u16, String)> = None;

		loop {
			tokio::select! {
				biased;
				maybe = send_rx.recv() => {
					match maybe {
						Some(payload) => {
							if let Err(err) = sink.send(Message::Text(payload.into())).await {
								saw_websocket_error = true;
								log_websocket_error(&ctx, worker, &key, actor_id.as_deref());
								let _ = err;
								break;
							}
						}
						None => {}
					}
				}
				incoming = stream.next() => {
					match incoming {
						Some(Ok(Message::Text(text))) => {
							let now = Instant::now();
							let data = text.as_str();
							handle_incoming_message(
								&ctx,
								worker,
								&key,
								actor_id.as_deref(),
								&workload,
								t0,
								data,
								&mut first_message_logged,
								&mut last_message_at,
								now,
								options.show_messages,
								&hooks,
							);
						}
						Some(Ok(Message::Binary(bin))) => {
							let now = Instant::now();
							handle_incoming_binary(
								&ctx,
								worker,
								&key,
								actor_id.as_deref(),
								&workload,
								t0,
								bin.len(),
								&mut first_message_logged,
								&mut last_message_at,
								now,
								options.show_messages,
							);
						}
						Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {
							continue;
						}
						Some(Ok(Message::Close(frame))) => {
							let (code, reason) = match frame {
								Some(f) => (u16::from(f.code), f.reason.to_string()),
								None => (0, String::new()),
							};
							close_info = Some((code, reason));
							break;
						}
						Some(Err(_)) => {
							saw_websocket_error = true;
							log_websocket_error(&ctx, worker, &key, actor_id.as_deref());
							break;
						}
						None => break,
					}
				}
			}
		}

		// Clean shutdown: try to send a polite close.
		let _ = sink
			.send(Message::Close(Some(CloseFrame {
				code: CloseCode::Normal,
				reason: "counter-latency complete".into(),
			})))
			.await;

		let (code, reason) = close_info.unwrap_or((0, String::new()));
		if !ctx.state.stopping()
			&& !saw_websocket_error
			&& code == ACTOR_STOPPED_CLOSE_CODE
			&& reason == ACTOR_STOPPED_CLOSE_REASON
		{
			log_reconnect(&ctx, worker, &key, actor_id.as_deref(), code, &reason);
			reconnect = true;
		} else {
			let unclean = !ctx.state.stopping();
			let detail = format!("code={} reason={}", code, reason);
			log_disconnect(&ctx, worker, &key, actor_id.as_deref(), &detail, unclean);
		}
		if saw_websocket_error {
			ctx.state.set_worker_health(worker, WorkerHealth::Ended);
		}
		if !reconnect {
			break;
		}
	}
}

#[allow(clippy::too_many_arguments)]
fn handle_incoming_message(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	workload: &Arc<dyn Workload>,
	t0: Instant,
	data: &str,
	first_message_logged: &mut bool,
	last_message_at: &mut Option<Instant>,
	now: Instant,
	show_messages: bool,
	hooks: &WorkloadHooks,
) {
	if !*first_message_logged {
		*first_message_logged = true;
		let elapsed_ms = now.duration_since(t0).as_secs_f64() * 1000.0;
		log_first_message(ctx, worker, key, actor_id, elapsed_ms);
	} else if !workload.suppress_generic_gap() {
		if let Some(prev) = *last_message_at {
			let gap_ms = now.duration_since(prev).as_secs_f64() * 1000.0;
			if gap_ms > MESSAGE_GAP_WARN_MS {
				log_message_gap(ctx, worker, key, actor_id, gap_ms);
			}
		}
	}
	*last_message_at = Some(now);

	if show_messages {
		let prefix = ctx.log_prefix();
		crate::out!(
			"{} {}{} message={}",
			prefix,
			pad(key, 32),
			format_actor(actor_id),
			data,
		);
	}

	if let Some(handler) = &hooks.on_message {
		handler(data);
	}
}

#[allow(clippy::too_many_arguments)]
fn handle_incoming_binary(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	workload: &Arc<dyn Workload>,
	t0: Instant,
	bytes_len: usize,
	first_message_logged: &mut bool,
	last_message_at: &mut Option<Instant>,
	now: Instant,
	show_messages: bool,
) {
	if !*first_message_logged {
		*first_message_logged = true;
		let elapsed_ms = now.duration_since(t0).as_secs_f64() * 1000.0;
		log_first_message(ctx, worker, key, actor_id, elapsed_ms);
	} else if !workload.suppress_generic_gap() {
		if let Some(prev) = *last_message_at {
			let gap_ms = now.duration_since(prev).as_secs_f64() * 1000.0;
			if gap_ms > MESSAGE_GAP_WARN_MS {
				log_message_gap(ctx, worker, key, actor_id, gap_ms);
			}
		}
	}
	*last_message_at = Some(now);

	if show_messages {
		let prefix = ctx.log_prefix();
		crate::out!(
			"{} {}{} message=<binary {} bytes>",
			prefix,
			pad(key, 32),
			format_actor(actor_id),
			bytes_len,
		);
	}
}

fn log_connect(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	connect_ms: f64,
	reconnect: bool,
) {
	ctx.state.stats.connects.fetch_add(1, Ordering::Relaxed);
	if reconnect {
		ctx.state.stats.reconnects.fetch_add(1, Ordering::Relaxed);
	}
	ctx.state.set_worker_health(worker, WorkerHealth::Healthy);
	let prefix = ctx.log_prefix();
	let label = if reconnect { "reconnect" } else { "connect" };
	crate::out!(
		"{} {}{} {}={}",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		label,
		color_ms(connect_ms),
	);
}

fn log_first_message(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	first_message_ms: f64,
) {
	ctx.state.stats.first_messages.fetch_add(1, Ordering::Relaxed);
	let _ = worker;
	let prefix = ctx.log_prefix();
	crate::out!(
		"{} {}{} first-message={}",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		color_ms(first_message_ms),
	);
}

fn log_disconnect(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	reason: &str,
	unclean: bool,
) {
	ctx.state.stats.disconnects.fetch_add(1, Ordering::Relaxed);
	if unclean {
		ctx.state.stats.unclean_failures_or_disconnects.fetch_add(1, Ordering::Relaxed);
	}
	ctx.state.set_worker_health(worker, WorkerHealth::Ended);
	let prefix = ctx.log_prefix();
	let (label_prefix, label) = if unclean {
		(RED, "DISCONNECT")
	} else {
		(DIM, "disconnect")
	};
	crate::out!(
		"{} {}{} {}{} {}{}",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		label_prefix,
		label,
		reason,
		RESET,
	);
}

fn log_reconnect(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	code: u16,
	reason: &str,
) {
	ctx.state.set_worker_health(worker, WorkerHealth::Pending);
	let prefix = ctx.log_prefix();
	crate::out!(
		"{} {}{} actor-stopped reconnect code={} reason={}",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		code,
		reason,
	);
}

fn log_message_gap(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	gap_ms: f64,
) {
	ctx.state.stats.message_gaps.fetch_add(1, Ordering::Relaxed);
	ctx.state.stats.unclean_failures_or_disconnects.fetch_add(1, Ordering::Relaxed);
	ctx.state.flag_worker_warning(worker);
	let prefix = ctx.log_prefix();
	crate::out!(
		"{} {}{} {}MESSAGE-GAP {}{}",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		RED,
		color_ms(gap_ms),
		RESET,
	);
}

fn log_slow_sql(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	elapsed_ms: f64,
	detail: &str,
) {
	ctx.state.stats.slow_sql.fetch_add(1, Ordering::Relaxed);
	ctx.state.flag_worker_warning(worker);
	let prefix = ctx.log_prefix();
	crate::out!(
		"{} {}{} {}SLOW-SQL {} {}{}",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		YELLOW,
		color_ms(elapsed_ms),
		detail,
		RESET,
	);
}

fn log_connect_error(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	elapsed_ms: f64,
	reason: &str,
) {
	ctx.state.stats.connect_errors.fetch_add(1, Ordering::Relaxed);
	ctx.state.stats.unclean_failures_or_disconnects.fetch_add(1, Ordering::Relaxed);
	ctx.state.set_worker_health(worker, WorkerHealth::Ended);
	let prefix = ctx.log_prefix();
	crate::out!(
		"{} {}{} {}CONNECT-ERROR {}{} ({})",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		RED,
		reason,
		RESET,
		color_ms(elapsed_ms),
	);
}

fn log_websocket_error(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
) {
	ctx.state.stats.websocket_errors.fetch_add(1, Ordering::Relaxed);
	ctx.state.stats.unclean_failures_or_disconnects.fetch_add(1, Ordering::Relaxed);
	ctx.state.flag_worker_warning(worker);
	let prefix = ctx.log_prefix();
	crate::out!(
		"{} {}{} {}WEBSOCKET-ERROR{}",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		RED,
		RESET,
	);
}

pub fn print_concurrent_summary(ctx: &Arc<WorkloadCtx>, reason: &str) {
	let (pending, healthy, warning, ended) = ctx.state.count_worker_health();
	crate::out!(
		"{}counter-latency summary{} reason={} c={}/{} s={}{}{}/{}{}{}/{}{}{}/{}{}{} disconnects={} connect-errors={} websocket-errors={} message-gaps={} slow-sql={} connects={} reconnects={} first-messages={}",
		BOLD,
		RESET,
		reason,
		ctx.state.workers_started(),
		ctx.args.concurrency,
		BLUE, pending, RESET,
		GREEN, healthy, RESET,
		YELLOW, warning, RESET,
		RED, ended, RESET,
		ctx.state.stats.disconnects.load(Ordering::Relaxed),
		ctx.state.stats.connect_errors.load(Ordering::Relaxed),
		ctx.state.stats.websocket_errors.load(Ordering::Relaxed),
		ctx.state.stats.message_gaps.load(Ordering::Relaxed),
		ctx.state.stats.slow_sql.load(Ordering::Relaxed),
		ctx.state.stats.connects.load(Ordering::Relaxed),
		ctx.state.stats.reconnects.load(Ordering::Relaxed),
		ctx.state.stats.first_messages.load(Ordering::Relaxed),
	);
}
