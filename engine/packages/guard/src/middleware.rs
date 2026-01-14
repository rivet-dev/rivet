use std::sync::Arc;

use rivet_guard_core::{
	MiddlewareFn,
	proxy_service::{
		MaxInFlightConfig, MiddlewareConfig, MiddlewareResponse, RateLimitConfig, RetryConfig,
		TimeoutConfig,
	},
};

/// Creates a middleware function that can use config and pools
pub fn create_middleware_function() -> MiddlewareFn {
	Arc::new(move |_headers: &hyper::HeaderMap| {
		Box::pin(async move {
			// In a real implementation, you would look up actor-specific middleware settings
			// For now, we'll just return a standard configuration

			// Create middleware config based on the actor ID
			// This could be fetched from a database in a real implementation
			Ok(MiddlewareResponse::Ok(MiddlewareConfig {
				rate_limit: RateLimitConfig {
					requests: 10000, // 10000 requests
					period: 60,      // per 60 seconds
				},
				max_in_flight: MaxInFlightConfig {
					amount: 2000, // 2000 concurrent requests
				},
				retry: RetryConfig {
					max_attempts: 7,       // 7 retry attempts
					initial_interval: 150, // 150ms initial interval
				},
				timeout: TimeoutConfig {
					request_timeout: 30, // 30 seconds for requests
				},
			}))
		})
	})
}
