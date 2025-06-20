use std::{
	path::Path,
	result::Result::{Err, Ok},
	time::Duration,
};

use anyhow::*;
use ctx::Ctx;
use futures_util::StreamExt;
use pegboard::system_info::SystemInfo;
use pegboard_config::Config;
use rand::seq::{IteratorRandom, SliceRandom};
use service_discovery::ServiceDiscovery;
use sqlx::sqlite::SqlitePool;
use tokio::{
	fs,
	runtime::{Builder, Runtime},
};
use tracing_subscriber::prelude::*;
use url::Url;

mod actor;
mod ctx;
mod event_sender;
mod image_download_handler;
mod metrics;
mod pull_addr_handler;
mod runner;
mod system_info;
mod utils;

const PROTOCOL_VERSION: u16 = 2;

#[derive(Clone)]
struct Init {
	config: Config,
	system: SystemInfo,
	pool: SqlitePool,
}

fn main() -> Result<()> {
	init_tracing();

	let init = { Runtime::new()?.block_on(init())? };
	let mut first = true;

	// Retry loop
	loop {
		let rt = Builder::new_multi_thread().enable_all().build()?;

		match rt.block_on(run(init.clone(), first)) {
			Ok(_) => return Ok(()),
			Err(err) => {
				// Only restart if its a `RuntimeError`
				let runtime_err = err.downcast::<ctx::RuntimeError>()?;

				tracing::error!("runtime error: {runtime_err}");

				// Destroy entire runtime to kill any background threads
				rt.shutdown_background();
			}
		}

		first = false;

		std::thread::sleep(Duration::from_secs(2));
	}
}

async fn init() -> Result<Init> {
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

	// Determine config format and parse
	let config = match config_path.extension().and_then(|s| s.to_str()) {
		Some("json") => serde_json::from_str::<Config>(&config_data).with_context(|| {
			format!(
				"Failed to parse JSON config file at {}",
				config_path.display()
			)
		})?,
		Some("json5") | Some("jsonc") => {
			json5::from_str::<Config>(&config_data).with_context(|| {
				format!(
					"Failed to parse JSON5 config file at {}",
					config_path.display()
				)
			})?
		}
		Some("yaml") | Some("yml") => {
			serde_yaml::from_str::<Config>(&config_data).with_context(|| {
				format!(
					"Failed to parse YAML config file at {}",
					config_path.display()
				)
			})?
		}
		_ => bail!(
			"unrecognized config file extension at {}",
			config_path.display()
		),
	};

	if config.client.logs.redirect_logs() {
		rivet_logs::Logs::new(
			config.client.data_dir().join("logs"),
			config.client.logs.retention(),
		)
		.start()
		.await?;
	}

	// SAFETY: No other task has spawned yet.
	// Set client_id env var so it can be read by the prometheus registry
	unsafe {
		std::env::set_var("CLIENT_ID", config.client.cluster.client_id.to_string());
	}

	// Read system metrics
	let system = crate::system_info::fetch().await?;

	// Init project directories
	utils::init_dir(&config).await?;

	// Init sqlite db
	let pool = utils::init_sqlite_db(&config).await?;

	Ok(Init {
		config,
		system,
		pool,
	})
}

async fn run(init: Init, first: bool) -> Result<()> {
	// We have to redirect logs here as well because the entire tokio runtime gets destroyed after a runtime
	// error
	if !first && init.config.client.logs.redirect_logs() {
		rivet_logs::Logs::new(
			init.config.client.data_dir().join("logs"),
			init.config.client.logs.retention(),
		)
		.start()
		.await?;
	}

	// Start metrics server
	let metrics_thread = tokio::spawn(metrics::run_standalone(init.config.client.metrics.port()));

	let url = build_ws_url(&init.config).await?;
	tracing::info!("connecting to pegboard ws: {}", &url);

	// Connect to WS
	let (ws_stream, _) = tokio_tungstenite::connect_async(url.to_string())
		.await
		.map_err(|source| ctx::RuntimeError::ConnectionFailed {
			url: url.clone(),
			source,
		})?;
	let (tx, rx) = ws_stream.split();

	tracing::info!("connected to pegboard ws");

	let ctx = Ctx::new(init.config, init.system, init.pool, tx);

	tokio::try_join!(
		async { metrics_thread.await?.map_err(Into::into) },
		ctx.run(rx),
	)?;

	Ok(())
}

async fn build_ws_url(config: &Config) -> Result<Url> {
	let endpoint = match &config.client.cluster.ws_addresses {
		pegboard_config::Addresses::Dynamic { fetch_endpoint } => {
			let sd = ServiceDiscovery::new(fetch_endpoint.clone());

			let servers = sd.fetch().await?;

			// Choose random server
			let mut rng = rand::thread_rng();
			let lan_ip = servers
				.into_iter()
				.flat_map(|s| s.lan_ip)
				.choose(&mut rng)
				.context("No worker servers available from service discovery")?;

			format!("ws://{lan_ip}:8082")
		}
		pegboard_config::Addresses::Static(addresses) => {
			// Choose random server
			let mut rng = rand::thread_rng();
			let addr = addresses
				.choose(&mut rng)
				.context("No ws addresses provided")?;

			format!("ws://{addr}")
		}
	};

	// Build WS connection URL
	let mut url = Url::parse(&endpoint)?;
	url.set_path(&format!("/v{PROTOCOL_VERSION}"));
	url.query_pairs_mut()
		.append_pair("client_id", &config.client.cluster.client_id.to_string())
		.append_pair("flavor", &config.client.runner.flavor.to_string());

	Ok(url)
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
