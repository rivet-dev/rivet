use chirp_workflow::prelude::*;
use clap::Parser;
use global_error::*;
use rivet_guard_core::proxy_service::{
    MaxInFlightConfig, MiddlewareConfig, MiddlewareResponse, RateLimitConfig,
    RetryConfig, RoutingResponse, TimeoutConfig,
};
use std::{path::PathBuf, sync::Arc};
use tokio::signal;

mod routing;

#[derive(Parser)]
#[command(name = "Rivet", version, about)]
struct Cli {
    #[clap(long, global = true)]
    config: Vec<PathBuf>,
}

fn main() -> GlobalResult<()> {
    rivet_runtime::run(async { main_inner().await })?;
    Ok(())
}

async fn main_inner() -> GlobalResult<()> {
    let cli = Cli::parse();

    // Load config
    let config = rivet_config::Config::load(&cli.config).await?;
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

    // Start the server
    tracing::info!("starting proxy server");
    tokio::select! {
        result = rivet_guard_core::run_server(config, routing_fn, middleware_fn) => {
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
    Arc::new(move |actor_id: &uuid::Uuid| {
        let ctx = ctx.clone();

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
                    max_attempts: 3,       // 3 retry attempts
                    initial_interval: 100, // 100ms initial interval
                },
                timeout: TimeoutConfig {
                    request_timeout: 30, // 30 seconds for requests
                },
            }))
        })
    })
}