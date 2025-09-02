use std::sync::Arc;

use anyhow::*;
use gas::prelude::*;
use rivet_guard_core::{
	MiddlewareFn,
	proxy_service::{
		MaxInFlightConfig, MiddlewareConfig, MiddlewareResponse, RateLimitConfig, RetryConfig,
		TimeoutConfig,
	},
};

/// Creates a middleware function that can use config and pools
pub fn create_middleware_function(ctx: StandaloneCtx) -> MiddlewareFn {
	Arc::new(move |_actor_id: &Id, _headers: &hyper::HeaderMap| {
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
