// CLI + env parsing. Uses clap derive with short flags for every option.

use std::env;
use std::process;

use clap::{Parser, Subcommand};

pub const DEFAULT_CONCURRENCY: u32 = 1_000;
pub const DEFAULT_CONCURRENT_INTERVAL_MS: u64 = 300;
pub const DEFAULT_MESSAGE_INTERVAL_MS: u64 = 1_000;
pub const DEFAULT_AGENT_MESSAGE_INTERVAL_MS: u64 = 30_000;
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
		rtt              spawn fresh counter actors and measure action RTTs\n  \
		concurrent       ramp persistent raw WebSocket tunnel-stress actors\n  \
		agent-concurrent ramp persistent SQLite-backed agent actors\n\nEnv:\n  \
		RIVET_ENDPOINT  required, proto://<ns>:<token>@host\n  \
		RIVET_POOL      runner pool name (default k8s)\n  \
		BATCHES         total workers in rtt mode (default infinite)\n  \
		SERIAL          1/true to serialize rtt workers\n  \
		RUN_FOR_MS      stop concurrent modes after this many ms"
)]
struct Cli {
	#[command(subcommand)]
	command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
	/// Spawn fresh counter actors and measure action RTTs.
	Rtt(RttCli),
	/// Ramp persistent raw WebSocket tunnel-stress actors.
	Concurrent(ConcurrentCli),
	/// Ramp persistent SQLite-backed agent actors.
	#[command(name = "agent-concurrent")]
	AgentConcurrent(ConcurrentCli),
}

#[derive(clap::Args, Clone)]
struct RttCli {
	/// Gap in ms between worker starts (required).
	#[arg(short = 'i', long)]
	interval: u64,
	/// Wait for actor ready before measuring (default: skip).
	#[arg(short = 'w', long)]
	wait_ready: bool,
}

#[derive(clap::Args, Clone)]
struct ConcurrentCli {
	/// Ramp-up gap in ms between connections.
	#[arg(short = 'i', long, default_value_t = DEFAULT_CONCURRENT_INTERVAL_MS)]
	interval: u64,
	/// Number of persistent connections.
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
}

#[derive(Clone)]
pub struct RttArgs {
	pub interval: u64,
	pub skip_ready_wait: bool,
}

#[derive(Clone)]
pub struct ConcurrentArgs {
	pub mode: ConcurrentMode,
	pub interval: u64,
	pub concurrency: u32,
	pub message_interval: u64,
	pub show_messages: bool,
	pub skip_ready_wait: bool,
	pub tokens_per_second: f64,
	pub duration_ms: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConcurrentMode {
	Concurrent,
	AgentConcurrent,
}

#[derive(Clone)]
pub enum Args {
	Rtt(RttArgs),
	Concurrent(ConcurrentArgs),
}

impl Args {
	pub fn interval(&self) -> u64 {
		match self {
			Args::Rtt(a) => a.interval,
			Args::Concurrent(a) => a.interval,
		}
	}

	pub fn skip_ready_wait(&self) -> bool {
		match self {
			Args::Rtt(a) => a.skip_ready_wait,
			Args::Concurrent(a) => a.skip_ready_wait,
		}
	}
}

pub struct EnvConfig {
	pub batches: u64,
	pub serial: bool,
	pub run_for_ms: u64,
	pub rivet_pool: String,
	pub endpoint: String,
}

impl EnvConfig {
	pub fn from_env() -> Self {
		let batches = env::var("BATCHES").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
		let serial = matches!(env::var("SERIAL").as_deref(), Ok("1") | Ok("true"));
		let run_for_ms = env::var("RUN_FOR_MS")
			.ok()
			.and_then(|v| v.parse().ok())
			.unwrap_or(0);
		let rivet_pool = env::var("RIVET_POOL").unwrap_or_else(|_| "k8s".to_string());
		let endpoint = match env::var("RIVET_ENDPOINT") {
			Ok(v) if !v.is_empty() => v,
			_ => {
				eprintln!("RIVET_ENDPOINT is required (proto://<ns>:<token>@host)");
				process::exit(1);
			}
		};
		Self { batches, serial, run_for_ms, rivet_pool, endpoint }
	}
}

pub fn parse_cli() -> Args {
	let cli = Cli::parse();
	match cli.command {
		Cmd::Rtt(rtt) => Args::Rtt(RttArgs {
			interval: rtt.interval,
			skip_ready_wait: !rtt.wait_ready,
		}),
		Cmd::Concurrent(c) => Args::Concurrent(build_concurrent(ConcurrentMode::Concurrent, c)),
		Cmd::AgentConcurrent(c) => {
			Args::Concurrent(build_concurrent(ConcurrentMode::AgentConcurrent, c))
		}
	}
}

fn build_concurrent(mode: ConcurrentMode, cli: ConcurrentCli) -> ConcurrentArgs {
	let default_message_interval = match mode {
		ConcurrentMode::AgentConcurrent => DEFAULT_AGENT_MESSAGE_INTERVAL_MS,
		ConcurrentMode::Concurrent => DEFAULT_MESSAGE_INTERVAL_MS,
	};
	ConcurrentArgs {
		mode,
		interval: cli.interval,
		concurrency: cli.concurrency,
		message_interval: cli.message_interval_ms.unwrap_or(default_message_interval),
		show_messages: cli.show_messages,
		skip_ready_wait: !cli.wait_ready,
		tokens_per_second: cli.tokens_per_second,
		duration_ms: cli.duration_ms,
	}
}
