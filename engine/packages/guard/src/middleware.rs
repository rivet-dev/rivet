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
	Arc::new(move |actor_id: &Option<Id>, _headers: &hyper::HeaderMap| {
		let ctx = ctx.clone();
		let is_actor_traffic = actor_id.is_some();

		Box::pin(async move {
			let guard = ctx.config().guard();
			let pegboard = ctx.config().pegboard();

			let config = if is_actor_traffic {
				// Actor traffic uses gateway_* settings
				MiddlewareConfig {
					rate_limit: RateLimitConfig {
						requests: pegboard.gateway_rate_limit_requests(),
						period: pegboard.gateway_rate_limit_period_secs(),
					},
					max_in_flight: MaxInFlightConfig {
						amount: pegboard.gateway_max_in_flight(),
					},
					retry: RetryConfig {
						max_attempts: pegboard.gateway_retry_max_attempts(),
						initial_interval: pegboard.gateway_retry_initial_interval_ms(),
					},
					timeout: TimeoutConfig {
						request_timeout: pegboard.gateway_actor_request_timeout_secs(),
					},
					max_incoming_ws_message_size: guard.websocket_max_message_size(),
					max_outgoing_ws_message_size: guard.websocket_max_outgoing_message_size(),
					max_http_request_body_size: pegboard.gateway_http_max_request_body_size(),
				}
			} else {
				// API traffic uses api_* settings
				MiddlewareConfig {
					rate_limit: RateLimitConfig {
						requests: pegboard.api_rate_limit_requests(),
						period: pegboard.api_rate_limit_period_secs(),
					},
					max_in_flight: MaxInFlightConfig {
						amount: pegboard.api_max_in_flight(),
					},
					retry: RetryConfig {
						max_attempts: pegboard.api_retry_max_attempts(),
						initial_interval: pegboard.api_retry_initial_interval_ms(),
					},
					timeout: TimeoutConfig {
						request_timeout: pegboard.gateway_api_request_timeout_secs(),
					},
					max_incoming_ws_message_size: guard.websocket_max_message_size(),
					max_outgoing_ws_message_size: guard.websocket_max_outgoing_message_size(),
					max_http_request_body_size: pegboard.api_max_http_request_body_size(),
				}
			};

			Ok(MiddlewareResponse::Ok(config))
		})
	})
}
