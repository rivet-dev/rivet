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
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

use crate::args::{
	ACTOR_STOPPED_CLOSE_CODE, ACTOR_STOPPED_CLOSE_REASON, ConcurrentArgs, ConcurrentMode,
	EnvConfig, MESSAGE_GAP_WARN_MS, WorkerMode,
};
use crate::endpoint::Endpoint;
use crate::log::{
	BLUE, BOLD, CYAN, DIM, GREEN, RED, RESET, YELLOW, color_ms, format_actor, iso_now, pad,
};
use crate::stats::{State, WorkerHealth};
use crate::ws::open_raw_ws;

pub fn make_key(worker: u32, prefix: &str) -> String {
	let now_ms = chrono::Utc::now().timestamp_millis();
	format!("{}-{}-{}", prefix, worker, base36(now_ms as u64))
}

fn base36(mut n: u64) -> String {
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
	/// Prefix format: `<dim>TIMESTAMP</dim> [connecting/pinging/connected/slow/failed]`.
	///
	/// Order matches the worker lifecycle: handshake → pings → steady → slow → failed.
	/// Each cell is colored independently so the dominant state is visually obvious.
	pub fn log_prefix(&self) -> String {
		let ts = iso_now();
		let counts = self.state.count_worker_health();
		let width = format!("{}", self.args.concurrency).len();
		let pad_num = |n: i64| format!("{:>width$}", n, width = width);
		let status_part = format!(
			"[{}{}{}/{}{}{}/{}{}{}/{}{}{}/{}{}{}]",
			BLUE,
			pad_num(counts.connecting),
			RESET,
			CYAN,
			pad_num(counts.pinging),
			RESET,
			GREEN,
			pad_num(counts.connected),
			RESET,
			YELLOW,
			pad_num(counts.connected_slow),
			RESET,
			RED,
			pad_num(counts.failed),
			RESET,
		);
		format!("{}{}{} {}", DIM, ts, RESET, status_part)
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
	if matches!(args.mode, ConcurrentMode::AgentConcurrent2) {
		let ctx = Arc::new(WorkloadCtx {
			endpoint: endpoint.clone(),
			args: args.clone(),
			env: env.clone(),
			state: state.clone(),
		});
		run_agent_concurrent_2_mode(args.clone(), env.clone(), ctx.clone(), state.clone()).await;
		print_concurrent_summary(&ctx, "complete");
		return;
	}

	let workload: Arc<dyn Workload> = match args.mode {
		ConcurrentMode::AgentConcurrent => Arc::new(AgentWorkload),
		ConcurrentMode::Concurrent => Arc::new(TunnelStressWorkload),
		ConcurrentMode::AgentConcurrent2 => unreachable!(),
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

	let options = ConcurrentWorkerOptions {
		message_interval: args.message_interval,
		show_messages: args.show_messages,
		skip_ready_wait: args.skip_ready_wait,
		tokens_per_second: args.tokens_per_second,
		duration_ms: args.duration_ms,
	};

	match args.worker_mode {
		WorkerMode::Steady => {
			run_steady(args.clone(), ctx.clone(), workload, state.clone(), options).await;
		}
		WorkerMode::Rolling => {
			run_rolling(args.clone(), ctx.clone(), workload, state.clone(), options).await;
		}
	}

	print_concurrent_summary(&ctx, "complete");
}

async fn run_agent_concurrent_2_mode(
	args: Arc<ConcurrentArgs>,
	env: Arc<EnvConfig>,
	ctx: Arc<WorkloadCtx>,
	state: Arc<State>,
) {
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
		state.set_worker_health(id, WorkerHealth::Connecting);
		let ctx_clone = ctx.clone();
		let handle = tokio::spawn(async move {
			run_agent_concurrent_2_worker(id, ctx_clone).await;
		});
		handles.push(handle);
		if i < args.concurrency - 1 {
			sleep(std::time::Duration::from_millis(args.interval)).await;
		}
	}
	for handle in handles {
		let _ = handle.await;
	}
}

async fn run_agent_concurrent_2_worker(worker: u32, ctx: Arc<WorkloadCtx>) {
	let key = make_key(worker, "cl-a2");
	let mut sequence: u64 = 0;
	let actor_id: Option<String> = None;

	'worker_loop: while !ctx.state.stopping() {
		sequence += 1;
		ctx.state.set_worker_health(worker, WorkerHealth::Connecting);
		let t0 = Instant::now();
		let url =
			ctx.endpoint
				.build_raw_ws_url("loadTestAgent2", &key, ctx.args.skip_ready_wait);

		let ws = match open_raw_ws_with_stall_log(&url, &ctx, &key, actor_id.as_deref()).await {
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
		let t_connect = Instant::now();
		let connect_ms = t_connect.duration_since(t0).as_secs_f64() * 1000.0;
		record_connect(&ctx, worker, sequence > 1);

		let (mut sink, mut stream) = ws.split();
		let phase1 = run_ping_phase(
			&mut sink,
			&mut stream,
			t0,
			t_connect,
			connect_ms,
			&ctx,
			&key,
			actor_id.as_deref(),
		)
		.await;

		match &phase1 {
			PingOutcome::Completed {
				first_ms,
				second_ms,
				total_ms,
			} => {
				log_rtt_line(
					&ctx,
					worker,
					&key,
					actor_id.as_deref(),
					connect_ms,
					*first_ms,
					*second_ms,
					*total_ms,
					sequence > 1,
				);
			}
			PingOutcome::Failed { first_ms, close } => {
				log_partial_rtt(
					&ctx,
					worker,
					&key,
					actor_id.as_deref(),
					connect_ms,
					*first_ms,
					sequence > 1,
				);
				if let Some((code, reason)) = close {
					let detail = format!("code={} reason={}", code, reason);
					log_disconnect(&ctx, worker, &key, actor_id.as_deref(), &detail, true);
				}
				// Per-worker phase-1 failure: skip this iteration, continue outer loop.
				// Do not call set_stopping — that would kill the whole test on a single error.
				sleep(std::time::Duration::from_millis(ctx.args.sleep_ms)).await;
				continue 'worker_loop;
			}
		}

		for repeat_index in 0..ctx.args.query_multiplier {
			let request_id = format!(
				"agent2-{}-{}-{}-{}",
				worker,
				to_base36(sequence),
				repeat_index + 1,
				to_base36(chrono::Utc::now().timestamp_millis() as u64),
			);
			let payload = serde_json::json!({
				"type": "agent2_connect",
				"clientId": request_id,
				"staggerHandleMs": ctx.args.stagger_handle_ms,
			})
			.to_string();

			if let Err(err) = sink.send(Message::Text(payload.into())).await {
				let _ = err;
				log_websocket_error(&ctx, worker, &key, actor_id.as_deref());
				// Per-worker send error: drop this connection, continue outer loop.
				sleep(std::time::Duration::from_millis(ctx.args.sleep_ms)).await;
				continue 'worker_loop;
			}

			let result = timeout(
				std::time::Duration::from_millis(ctx.args.timeout_ms),
				wait_agent_concurrent_2_result(
					&mut stream,
					&ctx,
					worker,
					&key,
					actor_id.as_deref(),
				),
			)
			.await;

			match result {
				Ok(AgentConcurrent2Cycle::Result { total_ms, summary }) => {
					log_agent_concurrent_2_result(
						&ctx,
						worker,
						&key,
						actor_id.as_deref(),
						total_ms,
						&summary,
					);
				}
				Ok(AgentConcurrent2Cycle::ServerError { error }) => {
					log_agent_concurrent_2_error(&ctx, worker, &key, actor_id.as_deref(), &error);
					sleep(std::time::Duration::from_millis(ctx.args.sleep_ms)).await;
					continue 'worker_loop;
				}
				Ok(AgentConcurrent2Cycle::Closed { detail }) => {
					log_disconnect(&ctx, worker, &key, actor_id.as_deref(), &detail, true);
					sleep(std::time::Duration::from_millis(ctx.args.sleep_ms)).await;
					continue 'worker_loop;
				}
				Err(_) => {
					let detail = format!(
						"timeout waiting for agent-concurrent-2 result after {}ms",
						ctx.args.timeout_ms,
					);
					log_disconnect(&ctx, worker, &key, actor_id.as_deref(), &detail, true);
					sleep(std::time::Duration::from_millis(ctx.args.sleep_ms)).await;
					continue 'worker_loop;
				}
			}
		}

		let _ = sink
			.send(Message::Text(
				serde_json::json!({ "type": "force_sleep" }).to_string().into(),
			))
			.await;
		let _ = timeout(
			std::time::Duration::from_millis(5_000),
			wait_agent_concurrent_2_sleeping(&mut stream),
		)
		.await;
		let _ = sink
			.send(Message::Close(Some(CloseFrame {
				code: CloseCode::Normal,
				reason: "counter-latency complete".into(),
			})))
			.await;

		sleep(std::time::Duration::from_millis(ctx.args.sleep_ms)).await;
	}
}

enum AgentConcurrent2Cycle {
	Result { total_ms: f64, summary: String },
	ServerError { error: String },
	Closed { detail: String },
}

async fn wait_agent_concurrent_2_result<R>(
	stream: &mut R,
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
) -> AgentConcurrent2Cycle
where
	R: futures_util::stream::Stream<
			Item = Result<Message, tokio_tungstenite::tungstenite::Error>,
		> + Unpin,
{
	while let Some(incoming) = stream.next().await {
		match incoming {
			Ok(Message::Text(text)) => {
				let data = text.as_str();
				if ctx.args.show_messages {
					let prefix = ctx.log_prefix();
					crate::out!(
						"{} {}{} message={}",
						prefix,
						pad(key, 32),
						format_actor(actor_id),
						data,
					);
				}
				let Ok(message) = serde_json::from_str::<serde_json::Value>(data) else {
					continue;
				};
				let ty = message.get("type").and_then(|v| v.as_str()).unwrap_or("");
				if ty == "agent2_result" {
					let total_ms = message
						.get("totalMs")
						.and_then(|v| v.as_f64())
						.unwrap_or_default();
					let summary = summarize_agent_concurrent_2_result(&message, ctx);
					return AgentConcurrent2Cycle::Result { total_ms, summary };
				}
				if ty == "agent2_error" {
					let mut error = message
						.get("error")
						.and_then(|v| v.as_str())
						.unwrap_or("unknown server error")
						.to_string();
					if let Some(stats) = summarize_agent_concurrent_2_query_stats(&message, ctx) {
						error.push(' ');
						error.push_str(&stats);
					}
					return AgentConcurrent2Cycle::ServerError { error };
				}
			}
			Ok(Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => {}
			Ok(Message::Close(frame)) => {
				let detail = match frame {
					Some(f) => format!("code={} reason={}", u16::from(f.code), f.reason),
					None => "code=0 reason=".to_string(),
				};
				return AgentConcurrent2Cycle::Closed { detail };
			}
			Err(_) => {
				log_websocket_error(ctx, worker, key, actor_id);
				return AgentConcurrent2Cycle::Closed {
					detail: "websocket error".to_string(),
				};
			}
		}
	}
	AgentConcurrent2Cycle::Closed {
		detail: "stream ended".to_string(),
	}
}

async fn wait_agent_concurrent_2_sleeping<R>(stream: &mut R)
where
	R: futures_util::stream::Stream<
			Item = Result<Message, tokio_tungstenite::tungstenite::Error>,
		> + Unpin,
{
	while let Some(incoming) = stream.next().await {
		let Ok(Message::Text(text)) = incoming else {
			continue;
		};
		let Ok(message) = serde_json::from_str::<serde_json::Value>(text.as_str()) else {
			continue;
		};
		if message.get("type").and_then(|v| v.as_str()) == Some("sleeping") {
			return;
		}
	}
}

fn summarize_agent_concurrent_2_result(
	message: &serde_json::Value,
	ctx: &Arc<WorkloadCtx>,
) -> String {
	let Some(results) = message.get("results").and_then(|v| v.as_array()) else {
		return "results=none".to_string();
	};
	let mut parts = results
		.iter()
		.map(|result| {
			let name = result.get("name").and_then(|v| v.as_str()).unwrap_or("?");
			let total = result
				.get("totalMs")
				.and_then(|v| v.as_f64())
				.unwrap_or_default()
				.round() as i64;
			format!("{}={}ms", name, total)
		})
		.collect::<Vec<_>>();
	if let Some(stats) = summarize_agent_concurrent_2_query_stats(message, ctx) {
		parts.push(stats);
	}
	parts.join(" ")
}

fn summarize_agent_concurrent_2_query_stats(
	message: &serde_json::Value,
	ctx: &Arc<WorkloadCtx>,
) -> Option<String> {
	let stats = message.get("stats")?;
	let cycle = stats.get("cycle")?;
	let wake = stats.get("wake")?;
	let actor = stats.get("actor")?;

	let cycle_total = stat_i64(cycle, "total");
	ctx.state
		.stats
		.agent2_queries
		.fetch_add(cycle_total, Ordering::Relaxed);
	ctx.state
		.stats
		.agent2_reads
		.fetch_add(stat_i64(cycle, "reads"), Ordering::Relaxed);
	ctx.state
		.stats
		.agent2_mutations
		.fetch_add(stat_i64(cycle, "mutations"), Ordering::Relaxed);
	ctx.state
		.stats
		.agent2_tx
		.fetch_add(stat_i64(cycle, "tx"), Ordering::Relaxed);
	ctx.state
		.stats
		.agent2_other
		.fetch_add(stat_i64(cycle, "other"), Ordering::Relaxed);
	ctx.state
		.stats
		.agent2_rows
		.fetch_add(stat_i64(cycle, "rows"), Ordering::Relaxed);
	ctx.state
		.stats
		.agent2_query_errors
		.fetch_add(stat_i64(cycle, "errors"), Ordering::Relaxed);
	ctx.state
		.stats
		.agent2_slow_queries
		.fetch_add(stat_i64(cycle, "slow"), Ordering::Relaxed);

	let wake_index = stat_i64(stats, "wakeIndex");
	let actor_iteration = stat_i64(stats, "actorIteration");
	let wake_iteration = stat_i64(stats, "wakeIteration");
	let max_step = cycle
		.get("maxStep")
		.and_then(|v| v.as_str())
		.unwrap_or("");
	let max_ms = stat_i64(cycle, "maxMs");
	let top_tables = top_counter_entries(cycle.get("byTable"), 3);
	Some(format!(
		"wake={} iter={}/{} cycleQ={} r/m/tx/o={}/{}/{}/{} wakeQ={} actorQ={} rows={} qerr={} qslow={} maxQ={}:{}ms tables={}",
		wake_index,
		wake_iteration,
		actor_iteration,
		cycle_total,
		stat_i64(cycle, "reads"),
		stat_i64(cycle, "mutations"),
		stat_i64(cycle, "tx"),
		stat_i64(cycle, "other"),
		stat_i64(wake, "total"),
		stat_i64(actor, "total"),
		stat_i64(cycle, "rows"),
		stat_i64(cycle, "errors"),
		stat_i64(cycle, "slow"),
		max_step,
		max_ms,
		top_tables,
	))
}

fn stat_i64(value: &serde_json::Value, key: &str) -> i64 {
	value.get(key).and_then(|v| v.as_i64()).unwrap_or_default()
}

fn top_counter_entries(value: Option<&serde_json::Value>, limit: usize) -> String {
	let Some(map) = value.and_then(|v| v.as_object()) else {
		return "-".to_string();
	};
	let mut entries = map
		.iter()
		.filter_map(|(key, value)| value.as_i64().map(|count| (key.as_str(), count)))
		.collect::<Vec<_>>();
	entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
	let summary = entries
		.into_iter()
		.take(limit)
		.map(|(key, count)| format!("{}:{}", key, count))
		.collect::<Vec<_>>()
		.join(",");
	if summary.is_empty() {
		"-".to_string()
	} else {
		summary
	}
}

/// Steady scheduler. Spawn N workers up front (with --interval gaps for ramp). Each worker holds
/// its connection forever, reconnects on actor-stopped close.
async fn run_steady(
	args: Arc<ConcurrentArgs>,
	ctx: Arc<WorkloadCtx>,
	workload: Arc<dyn Workload>,
	state: Arc<State>,
	options: ConcurrentWorkerOptions,
) {
	let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
	for i in 0..args.concurrency {
		let id = i + 1;
		state.set_workers_started(id as i64);
		state.set_worker_health(id, WorkerHealth::Connecting);
		let ctx_clone = ctx.clone();
		let workload_clone = workload.clone();
		let options_clone = options.clone();
		let handle = tokio::spawn(async move {
			run_concurrent_worker(id, workload_clone, ctx_clone, options_clone, WorkerMode::Steady)
				.await;
		});
		handles.push(handle);
		if i < args.concurrency - 1 {
			sleep(std::time::Duration::from_millis(args.interval)).await;
		}
	}
	for h in handles {
		let _ = h.await;
	}
}

/// Rolling scheduler. Spawn rate is `--interval` (one worker per `interval` ms) throughout —
/// during ramp AND in steady state. `--concurrency` is a hard cap enforced via a semaphore: if N
/// workers are already in flight when the next spawn tick fires, the spawn blocks until one
/// finishes. Each worker runs one cycle (open → ping1 → ping2 → close) and exits, so the
/// scheduler launches a replacement on the next tick.
///
/// This intentionally keeps the spawn cadence steady even when workers are short-lived. With
/// `-c 1 -i 1000` it behaves like the old rtt mode (1 spawn/sec, 1 in flight). With
/// `-c 10 -i 100` it's 10 spawns/sec capped at 10 in flight.
async fn run_rolling(
	args: Arc<ConcurrentArgs>,
	ctx: Arc<WorkloadCtx>,
	workload: Arc<dyn Workload>,
	state: Arc<State>,
	options: ConcurrentWorkerOptions,
) {
	let permits = Arc::new(tokio::sync::Semaphore::new(args.concurrency as usize));
	let mut next_id: u32 = 0;
	let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
	let mut next_spawn = Instant::now();
	let interval = std::time::Duration::from_millis(args.interval);

	while !state.stopping() {
		// Steady spawn cadence: wait until the next scheduled tick before attempting to acquire
		// a permit. We use a fixed wall-clock cadence (`next_spawn += interval`) instead of
		// `sleep(interval)` after each spawn so that brief permit-wait stalls don't propagate
		// into permanent skew.
		let now = Instant::now();
		if next_spawn > now {
			tokio::time::sleep_until(next_spawn.into()).await;
		}
		next_spawn += interval;

		let Ok(permit) = Arc::clone(&permits).acquire_owned().await else {
			break;
		};
		if state.stopping() {
			drop(permit);
			break;
		}

		next_id += 1;
		let id = next_id;
		state.set_workers_started(id as i64);
		state.set_worker_health(id, WorkerHealth::Connecting);

		let ctx_clone = ctx.clone();
		let workload_clone = workload.clone();
		let options_clone = options.clone();
		let state_clone = state.clone();
		let handle = tokio::spawn(async move {
			run_concurrent_worker(id, workload_clone, ctx_clone, options_clone, WorkerMode::Rolling)
				.await;
			// Drop tracked health entry so the scoreboard counts the next replacement, not a
			// growing ledger of completed workers.
			state_clone.drop_worker_health(id);
			drop(permit);
		});
		handles.push(handle);
	}

	for h in handles {
		let _ = h.await;
	}
}

async fn run_concurrent_worker(
	worker: u32,
	workload: Arc<dyn Workload>,
	ctx: Arc<WorkloadCtx>,
	options: ConcurrentWorkerOptions,
	worker_mode: WorkerMode,
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

		let ws = match open_raw_ws_with_stall_log(&url, &ctx, &key, actor_id.as_deref()).await {
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
		let t_connect = Instant::now();
		let connect_ms = t_connect.duration_since(t0).as_secs_f64() * 1000.0;
		record_connect(&ctx, worker, reconnect);
		let was_reconnect = reconnect;
		reconnect = false;

		let (mut sink, mut stream) = ws.split();

		// Phase 1: ping probe RTT. Send {type:"ping", id:1} → wait for {type:"pong", id:1} →
		// send id:2 → wait for id:2. Workload-generated messages that arrive in between are
		// dropped here (they have no useful semantics yet — `on_open` hasn't run).
		let phase1 = run_ping_phase(
			&mut sink,
			&mut stream,
			t0,
			t_connect,
			connect_ms,
			&ctx,
			&key,
			actor_id.as_deref(),
		)
		.await;

		match &phase1 {
			PingOutcome::Completed {
				first_ms,
				second_ms,
				total_ms,
			} => {
				log_rtt_line(
					&ctx,
					worker,
					&key,
					actor_id.as_deref(),
					connect_ms,
					*first_ms,
					*second_ms,
					*total_ms,
					was_reconnect,
				);
			}
			PingOutcome::Failed { first_ms, close } => {
				log_partial_rtt(&ctx, worker, &key, actor_id.as_deref(), connect_ms, *first_ms, was_reconnect);
				if let Some((code, reason)) = close {
					let detail = format!("code={} reason={}", code, reason);
					log_disconnect(&ctx, worker, &key, actor_id.as_deref(), &detail, true);
				}
			}
		}

		// Rolling mode: we're done after one rtt line. Close and exit so the scheduler can
		// launch a replacement.
		if matches!(worker_mode, WorkerMode::Rolling) {
			let _ = sink
				.send(Message::Close(Some(CloseFrame {
					code: CloseCode::Normal,
					reason: "counter-latency complete".into(),
				})))
				.await;
			ctx.state.set_worker_health(worker, WorkerHealth::Failed);
			break;
		}

		// If phase 1 failed: maybe reconnect on actor-stopped close, otherwise exit.
		if let PingOutcome::Failed { close, .. } = &phase1 {
			if !ctx.state.stopping()
				&& close
					.as_ref()
					.map(|(code, reason)| {
						*code == ACTOR_STOPPED_CLOSE_CODE
							&& reason == ACTOR_STOPPED_CLOSE_REASON
					})
					.unwrap_or(false)
			{
				let (code, reason) = close.clone().unwrap();
				log_reconnect(&ctx, worker, &key, actor_id.as_deref(), code, &reason);
				reconnect = true;
				continue;
			}
			ctx.state.set_worker_health(worker, WorkerHealth::Failed);
			break;
		}

		// Phase 2 (steady): start the workload's periodic sender, then run the normal message
		// loop. Per-message MESSAGE-GAP / SLOW-SQL detection fires as before; no more rtt
		// timing in this phase.
		let (send_tx, mut send_rx) = mpsc::channel::<String>(64);
		let hooks =
			workload.on_open(ctx.clone(), worker, key.clone(), options.clone(), send_tx);

		let mut last_message_at: Option<Instant> = None;
		let mut saw_websocket_error = false;
		let mut close_info: Option<(u16, String)> = None;
		let mut steady_silence_since = Instant::now();
		// Log a stall warning at most once per silence window for this connection. Resets when
		// any inbound message arrives (silence window restarts).
		let mut steady_stall_logged = false;
		let mut steady_stall_tick = tokio::time::interval(STALL_TICK_INTERVAL);
		steady_stall_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
		steady_stall_tick.tick().await; // drain immediate tick

		loop {
			tokio::select! {
				biased;
				_ = steady_stall_tick.tick() => {
					if !steady_stall_logged {
						let elapsed_ms = steady_silence_since.elapsed().as_millis() as u64;
						if elapsed_ms >= STALL_WARN_THRESHOLD_MS {
							ctx.state.stats.stalls.fetch_add(1, Ordering::Relaxed);
							ctx.state.flag_worker_slow(worker);
							let prefix = ctx.log_prefix();
							crate::out!(
								"{} {}{} {}STALL{} stage=no inbound message in steady phase elapsed_ms={}",
								prefix,
								pad(&key, 32),
								format_actor(actor_id.as_deref()),
								YELLOW,
								RESET,
								elapsed_ms,
							);
							steady_stall_logged = true;
						}
					}
				}
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
					steady_silence_since = Instant::now();
					steady_stall_logged = false;
					match incoming {
						Some(Ok(Message::Text(text))) => {
							let now = Instant::now();
							let data = text.as_str();
							handle_steady_message(
								&ctx,
								worker,
								&key,
								actor_id.as_deref(),
								&workload,
								data,
								&mut last_message_at,
								now,
								options.show_messages,
								&hooks,
							);
						}
						Some(Ok(Message::Binary(bin))) => {
							let now = Instant::now();
							handle_steady_binary(
								&ctx,
								worker,
								&key,
								actor_id.as_deref(),
								&workload,
								bin.len(),
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
			ctx.state.set_worker_health(worker, WorkerHealth::Failed);
		}
		if !reconnect {
			break;
		}
	}
}

enum PingOutcome {
	Completed {
		first_ms: f64,
		second_ms: f64,
		total_ms: f64,
	},
	Failed {
		first_ms: Option<f64>,
		close: Option<(u16, String)>,
	},
}

/// Phase 1 of every connection: send two pings, wait for pongs, measure round-trip latency.
/// Pong messages are `{"type":"pong","id":N,...}`; anything else inbound is dropped so the rtt
/// numbers are clean. No client-side timeout — the cycle ends only when both pongs arrive or
/// the WS closes / errors. The engine + actor decide when to give up; we just measure what they
/// produce. While waiting on a stage > `STALL_WARN_THRESHOLD`, emit a periodic warning so the
/// operator can see what's blocked without waiting for the eventual close.
async fn run_ping_phase<S, R>(
	sink: &mut S,
	stream: &mut R,
	t0: Instant,
	t_connect: Instant,
	_connect_ms: f64,
	ctx: &Arc<WorkloadCtx>,
	key: &str,
	actor_id: Option<&str>,
) -> PingOutcome
where
	S: SinkExt<Message> + Unpin,
	R: futures_util::stream::Stream<
			Item = Result<Message, tokio_tungstenite::tungstenite::Error>,
		> + Unpin,
{
	const PING1: &str = r#"{"type":"ping","id":1}"#;
	const PING2: &str = r#"{"type":"ping","id":2}"#;

	let ping1_send_ts = Instant::now();
	if sink.send(Message::Text(PING1.into())).await.is_err() {
		return PingOutcome::Failed {
			first_ms: None,
			close: None,
		};
	}

	let mut first_ms: Option<f64> = None;
	let mut ping2_send_ts: Option<Instant> = None;
	let mut stage = PingStage::WaitingForPong1;
	let mut stage_start = ping1_send_ts;
	let mut stage_stall_logged = false;
	let mut stall_tick = tokio::time::interval(STALL_TICK_INTERVAL);
	stall_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	// Tick fires immediately on first poll; drain the immediate one so the first real tick
	// fires after STALL_TICK_INTERVAL.
	stall_tick.tick().await;

	loop {
		tokio::select! {
			_ = stall_tick.tick() => {
				// Log a stall warning at most once per stage per connection. Resets when the
				// stage advances (pong 1 → waiting for pong 2).
				if !stage_stall_logged {
					let elapsed_ms = stage_start.elapsed().as_millis() as u64;
					if elapsed_ms >= STALL_WARN_THRESHOLD_MS {
						log_phase1_stall(ctx, key, actor_id, stage, elapsed_ms);
						stage_stall_logged = true;
					}
				}
				continue;
			}
			incoming = stream.next() => {
				match incoming {
					Some(Ok(Message::Text(text))) => {
						match parse_pong_id(text.as_str()) {
							Some(1) => {
								let now = Instant::now();
								first_ms = Some(now.duration_since(ping1_send_ts).as_secs_f64() * 1000.0);
								let ts = Instant::now();
								if sink.send(Message::Text(PING2.into())).await.is_err() {
									return PingOutcome::Failed { first_ms, close: None };
								}
								ping2_send_ts = Some(ts);
								stage = PingStage::WaitingForPong2;
								stage_start = ts;
								stage_stall_logged = false;
							}
							Some(2) => {
								let Some(send_ts) = ping2_send_ts else {
									// We got pong id=2 before we sent ping id=2 — protocol bug; treat as failed.
									return PingOutcome::Failed { first_ms, close: None };
								};
								let now = Instant::now();
								let second_ms = now.duration_since(send_ts).as_secs_f64() * 1000.0;
								let total_ms = now.duration_since(t0).as_secs_f64() * 1000.0;
								let _ = t_connect;
								return PingOutcome::Completed {
									first_ms: first_ms.unwrap_or(0.0),
									second_ms,
									total_ms,
								};
							}
							_ => {
								// Some other workload-generated text frame. Drop it during phase 1.
							}
						}
					}
					Some(Ok(Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {
						// Drop binary / control frames during phase 1.
					}
					Some(Ok(Message::Close(frame))) => {
						let close = frame.map(|f| (u16::from(f.code), f.reason.to_string()));
						return PingOutcome::Failed { first_ms, close };
					}
					Some(Err(_)) | None => {
						return PingOutcome::Failed { first_ms, close: None };
					}
				}
			}
		}
	}
}

/// Stall warning threshold. Fires at most once per stage per connection (ws handshake, ping
/// stage, steady-mode silence window). After it fires, no further stall logs for that same
/// stage on the same connection until the stage advances or the connection cycles.
const STALL_WARN_THRESHOLD_MS: u64 = 2_000;
const STALL_TICK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

#[derive(Copy, Clone)]
enum PingStage {
	WaitingForPong1,
	WaitingForPong2,
}

impl PingStage {
	fn label(self) -> &'static str {
		match self {
			PingStage::WaitingForPong1 => "waiting for pong id=1",
			PingStage::WaitingForPong2 => "waiting for pong id=2",
		}
	}
}

fn log_phase1_stall(
	ctx: &Arc<WorkloadCtx>,
	key: &str,
	actor_id: Option<&str>,
	stage: PingStage,
	elapsed_ms: u64,
) {
	ctx.state
		.stats
		.stalls
		.fetch_add(1, Ordering::Relaxed);
	let prefix = ctx.log_prefix();
	crate::out!(
		"{} {}{} {}STALL{} stage={} elapsed_ms={}",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		YELLOW,
		RESET,
		stage.label(),
		elapsed_ms,
	);
}

/// Parse `{"type":"pong","id":N,...}`. Returns Some(N) if the frame is a pong, None otherwise.
/// Tolerant of unrelated fields and surrounding workload messages.
fn parse_pong_id(text: &str) -> Option<u64> {
	let parsed: serde_json::Value = serde_json::from_str(text).ok()?;
	if parsed.get("type")?.as_str()? != "pong" {
		return None;
	}
	parsed.get("id")?.as_u64()
}

/// Wraps `open_raw_ws` with a periodic stall warning while the WS handshake is still pending.
/// The connect future itself has no client-side timeout — we just observe and log how long it's
/// taking. Returns the same Result the underlying open returns.
async fn open_raw_ws_with_stall_log(
	url: &str,
	ctx: &Arc<WorkloadCtx>,
	key: &str,
	actor_id: Option<&str>,
) -> anyhow::Result<crate::ws::Ws> {
	let connect_started = Instant::now();
	// At most one stall log per handshake attempt.
	let mut stall_logged = false;
	let mut stall_tick = tokio::time::interval(STALL_TICK_INTERVAL);
	stall_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	stall_tick.tick().await; // drain immediate tick

	let connect_future = open_raw_ws(url);
	tokio::pin!(connect_future);

	loop {
		tokio::select! {
			res = &mut connect_future => {
				return res;
			}
			_ = stall_tick.tick() => {
				if !stall_logged {
					let elapsed_ms = connect_started.elapsed().as_millis() as u64;
					if elapsed_ms >= STALL_WARN_THRESHOLD_MS {
						ctx.state.stats.stalls.fetch_add(1, Ordering::Relaxed);
						let prefix = ctx.log_prefix();
						crate::out!(
							"{} {}{} {}STALL{} stage=waiting for ws handshake elapsed_ms={}",
							prefix,
							pad(key, 32),
							format_actor(actor_id),
							YELLOW,
							RESET,
							elapsed_ms,
						);
						stall_logged = true;
					}
				}
			}
		}
	}
}

#[allow(clippy::too_many_arguments)]
fn handle_steady_message(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	workload: &Arc<dyn Workload>,
	data: &str,
	last_message_at: &mut Option<Instant>,
	now: Instant,
	show_messages: bool,
	hooks: &WorkloadHooks,
) {
	if !workload.suppress_generic_gap() {
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
fn handle_steady_binary(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	workload: &Arc<dyn Workload>,
	bytes_len: usize,
	last_message_at: &mut Option<Instant>,
	now: Instant,
	show_messages: bool,
) {
	if !workload.suppress_generic_gap() {
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

/// Bump the connect/reconnect counters and mark the worker healthy. The rtt line itself is
/// deferred until the second inbound message arrives (combined `connect=… first=… second=…
/// total=…`). If the connection closes before that, `log_partial_rtt` emits whatever fragments
/// we have.
fn record_connect(ctx: &Arc<WorkloadCtx>, worker: u32, reconnect: bool) {
	ctx.state.stats.connects.fetch_add(1, Ordering::Relaxed);
	if reconnect {
		ctx.state.stats.reconnects.fetch_add(1, Ordering::Relaxed);
	}
	ctx.state.set_worker_health(worker, WorkerHealth::Pinging);
}

/// rtt-shape line. Same format as the old `rtt` subcommand for parity:
/// `connect=… first=… second=… total=…`. Also promotes the worker from `Pinging` → `Connected`
/// since phase 1 is complete by definition when this line emits.
#[allow(clippy::too_many_arguments)]
fn log_rtt_line(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	connect_ms: f64,
	first_ms: f64,
	second_ms: f64,
	total_ms: f64,
	reconnect: bool,
) {
	ctx.state.stats.first_messages.fetch_add(1, Ordering::Relaxed);
	ctx.state.set_worker_health(worker, WorkerHealth::Connected);
	let prefix = ctx.log_prefix();
	let label = if reconnect { "reconnect" } else { "connect" };
	crate::out!(
		"{} {}{} {}={} first={} second={} total={}",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		label,
		color_ms(connect_ms),
		color_ms(first_ms),
		color_ms(second_ms),
		color_ms(total_ms),
	);
}

/// Fallback for connections that close before delivering enough messages to compute the full
/// rtt line. Emits whatever fragments are available so the line is still visible.
///
/// Also downgrades the worker's health from Healthy → Ended at this point so the scoreboard
/// reflects the failure immediately instead of waiting for the subsequent DISCONNECT log to
/// flip the count.
fn log_partial_rtt(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	connect_ms: f64,
	first_ms: Option<f64>,
	reconnect: bool,
) {
	ctx.state.set_worker_health(worker, WorkerHealth::Failed);
	let prefix = ctx.log_prefix();
	let label = if reconnect { "reconnect" } else { "connect" };
	let first_part = first_ms
		.map(|ms| format!(" first={}", color_ms(ms)))
		.unwrap_or_else(|| " first=none".to_string());
	crate::out!(
		"{} {}{} {}FAILED{} {}={}{} second=none total=none",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		RED,
		RESET,
		label,
		color_ms(connect_ms),
		first_part,
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
	ctx.state.set_worker_health(worker, WorkerHealth::Failed);
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
	ctx.state.set_worker_health(worker, WorkerHealth::Connecting);
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
	ctx.state.flag_worker_slow(worker);
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
	ctx.state.flag_worker_slow(worker);
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
	ctx.state.set_worker_health(worker, WorkerHealth::Failed);
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
	ctx.state.flag_worker_slow(worker);
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

fn log_agent_concurrent_2_result(
	ctx: &Arc<WorkloadCtx>,
	_worker: u32,
	key: &str,
	actor_id: Option<&str>,
	total_ms: f64,
	summary: &str,
) {
	let prefix = ctx.log_prefix();
	crate::out!(
		"{} {}{} agent-concurrent-2 total={} {}",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		color_ms(total_ms),
		summary,
	);
}

fn log_agent_concurrent_2_error(
	ctx: &Arc<WorkloadCtx>,
	worker: u32,
	key: &str,
	actor_id: Option<&str>,
	error: &str,
) {
	ctx.state.stats.slow_sql.fetch_add(1, Ordering::Relaxed);
	ctx.state.stats.unclean_failures_or_disconnects.fetch_add(1, Ordering::Relaxed);
	ctx.state.set_worker_health(worker, WorkerHealth::Failed);
	let prefix = ctx.log_prefix();
	crate::out!(
		"{} {}{} {}AGENT-CONCURRENT-2-ERROR {}{}",
		prefix,
		pad(key, 32),
		format_actor(actor_id),
		RED,
		error,
		RESET,
	);
}

pub fn print_concurrent_summary(ctx: &Arc<WorkloadCtx>, reason: &str) {
	let counts = ctx.state.count_worker_health();
	crate::out!(
		"{}counter-latency summary{} reason={} workers={} [{}{}{}/{}{}{}/{}{}{}/{}{}{}/{}{}{}] disconnects={} connect-errors={} websocket-errors={} message-gaps={} slow-sql={} stalls={} connects={} reconnects={} first-messages={} agent2-q={} agent2-r/m/tx/o={}/{}/{}/{} agent2-rows={} agent2-qerr={} agent2-qslow={}",
		BOLD,
		RESET,
		reason,
		ctx.state.workers_started(),
		BLUE, counts.connecting, RESET,
		CYAN, counts.pinging, RESET,
		GREEN, counts.connected, RESET,
		YELLOW, counts.connected_slow, RESET,
		RED, counts.failed, RESET,
		ctx.state.stats.disconnects.load(Ordering::Relaxed),
		ctx.state.stats.connect_errors.load(Ordering::Relaxed),
		ctx.state.stats.websocket_errors.load(Ordering::Relaxed),
		ctx.state.stats.message_gaps.load(Ordering::Relaxed),
		ctx.state.stats.slow_sql.load(Ordering::Relaxed),
		ctx.state.stats.stalls.load(Ordering::Relaxed),
		ctx.state.stats.connects.load(Ordering::Relaxed),
		ctx.state.stats.reconnects.load(Ordering::Relaxed),
		ctx.state.stats.first_messages.load(Ordering::Relaxed),
		ctx.state.stats.agent2_queries.load(Ordering::Relaxed),
		ctx.state.stats.agent2_reads.load(Ordering::Relaxed),
		ctx.state.stats.agent2_mutations.load(Ordering::Relaxed),
		ctx.state.stats.agent2_tx.load(Ordering::Relaxed),
		ctx.state.stats.agent2_other.load(Ordering::Relaxed),
		ctx.state.stats.agent2_rows.load(Ordering::Relaxed),
		ctx.state.stats.agent2_query_errors.load(Ordering::Relaxed),
		ctx.state.stats.agent2_slow_queries.load(Ordering::Relaxed),
	);
	if let Some(path) = crate::tee::log_file_path() {
		crate::out!("counter-latency log: {}", path);
	}
}
