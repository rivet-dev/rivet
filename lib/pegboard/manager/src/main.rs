use anyhow::*;
use futures_util::StreamExt;
use tracing_subscriber::prelude::*;
use url::Url;
use uuid::Uuid;

mod client;
mod container;
mod ctx;
mod utils;

use client::Client;

#[tokio::main]
async fn main() -> Result<()> {
	init_tracing();

	let client_id: Uuid = todo!();

	// Build connection URL
	let mut url = Url::parse("ws://127.0.0.1:5030")?;
	url.set_path("/v1");
	url.query_pairs_mut()
		.append_pair("client_id", &client_id.to_string());

	// Connect to WS
	let (ws_stream, _) = tokio_tungstenite::connect_async(url.to_string()).await?;
	let (mut tx, mut rx) = ws_stream.split();

	let client = Client::new(tx);

	client.start(rx).await
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
