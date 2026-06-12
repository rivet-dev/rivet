// counter-latency: Rust port of scripts/counter-latency.ts.
// Subcommands:
//   concurrent       ramp raw WS tunnel-stress actors (steady or rolling).
//   agent-concurrent ramp SQLite-backed agent actors (steady or rolling).
//   agent-concurrent-2 cycle SQLite-backed agent actors through work and sleep.
//
// Env:
//   RUN_FOR_MS  optional run cap for both modes.
//   RIVET_POOL  runner pool name (default "k8s").

mod args;
mod concurrent;
mod endpoint;
mod log;
mod stats;
mod tee;
mod ws;

use std::sync::Arc;

use crate::args::{Args, EnvConfig};
use crate::concurrent::{WorkloadCtx, print_concurrent_summary, spawn_scale_down};
use crate::endpoint::Endpoint;
use crate::log::{BOLD, COLOR_MAX_MS, COLOR_MIN_MS, DIM, RESET, gradient_color};
use crate::stats::State;

fn main() {
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.expect("tokio runtime");
	runtime.block_on(run());
}

async fn run() {
	let parsed = args::parse_cli();
	let env_cfg = Arc::new(EnvConfig::from_env());

	let run_id = format!(
		"{}-{}",
		chrono::Utc::now().format("%Y%m%dT%H%M%S"),
		std::process::id(),
	);
	match tee::init(&run_id) {
		Ok(path) => eprintln!("counter-latency log: {}", path),
		Err(err) => {
			eprintln!("fatal: cannot open log file: {}", err);
			std::process::exit(1);
		}
	}

	let endpoint = match Endpoint::parse(&env_cfg.endpoint, env_cfg.rivet_pool.clone()) {
		Ok(e) => Arc::new(e),
		Err(err) => {
			eprintln!("fatal: {}", err);
			std::process::exit(1);
		}
	};
	print_header(&parsed, &env_cfg, &endpoint);

	match parsed {
		Args::Concurrent(concurrent_args) => {
			let state = Arc::new(State::new());
			let ctx = Arc::new(WorkloadCtx {
				endpoint: endpoint.clone(),
				args: Arc::new(concurrent_args.clone()),
				env: env_cfg.clone(),
				state: state.clone(),
			});
			install_signal_handlers(ctx.clone());
			concurrent::run_concurrent_mode(
				concurrent_args,
				env_cfg.clone(),
				endpoint.clone(),
				state.clone(),
			)
			.await;
		}
	}
}

fn install_signal_handlers(ctx: Arc<WorkloadCtx>) {
	let ctx_int = ctx.clone();
	tokio::spawn(async move {
		// First SIGINT: begin gradual scale-down. Second SIGINT: force immediate exit.
		if tokio::signal::ctrl_c().await.is_ok() {
			handle_first_signal(&ctx_int, "sigint");
		}
		if tokio::signal::ctrl_c().await.is_ok() {
			ctx_int.state.set_stopping();
			print_concurrent_summary(&ctx_int, "sigint-force");
			std::process::exit(130);
		}
	});

	#[cfg(unix)]
	{
		use tokio::signal::unix::{SignalKind, signal};
		let ctx_term = ctx.clone();
		tokio::spawn(async move {
			let Ok(mut sig) = signal(SignalKind::terminate()) else {
				return;
			};
			// First SIGTERM: begin gradual scale-down. Second SIGTERM: force immediate exit.
			if sig.recv().await.is_some() {
				handle_first_signal(&ctx_term, "sigterm");
			}
			if sig.recv().await.is_some() {
				ctx_term.state.set_stopping();
				print_concurrent_summary(&ctx_term, "sigterm-force");
				std::process::exit(143);
			}
		});
	}
}

fn handle_first_signal(ctx: &Arc<WorkloadCtx>, reason: &'static str) {
	if ctx.state.try_begin_scale_down() {
		let scale_down = std::time::Duration::from_millis(ctx.env.scale_down_ms);
		spawn_scale_down(ctx.clone(), scale_down, reason);
	}
}

fn print_header(args: &Args, env: &EnvConfig, endpoint: &Endpoint) {
	let mode = match args {
		Args::Concurrent(a) => match a.mode {
			args::ConcurrentMode::Concurrent => "concurrent",
			args::ConcurrentMode::AgentConcurrent => "agent-concurrent",
			args::ConcurrentMode::AgentConcurrent2 => "agent-concurrent-2",
		},
	};
	let header = format!(
		"{}counter-latency{} endpoint={} ns={} mode={} interval={}ms",
		BOLD,
		RESET,
		endpoint.display_origin,
		endpoint.namespace,
		mode,
		args.interval(),
	);
	match args {
		Args::Concurrent(a) => {
			let worker_mode = match a.worker_mode {
				args::WorkerMode::Steady => "steady",
				args::WorkerMode::Rolling => "rolling",
			};
			let agent_part = match a.mode {
				args::ConcurrentMode::AgentConcurrent => format!(
					" tokens-per-second={} duration-ms={}",
					a.tokens_per_second, a.duration_ms,
				),
				args::ConcurrentMode::AgentConcurrent2 => format!(
					" sleep-ms={} timeout-ms={} stagger-handle-ms={} query-multiplier={}",
					a.sleep_ms, a.timeout_ms, a.stagger_handle_ms, a.query_multiplier,
				),
				args::ConcurrentMode::Concurrent => String::new(),
			};
			let run_for_part = if env.run_for_ms > 0 {
				format!(" run-for-ms={}", env.run_for_ms)
			} else {
				String::new()
			};
			out!(
				"{} worker-mode={} concurrency={} message-every={}ms show-messages={} skip-ready-wait={} rvt-runner={}{}{}",
				header,
				worker_mode,
				a.concurrency,
				a.message_interval,
				a.show_messages,
				a.skip_ready_wait,
				env.rivet_pool,
				agent_part,
				run_for_part,
			);
		}
	}
	let mid = (COLOR_MIN_MS + COLOR_MAX_MS) / 2.0;
	out!(
		"{}gradient: {}{}ms{}{} -> {}{}ms{}{} -> {}{}ms{}",
		DIM,
		gradient_color(COLOR_MIN_MS),
		COLOR_MIN_MS as i64,
		RESET,
		DIM,
		gradient_color(mid),
		mid as i64,
		RESET,
		DIM,
		gradient_color(COLOR_MAX_MS),
		COLOR_MAX_MS as i64,
		RESET,
	);
	out!();
}
