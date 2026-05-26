// CLI + env parsing. Uses clap derive with short flags for every option.

use std::env;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};

pub const DEFAULT_CONCURRENCY: u32 = 1_000;
pub const DEFAULT_CONCURRENT_INTERVAL_MS: u64 = 300;
pub const DEFAULT_MESSAGE_INTERVAL_MS: u64 = 1_000;
pub const DEFAULT_AGENT_MESSAGE_INTERVAL_MS: u64 = 30_000;
pub const DEFAULT_AGENT_CONCURRENT_2_SLEEP_MS: u64 = 5_000;
pub const DEFAULT_AGENT_CONCURRENT_2_TIMEOUT_MS: u64 = 120_000;
pub const DEFAULT_AGENT_CONCURRENT_2_QUERY_MULTIPLIER: u32 = 1;
pub const DEFAULT_TOKENS_PER_SECOND: f64 = 20.0;
pub const DEFAULT_DURATION_MS: u64 = 5_000;
pub const MESSAGE_GAP_WARN_MS: f64 = 3_000.0;
pub const ACTOR_STOPPED_CLOSE_CODE: u16 = 1000;
pub const ACTOR_STOPPED_CLOSE_REASON: &str = "hack_force_close";

#[derive(Parser)]
#[command(
	name = "counter-latency",
	about = "Mini load-test client for Rivet kitchen-sink actors",
	long_about = "Subcommands:\n  \
		concurrent       ramp raw WebSocket tunnel-stress actors (steady or rolling)\n  \
		agent-concurrent ramp SQLite-backed agent actors (steady or rolling)\n  \
		agent-concurrent-2 cycle SQLite-backed agent actors through work and sleep\n\nEnv:\n  \
		RIVET_ENDPOINT  required, proto://<ns>:<token>@host\n  \
		RIVET_POOL      runner pool name (default k8s)\n  \
		RUN_FOR_MS      stop after this many ms"
)]
struct Cli {
	#[command(subcommand)]
	command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
	/// Ramp raw WebSocket tunnel-stress actors. Set `-c 1 --mode rolling` for an rtt-style workload.
	Concurrent(ConcurrentCli),
	/// Ramp SQLite-backed agent actors. Set `-c 1 --mode rolling` for an rtt-style workload.
	#[command(name = "agent-concurrent")]
	AgentConcurrent(ConcurrentCli),
	/// Cycle SQLite-backed agent actors through one workload pass and forced sleep.
	#[command(name = "agent-concurrent-2")]
	AgentConcurrent2(ConcurrentCli),
}

#[derive(clap::Args, Clone)]
struct ConcurrentCli {
	/// Worker lifecycle: `steady` keeps each connection alive; `rolling` closes the connection
	/// after the second inbound message (rtt-style) and a fresh worker replaces it, maintaining N
	/// in-flight.
	#[arg(short = 'M', long = "mode", value_enum, default_value_t = WorkerMode::Steady)]
	worker_mode: WorkerMode,
	/// Ramp-up gap in ms between connection starts.
	#[arg(short = 'i', long, default_value_t = DEFAULT_CONCURRENT_INTERVAL_MS)]
	interval: u64,
	/// Number of in-flight connections.
	#[arg(short = 'c', long, default_value_t = DEFAULT_CONCURRENCY)]
	concurrency: u32,
	/// Gap between client messages in ms (default: 1000 concurrent / 30000 agent-concurrent).
	#[arg(short = 'm', long = "message-interval-ms")]
	message_interval_ms: Option<u64>,
	/// SQLite token inserts per second (agent-concurrent only).
	#[arg(short = 't', long, default_value_t = DEFAULT_TOKENS_PER_SECOND)]
	tokens_per_second: f64,
	/// Inference stream duration in ms (agent-concurrent only).
	#[arg(short = 'd', long, default_value_t = DEFAULT_DURATION_MS)]
	duration_ms: u64,
	/// Log all received WebSocket messages.
	#[arg(short = 's', long)]
	show_messages: bool,
	/// Wait for actor ready before connecting (default: skip).
	#[arg(short = 'w', long)]
	wait_ready: bool,
	/// Delay after forcing actor sleep before reconnecting (agent-concurrent-2 only).
	#[arg(long, default_value_t = DEFAULT_AGENT_CONCURRENT_2_SLEEP_MS)]
	sleep_ms: u64,
	/// Per-workload timeout in ms (agent-concurrent-2 only).
	#[arg(long, default_value_t = DEFAULT_AGENT_CONCURRENT_2_TIMEOUT_MS)]
	timeout_ms: u64,
	/// Delay before the server-side write transaction starts (agent-concurrent-2 only).
	#[arg(long, default_value_t = 0)]
	stagger_handle_ms: u64,
	/// Number of times to repeat the SQL workload per cycle (agent-concurrent-2 only).
	#[arg(long, default_value_t = DEFAULT_AGENT_CONCURRENT_2_QUERY_MULTIPLIER)]
	query_multiplier: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum WorkerMode {
	/// Each worker keeps its connection alive forever and reconnects on actor-stopped close.
	Steady,
	/// Each worker closes after its second inbound message. A fresh worker immediately replaces it.
	Rolling,
}

#[derive(Clone)]
pub struct ConcurrentArgs {
	pub mode: ConcurrentMode,
	pub worker_mode: WorkerMode,
	pub interval: u64,
	pub concurrency: u32,
	pub message_interval: u64,
	pub show_messages: bool,
	pub skip_ready_wait: bool,
	pub tokens_per_second: f64,
	pub duration_ms: u64,
	pub sleep_ms: u64,
	pub timeout_ms: u64,
	pub stagger_handle_ms: u64,
	pub query_multiplier: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConcurrentMode {
	Concurrent,
	AgentConcurrent,
	AgentConcurrent2,
}

#[derive(Clone)]
pub enum Args {
	Concurrent(ConcurrentArgs),
}

impl Args {
	pub fn interval(&self) -> u64 {
		match self {
			Args::Concurrent(a) => a.interval,
		}
	}
}

pub const DEFAULT_SCALE_DOWN_MS: u64 = 30_000;

pub struct EnvConfig {
	pub run_for_ms: u64,
	pub scale_down_ms: u64,
	pub rivet_pool: String,
	pub endpoint: String,
}

impl EnvConfig {
	pub fn from_env() -> Self {
		let run_for_ms = env::var("RUN_FOR_MS")
			.ok()
			.and_then(|v| v.parse().ok())
			.unwrap_or(0);
		let scale_down_ms = env::var("SCALE_DOWN_MS")
			.ok()
			.and_then(|v| v.parse().ok())
			.unwrap_or(DEFAULT_SCALE_DOWN_MS);
		let rivet_pool = env::var("RIVET_POOL").unwrap_or_else(|_| "k8s".to_string());
		let endpoint = match env::var("RIVET_ENDPOINT") {
			Ok(v) if !v.is_empty() => v,
			_ => {
				eprintln!("RIVET_ENDPOINT is required (proto://<ns>:<token>@host)");
				process::exit(1);
			}
		};
		Self { run_for_ms, scale_down_ms, rivet_pool, endpoint }
	}
}

pub fn parse_cli() -> Args {
	let cli = Cli::parse();
	match cli.command {
		Cmd::Concurrent(c) => Args::Concurrent(build_concurrent(ConcurrentMode::Concurrent, c)),
		Cmd::AgentConcurrent(c) => {
			Args::Concurrent(build_concurrent(ConcurrentMode::AgentConcurrent, c))
		}
		Cmd::AgentConcurrent2(c) => {
			Args::Concurrent(build_concurrent(ConcurrentMode::AgentConcurrent2, c))
		}
	}
}

fn build_concurrent(mode: ConcurrentMode, cli: ConcurrentCli) -> ConcurrentArgs {
	let default_message_interval = match mode {
		ConcurrentMode::AgentConcurrent => DEFAULT_AGENT_MESSAGE_INTERVAL_MS,
		ConcurrentMode::AgentConcurrent2 => DEFAULT_MESSAGE_INTERVAL_MS,
		ConcurrentMode::Concurrent => DEFAULT_MESSAGE_INTERVAL_MS,
	};
	ConcurrentArgs {
		mode,
		worker_mode: cli.worker_mode,
		interval: cli.interval,
		concurrency: cli.concurrency,
		message_interval: cli.message_interval_ms.unwrap_or(default_message_interval),
		show_messages: cli.show_messages,
		skip_ready_wait: !cli.wait_ready,
		tokens_per_second: cli.tokens_per_second,
		duration_ms: cli.duration_ms,
		sleep_ms: cli.sleep_ms,
		timeout_ms: cli.timeout_ms,
		stagger_handle_ms: cli.stagger_handle_ms,
		query_multiplier: cli.query_multiplier.max(1),
	}
}
