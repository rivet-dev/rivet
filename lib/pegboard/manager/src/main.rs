use std::{
	net::{IpAddr, Ipv4Addr},
	path::Path,
};

use anyhow::*;
use futures_util::StreamExt;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tracing_subscriber::prelude::*;
use url::Url;
use uuid::Uuid;

mod container;
mod ctx;
mod metrics;
mod utils;

use ctx::Ctx;

const PROTOCOL_VERSION: u16 = 1;

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing();

	// Print version
	if std::env::args().any(|a| a == "-v" || a == "--version") {
		println!(env!("CARGO_PKG_VERSION"));
		return Ok(());
	}

	// Start metrics server
	tokio::spawn(metrics::run_standalone());

	let client_id = Uuid::parse_str(&utils::var("CLIENT_ID")?)?;
	let network_ip = get_network_ip(&utils::var("NETWORK_INTERFACE")?)?;

	let system = System::new_with_specifics(
		RefreshKind::new()
			.with_cpu(CpuRefreshKind::new().with_frequency())
			.with_memory(MemoryRefreshKind::new().with_ram()),
	);

	let working_path = Path::new("/etc/pegboard");
	utils::init_working_dir(&working_path).await?;

	// Init sqlite db
	let sqlite_db_url = format!(
		"sqlite://{}",
		working_path.join("db").join("database.db").display()
	);
	utils::init_sqlite_db(&sqlite_db_url).await?;

	// Connect to sqlite db
	let pool = utils::build_sqlite_pool(&sqlite_db_url).await?;
	utils::init_sqlite_schema(&pool).await?;

	// Build connection URL
	let mut url = Url::parse("ws://127.0.0.1:5030")?;
	url.set_path(&format!("/v{PROTOCOL_VERSION}"));
	url.query_pairs_mut()
		.append_pair("client_id", &client_id.to_string());

	tracing::info!("connecting to ws: {url}");

	// Connect to WS
	let (ws_stream, _) = tokio_tungstenite::connect_async(url.to_string())
		.await
		.context("failed to connect to websocket")?;
	let (tx, rx) = ws_stream.split();

	tracing::info!("connected");

	let ctx = Ctx::new(working_path.to_path_buf(), network_ip, system, pool, tx);

	ctx.start(rx).await
}

fn get_network_ip(network_interface_name: &str) -> Result<Ipv4Addr> {
	let network_interface = pnet_datalink::interfaces()
		.into_iter()
		.find(|iface| iface.name == network_interface_name)
		.context(format!(
			"network interface not found: {network_interface_name}"
		))?;
	let network_ip = network_interface
		.ips
		.iter()
		.find_map(|net| {
			if let IpAddr::V4(ip) = net.ip() {
				Some(ip)
			} else {
				None
			}
		})
		.context("no ipv4 network on interface")?;

	Ok(network_ip)
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
