use std::path::Path;

use anyhow::*;
use futures_util::StreamExt;
use tracing_subscriber::prelude::*;
use url::Url;
use uuid::Uuid;

mod container;
mod ctx;
mod utils;

use ctx::Ctx;

const PROTOCOL_VERSION: u16 = 1;

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing();

	let client_id: Uuid = Uuid::parse_str(
		&std::env::args()
			.skip(1)
			.next()
			.context("`client_id` arg required")?,
	)?;

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

	let ctx = Ctx::new(working_path.to_path_buf(), pool, tx);

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
