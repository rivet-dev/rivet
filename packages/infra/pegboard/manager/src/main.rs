use std::path::Path;

use anyhow::*;
use futures_util::StreamExt;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tracing_subscriber::prelude::*;
use url::Url;

mod actor;
mod config;
mod ctx;
mod metrics;
mod runner;
mod utils;

use config::Config;
use ctx::Ctx;

const PROTOCOL_VERSION: u16 = 1;

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing();

	// Read args
	let mut config_flag = false;
	let mut args = std::env::args();
	let config_path = loop {
		let Some(arg) = args.next() else {
			bail!("missing `--config` argument");
		};

		if config_flag {
			break Path::new(&arg).to_path_buf();
		} else if arg == "-c" || arg == "--config" {
			config_flag = true;
		} else if arg == "-v" || arg == "--version" {
			// Print version
			println!(env!("CARGO_PKG_VERSION"));
			return Ok(());
		} else {
			bail!("unexpected argument {arg}");
		}
	};

	// Read config
	let config_data = std::fs::read_to_string(&config_path)
		.with_context(|| format!("Failed to read config file at {}", config_path.display()))?;
	let config = serde_json::from_str::<Config>(&config_data)
		.with_context(|| format!("Failed to parse config file at {}", config_path.display()))?;

	// SAFETY: No other task has spawned yet.
	// set client_id env var so it can be read by the prometheus registry
	unsafe {
		std::env::set_var("CLIENT_ID", config.client_id.to_string());
	}

	// Start metrics server
	tokio::spawn(metrics::run_standalone());

	// Read system metrics
	let system = System::new_with_specifics(
		RefreshKind::new()
			.with_cpu(CpuRefreshKind::new().with_frequency())
			.with_memory(MemoryRefreshKind::new().with_ram()),
	);

	// Init project directories
	utils::init_dir(&config).await?;

	// Init sqlite db
	let sqlite_db_url = format!(
		"sqlite://{}",
		config.working_path.join("db").join("database.db").display()
	);
	utils::init_sqlite_db(&sqlite_db_url).await?;

	// Connect to sqlite db
	let pool = utils::build_sqlite_pool(&sqlite_db_url).await?;
	utils::init_sqlite_schema(&pool).await?;

	// Build WS connection URL
	let mut url = Url::parse("ws://127.0.0.1:5030")?;
	url.set_path(&format!("/v{PROTOCOL_VERSION}"));
	url.query_pairs_mut()
		.append_pair("client_id", &config.client_id.to_string())
		.append_pair("datacenter_id", &config.datacenter_id.to_string())
		.append_pair("flavor", &config.flavor.to_string());

	tracing::info!("connecting to ws: {url}");

	// Connect to WS
	let (ws_stream, _) = tokio_tungstenite::connect_async(url.to_string())
		.await
		.context("failed to connect to websocket")?;
	let (tx, rx) = ws_stream.split();

	tracing::info!("connected");

	let ctx = Ctx::new(config, system, pool, tx);

	ctx.start(rx).await
}

fn init_tracing() {
	tracing_subscriber::registry()
		.with(
			tracing_logfmt::builder()
				.layer()
				.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
		)
		.init();
}
