use std::{path::PathBuf, sync::Arc, time::Duration};

use chirp_workflow::prelude::*;
use clap::Parser;
use global_error::GlobalResult;
use rivet_guard_core::proxy_service::{
	MaxInFlightConfig, MiddlewareConfig, MiddlewareResponse, RateLimitConfig, RetryConfig,
	TimeoutConfig,
};
use tokio::signal;

mod routing;
mod tls;

// 7 day logs retention
const LOGS_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Parser)]
#[command(name = "Rivet", version, about)]
struct Cli {
	#[clap(long, global = true)]
	config: Vec<PathBuf>,
}

fn main() -> GlobalResult<()> {
	// Initialize with a default CryptoProvider for rustls
	let provider = rustls::crypto::ring::default_provider();
	provider
		.install_default()
		.expect("Failed to install crypto provider");

	rivet_runtime::run(main_inner()).transpose()?;
	Ok(())
}

async fn main_inner() -> GlobalResult<()> {
	let cli = Cli::parse();

	// Load config
	let config = rivet_config::Config::load(&cli.config).await?;

	// Redirect logs if enabled
	if let Some(logs_dir) = config
		.server()
		.ok()
		.and_then(|x| x.rivet.edge.as_ref())
		.and_then(|x| x.redirect_logs_dir.as_ref())
	{
		tokio::fs::create_dir_all(logs_dir).await?;
		unwrap!(
			rivet_logs::Logs::new(logs_dir.clone(), LOGS_RETENTION)
				.start()
				.await
		);
	}

	let pools = rivet_pools::Pools::new(config.clone()).await?;

	// Create context
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("rivet-guard");
	let cache = rivet_cache::CacheInner::from_env(&config, pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseFdbSqliteNats::from_pools(pools.clone())?,
		config.clone(),
		rivet_connection::Connection::new(client, pools, cache),
		"rivet-guard",
	)
	.await?;

	// Create a routing function that has access to config and pool
	let routing_fn = routing::create_routing_function(ctx.clone());

	// Create a middleware function
	let middleware_fn = create_middleware_function(ctx.clone());

	// Create certificate resolver for TLS
	let cert_resolver = tls::create_cert_resolver(&ctx).await?;

	// Print TLS configuration status
	if let Some(_) = &cert_resolver {
		tracing::info!("TLS certificate resolver configured");
	} else {
		tracing::info!("No TLS configuration found, HTTPS will not be enabled");
	}

	// Start the server
	tracing::info!("starting proxy server");
	tokio::select! {
		result = rivet_guard_core::run_server(config, routing_fn, middleware_fn, cert_resolver) => {
			if let Err(e) = result {
				tracing::error!("Server error: {}", e);
			}
		}
		_ = signal::ctrl_c() => {
			tracing::info!("received Ctrl+C, shutting down");
		}
	}

	Ok(())
}

/// Creates a middleware function that can use config and pools
fn create_middleware_function(
	ctx: StandaloneCtx,
) -> Arc<
	dyn for<'a> Fn(
			&'a uuid::Uuid,
		) -> futures::future::BoxFuture<'a, GlobalResult<MiddlewareResponse>>
		+ Send
		+ Sync,
> {
	Arc::new(move |_actor_id: &uuid::Uuid| {
		let _ctx = ctx.clone();

		Box::pin(async move {
			// In a real implementation, you would look up actor-specific middleware settings
			// For now, we'll just return a standard configuration

			// Create middleware config based on the actor ID
			// This could be fetched from a database in a real implementation
			Ok(MiddlewareResponse::Ok(MiddlewareConfig {
				rate_limit: RateLimitConfig {
					requests: 100, // 100 requests
					period: 60,    // per 60 seconds
				},
				max_in_flight: MaxInFlightConfig {
					amount: 20, // 20 concurrent requests
				},
				retry: RetryConfig {
					max_attempts: 7,
					initial_interval: 150,
				},
				timeout: TimeoutConfig {
					request_timeout: 30, // 30 seconds for requests
				},
			}))
		})
	})
}
