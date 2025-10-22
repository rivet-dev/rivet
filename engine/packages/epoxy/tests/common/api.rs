use anyhow::*;
use rivet_api_builder::create_router;
use std::net::SocketAddr;

pub async fn setup_api_server(
	config: rivet_config::Config,
	pools: rivet_pools::Pools,
	port: u16,
) -> Result<tokio::task::JoinHandle<()>> {
	let addr = SocketAddr::from(([127, 0, 0, 1], port));

	let router = create_router("api-peer-test", config, pools, |router| {
		epoxy::http_routes::mount_routes(router)
	})
	.await?;

	let listener = tokio::net::TcpListener::bind(addr).await?;
	tracing::info!(?port, "test api-peer server listening");

	let handle = tokio::spawn(async move {
		axum::serve(listener, router).await.unwrap();
	});

	Ok(handle)
}
