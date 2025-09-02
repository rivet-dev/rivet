use std::net::SocketAddr;

use anyhow::*;

pub mod actors;
pub mod namespaces;
pub mod router;
pub mod runners;

pub use router::router as create_router;

pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let host = config.api_peer().host();
	let port = config.api_peer().port();
	let addr = SocketAddr::from((host, port));

	let router = router::router("api-peer", config, pools).await?;

	let listener = tokio::net::TcpListener::bind(addr).await?;
	tracing::info!(?host, ?port, "api-peer server listening");

	axum::serve(listener, router).await?;

	Ok(())
}
