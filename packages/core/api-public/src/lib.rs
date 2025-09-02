use std::net::SocketAddr;

use anyhow::*;

pub mod actors;
pub mod datacenters;
mod errors;
pub mod namespaces;
pub mod router;
pub mod runners;
pub mod ui;

pub use router::router as create_router;

pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let host = config.api_public().host();
	let port = config.api_public().port();
	let addr = SocketAddr::from((host, port));

	let router = router::router("api-public", config, pools).await?;

	let listener = tokio::net::TcpListener::bind(addr).await?;
	tracing::info!(?host, ?port, "api-public server listening");

	axum::serve(listener, router).await?;

	Ok(())
}
