use anyhow::*;
use gas::prelude::*;

pub mod cache;
pub mod errors;
pub mod middleware;
pub mod routing;
pub mod tls;

#[tracing::instrument(skip_all)]
pub async fn start(config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseKv::from_pools(pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		"guard",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)?;

	// Initialize with a default CryptoProvider for rustls
	let provider = rustls::crypto::ring::default_provider();
	if provider.install_default().is_err() {
		tracing::warn!("crypto provider already installed in this process");
	}

	// Create handlers
	let routing_fn = routing::create_routing_function(ctx.clone());
	let cache_key_fn = cache::create_cache_key_function(ctx.clone());
	let middleware_fn = middleware::create_middleware_function(ctx.clone());
	let cert_resolver = tls::create_cert_resolver(&ctx).await?;

	if let Some(_) = &cert_resolver {
		tracing::info!("TLS certificate resolver configured");
	} else {
		tracing::info!("No TLS configuration found, HTTPS will not be enabled");
	}

	// Start the server
	tracing::info!("starting proxy server");
	let clickhouse_inserter = ctx.clickhouse_inserter().ok();
	rivet_guard_core::run_server(
		config,
		routing_fn,
		cache_key_fn,
		middleware_fn,
		cert_resolver,
		clickhouse_inserter,
	)
	.await
}
