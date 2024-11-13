use std::{
	os::fd::AsRawFd,
	path::{Path, PathBuf},
	time::Duration,
};

use anyhow::*;
use futures_util::StreamExt;
use pegboard::system_info::SystemInfo;
use sqlx::sqlite::SqlitePool;
use tokio::{
	fs,
	runtime::{Builder, Runtime},
};
use tracing_subscriber::prelude::*;
use url::Url;

mod actor;
mod system_info;
mod config;
mod ctx;
mod metrics;
mod runner;
mod utils;

use config::Config;
use ctx::Ctx;

const PROTOCOL_VERSION: u16 = 1;

#[derive(Clone)]
struct Init {
	config: Config,
	system: SystemInfo,
	pool: SqlitePool,
	url: Url,
}

fn main() -> Result<()> {
	let init = { Runtime::new()?.block_on(init())? };

	// Retry loop
	loop {
		let runtime = Builder::new_multi_thread().enable_all().build()?;

		use std::result::Result::{Err, Ok};
		match runtime.block_on(run(init.clone())) {
			Ok(_) => return Ok(()),
			Err(err) => {
				// Only restart if its a `RuntimeError`
				let runtime_err = err.downcast::<ctx::RuntimeError>()?;

				tracing::error!("runtime error: {runtime_err}");

				// Destroy entire runtime to kill any background threads
				runtime.shutdown_background();
			}
		}

		std::thread::sleep(Duration::from_secs(2));
	}
}

async fn init() -> Result<Init> {
	init_tracing();

	// Read args
	let mut config_flag = false;
	let mut args = std::env::args();
	// Skip exec
	args.next();

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
			std::process::exit(0);
		} else {
			bail!("unexpected argument {arg}");
		}
	};

	// Read config
	let config_data = fs::read_to_string(&config_path)
		.await
		.with_context(|| format!("Failed to read config file at {}", config_path.display()))?;
	let config = serde_json::from_str::<Config>(&config_data)
		.with_context(|| format!("Failed to parse config file at {}", config_path.display()))?;

	if config.redirect_logs {
		redirect_logs(config.data_dir.join("log")).await?;
	}

	// SAFETY: No other task has spawned yet.
	// Set client_id env var so it can be read by the prometheus registry
	unsafe {
		std::env::set_var("CLIENT_ID", config.client_id.to_string());
	}

	// Read system metrics
	let system = crate::system_info::fetch().await?;

	// Init project directories
	utils::init_dir(&config).await?;

	// Init sqlite db
	let sqlite_db_url = format!(
		"sqlite://{}",
		config.data_dir.join("db").join("database.db").display()
	);
	utils::init_sqlite_db(&sqlite_db_url).await?;

	// Connect to sqlite db
	let pool = utils::build_sqlite_pool(&sqlite_db_url).await?;
	utils::init_sqlite_schema(&pool).await?;

	// Build WS connection URL
	let mut url = config.pegboard_ws_endpoint.clone();
	url.set_path(&format!("/v{PROTOCOL_VERSION}"));
	url.query_pairs_mut()
		.append_pair("client_id", &config.client_id.to_string())
		.append_pair("datacenter_id", &config.datacenter_id.to_string())
		.append_pair("flavor", &config.flavor.to_string());

	Ok(Init {
		config,
		system,
		pool,
		url,
	})
}

async fn run(init: Init) -> Result<()> {
	// Start metrics server
	let metrics_thread = tokio::spawn(metrics::run_standalone());

	tracing::info!("connecting to ws: {}", &init.url);

	// Connect to WS
	let (ws_stream, _) = tokio_tungstenite::connect_async(init.url.to_string())
		.await
		.map_err(|source| ctx::RuntimeError::ConnectionFailed {
			url: init.url.clone(),
			source,
		})?;
	let (tx, rx) = ws_stream.split();

	tracing::info!("connected");

	let ctx = Ctx::new(init.config, init.system, init.pool, tx);

	tokio::try_join!(
		async { metrics_thread.await?.map_err(Into::into) },
		ctx.run(rx),
	)?;

	Ok(())
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

async fn redirect_logs(log_file_path: PathBuf) -> Result<()> {
	tracing::info!("Redirecting all logs to {}", log_file_path.display());
	let log_file = fs::OpenOptions::new()
		.write(true)
		.create(true)
		.append(true)
		.open(log_file_path)
		.await?;
	let log_fd = log_file.as_raw_fd();

	nix::unistd::dup2(log_fd, nix::libc::STDOUT_FILENO)?;
	nix::unistd::dup2(log_fd, nix::libc::STDERR_FILENO)?;

	Ok(())
}
